use std::net::SocketAddr;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use nexus_domain::BedrockManagementProfile;
use nexus_domain::CoreId;
use nexus_domain::FileContent;
use nexus_domain::FileEntry;
use nexus_domain::FilePage;
use nexus_domain::Instance;
use nexus_domain::InstanceCreate;
use nexus_domain::InstanceId;
use nexus_domain::InstanceLogPage;
use nexus_domain::InstanceMetricSample;
use nexus_domain::InstancePage;
use nexus_domain::InstanceState;
use nexus_domain::InstanceUpdate;
use nexus_domain::ManagedRuntime;
use nexus_domain::PRODUCT_VERSION;
use nexus_domain::ProvisionPlan;
use nexus_domain::ProxySubserver;
use nexus_domain::RequestId;
use nexus_domain::RuntimeInstallManifest;
use nexus_domain::TaskId;
use nexus_protocol::CURRENT_PROTOCOL_VERSION;
use nexus_protocol::NoiseTransport;
use nexus_protocol::PresharedKey;
use nexus_protocol::ProtocolVersion;
use nexus_protocol::TlsClientStream;
use nexus_protocol::WireMessage;
use nexus_protocol::connect_tls;
use serde_json::Value;
use serde_json::from_value;
use serde_json::json;
use serde_json::to_value;
use tokio::net::TcpStream;

use crate::CoreConnectionError;
use crate::CoreEndpoint;

const PANEL_CAPABILITIES: [&str; 8] = [
    "events",
    "files",
    "instances",
    "metrics",
    "proxy-subservers",
    "provision",
    "runtimes",
    "settings",
];

pub struct CoreConnection {
    capabilities: Vec<String>,
    core_id: CoreId,
    heartbeat_seconds: u64,
    protocol: ProtocolVersion,
    tls_certificate_sha256: String,
    transport: NoiseTransport<TlsClientStream<TcpStream>>,
}

impl CoreConnection {
    pub async fn connect(
        address: SocketAddr,
        pre_shared_key: &PresharedKey,
        panel_id: &str,
        panel_name: &str,
    ) -> Result<Self, CoreConnectionError> {
        Self::connect_endpoint(
            &CoreEndpoint::from_socket_address(address),
            pre_shared_key,
            panel_id,
            panel_name,
        )
        .await
    }

    pub async fn connect_address(
        address: &str,
        skip_certificate_verification: bool,
        pre_shared_key: &PresharedKey,
        panel_id: &str,
        panel_name: &str,
    ) -> Result<Self, CoreConnectionError> {
        let endpoint = CoreEndpoint::parse(address, skip_certificate_verification)?;

        Self::connect_endpoint(&endpoint, pre_shared_key, panel_id, panel_name).await
    }

    pub async fn connect_endpoint(
        endpoint: &CoreEndpoint,
        pre_shared_key: &PresharedKey,
        panel_id: &str,
        panel_name: &str,
    ) -> Result<Self, CoreConnectionError> {
        let address = format!("{}:{}", endpoint.host(), endpoint.port());
        let stream = TcpStream::connect((endpoint.host(), endpoint.port()))
            .await
            .map_err(|source| CoreConnectionError::Connect {
                address: address.clone(),
                source,
            })?;
        let (stream, tls_certificate_sha256) = connect_tls(
            stream,
            endpoint.host().to_owned(),
            endpoint.verify_certificate(),
        )
        .await?;
        let mut transport = NoiseTransport::connect(stream, pre_shared_key).await?;
        let request_id = RequestId::new();
        let hello = WireMessage::Request {
            request_id,
            method: "session.hello".to_owned(),
            params: json!({
                "protocol": CURRENT_PROTOCOL_VERSION,
                "panelId": panel_id,
                "panelName": panel_name,
                "clientVersion": PRODUCT_VERSION,
                "capabilities": PANEL_CAPABILITIES,
            }),
            deadline: None,
            idempotency_key: None,
        };

        transport.write_message(&hello).await?;
        let welcome = response_result(transport.read_message().await?, request_id)?;
        let protocol = response_field(&welcome, "protocol")?;
        let protocol = from_value(protocol)
            .map_err(|_| CoreConnectionError::InvalidResponse { field: "protocol" })?;
        let protocol = CURRENT_PROTOCOL_VERSION.negotiate(protocol)?;
        let core_id = response_field(&welcome, "coreId")?;
        let core_id = from_value(core_id)
            .map_err(|_| CoreConnectionError::InvalidResponse { field: "coreId" })?;
        let capabilities = response_field(&welcome, "capabilities")?;
        let capabilities =
            from_value(capabilities).map_err(|_| CoreConnectionError::InvalidResponse {
                field: "capabilities",
            })?;
        let heartbeat_seconds = response_field(&welcome, "heartbeatSeconds")?
            .as_u64()
            .ok_or(CoreConnectionError::InvalidResponse {
                field: "heartbeatSeconds",
            })?;
        let welcome_certificate_sha256 = response_field(&welcome, "tlsCertificateSha256")?;
        let welcome_certificate_sha256 =
            welcome_certificate_sha256
                .as_str()
                .ok_or(CoreConnectionError::InvalidResponse {
                    field: "tlsCertificateSha256",
                })?;
        if welcome_certificate_sha256 != tls_certificate_sha256 {
            return Err(CoreConnectionError::CertificateFingerprintMismatch);
        }

        Ok(Self {
            capabilities,
            core_id,
            heartbeat_seconds,
            protocol,
            tls_certificate_sha256,
            transport,
        })
    }

    #[must_use]
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    #[must_use]
    pub const fn core_id(&self) -> CoreId {
        self.core_id
    }

    #[must_use]
    pub const fn heartbeat_seconds(&self) -> u64 {
        self.heartbeat_seconds
    }

    #[must_use]
    pub const fn protocol(&self) -> ProtocolVersion {
        self.protocol
    }

    #[must_use]
    pub fn tls_certificate_sha256(&self) -> &str {
        &self.tls_certificate_sha256
    }

    pub async fn ping(&mut self) -> Result<String, CoreConnectionError> {
        let result = self.request("system.ping", json!({})).await?;

        response_field(&result, "receivedAt")?
            .as_str()
            .map(str::to_owned)
            .ok_or(CoreConnectionError::InvalidResponse {
                field: "receivedAt",
            })
    }

    pub async fn system_info(&mut self) -> Result<Value, CoreConnectionError> {
        self.request("system.info", json!({})).await
    }

    pub async fn list_managed_runtimes(
        &mut self,
    ) -> Result<Vec<ManagedRuntime>, CoreConnectionError> {
        let result = self.request("runtime.list", json!({})).await?;
        let items = response_field(&result, "items")?;

        from_value(items).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "managedRuntimes",
        })
    }

    pub async fn install_runtime(
        &mut self,
        manifest: &RuntimeInstallManifest,
        set_as_default: bool,
        idempotency_key: &str,
    ) -> Result<Value, CoreConnectionError> {
        let manifest = to_value(manifest).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "runtimeManifest",
        })?;
        self.request_with_idempotency(
            "runtime.install",
            json!({
                "manifest": manifest,
                "setAsDefault": set_as_default,
            }),
            Some(idempotency_key),
        )
        .await
    }

    pub async fn verify_runtime(
        &mut self,
        runtime_id: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreConnectionError> {
        self.request_with_idempotency(
            "runtime.verify",
            json!({ "runtimeId": runtime_id }),
            Some(idempotency_key),
        )
        .await
    }

    pub async fn get_runtime_task(
        &mut self,
        task_id: &TaskId,
    ) -> Result<Value, CoreConnectionError> {
        self.request("runtime.task.get", json!({ "taskId": task_id }))
            .await
    }

    pub async fn delete_runtime(
        &mut self,
        runtime_id: &str,
        idempotency_key: &str,
    ) -> Result<TaskId, CoreConnectionError> {
        let result = self
            .request_with_idempotency(
                "runtime.delete",
                json!({ "runtimeId": runtime_id }),
                Some(idempotency_key),
            )
            .await?;
        let task_id = response_field(&result, "taskId")?;

        from_value(task_id).map_err(|_| CoreConnectionError::InvalidResponse { field: "taskId" })
    }

    pub async fn resolve_provision(
        &mut self,
        plan: &ProvisionPlan,
    ) -> Result<Value, CoreConnectionError> {
        let plan = to_value(plan).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "provisionPlan",
        })?;
        self.request("provision.resolve", plan).await
    }

    pub async fn execute_provision(
        &mut self,
        plan: &ProvisionPlan,
        plan_hash: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreConnectionError> {
        let plan = to_value(plan).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "provisionPlan",
        })?;
        self.request_with_idempotency(
            "provision.execute",
            json!({
                "resolvedPlan": plan,
                "planHash": plan_hash,
            }),
            Some(idempotency_key),
        )
        .await
    }

    pub async fn get_provision_task(
        &mut self,
        task_id: &TaskId,
    ) -> Result<Value, CoreConnectionError> {
        self.request("provision.task.get", json!({ "taskId": task_id }))
            .await
    }

    pub async fn get_bedrock_profile(
        &mut self,
        instance_id: &InstanceId,
    ) -> Result<BedrockManagementProfile, CoreConnectionError> {
        let result = self
            .request("bedrock.profile", json!({ "instanceId": instance_id }))
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "bedrockProfile",
        })
    }

    pub async fn list_proxy_subservers(
        &mut self,
        proxy_instance_id: &InstanceId,
    ) -> Result<Vec<ProxySubserver>, CoreConnectionError> {
        let result = self
            .request(
                "proxy.subserver.list",
                json!({ "proxyInstanceId": proxy_instance_id }),
            )
            .await?;
        let items = response_field(&result, "items")?;

        from_value(items).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "proxySubservers",
        })
    }

    pub async fn upsert_proxy_subserver(
        &mut self,
        proxy_instance_id: &InstanceId,
        subserver: &ProxySubserver,
        idempotency_key: &str,
    ) -> Result<ProxySubserver, CoreConnectionError> {
        let subserver = to_value(subserver).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "proxySubserver",
        })?;
        let result = self
            .request_with_idempotency(
                "proxy.subserver.upsert",
                json!({
                    "proxyInstanceId": proxy_instance_id,
                    "subserver": subserver,
                }),
                Some(idempotency_key),
            )
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "proxySubserver",
        })
    }

    pub async fn delete_proxy_subserver(
        &mut self,
        proxy_instance_id: &InstanceId,
        subserver_id: &str,
        idempotency_key: &str,
    ) -> Result<(), CoreConnectionError> {
        self.request_with_idempotency(
            "proxy.subserver.delete",
            json!({
                "proxyInstanceId": proxy_instance_id,
                "subserverId": subserver_id,
            }),
            Some(idempotency_key),
        )
        .await?;

        Ok(())
    }

    pub async fn create_instance(
        &mut self,
        instance: &InstanceCreate,
    ) -> Result<Instance, CoreConnectionError> {
        self.create_instance_with_idempotency(instance, None).await
    }

    pub async fn create_instance_with_idempotency(
        &mut self,
        instance: &InstanceCreate,
        idempotency_key: Option<&str>,
    ) -> Result<Instance, CoreConnectionError> {
        let params = to_value(instance)
            .map_err(|_| CoreConnectionError::InvalidResponse { field: "instance" })?;
        let result = self
            .request_with_idempotency("instance.create", params, idempotency_key)
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse { field: "instance" })
    }

    pub async fn get_instance(
        &mut self,
        instance_id: &InstanceId,
    ) -> Result<Instance, CoreConnectionError> {
        let result = self
            .request("instance.get", json!({ "instanceId": instance_id }))
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse { field: "instance" })
    }

    pub async fn update_instance(
        &mut self,
        instance_id: &InstanceId,
        expected_revision: u64,
        update: &InstanceUpdate,
    ) -> Result<Instance, CoreConnectionError> {
        let patch = to_value(update).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "instanceUpdate",
        })?;
        let result = self
            .request(
                "instance.update",
                json!({
                    "instanceId": instance_id,
                    "expectedRevision": expected_revision,
                    "patch": patch,
                }),
            )
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse { field: "instance" })
    }

    pub async fn list_instances(&mut self) -> Result<InstancePage, CoreConnectionError> {
        self.list_instances_with_filters(None, None, None).await
    }

    pub async fn list_instances_with_filters(
        &mut self,
        cursor: Option<&InstanceId>,
        limit: Option<usize>,
        state: Option<InstanceState>,
    ) -> Result<InstancePage, CoreConnectionError> {
        let mut params = json!({});
        if let Some(cursor) = cursor {
            params["cursor"] = json!(cursor);
        }
        if let Some(limit) = limit {
            params["limit"] = json!(limit);
        }
        if let Some(state) = state {
            params["state"] = json!(state);
        }
        let result = self.request("instance.list", params).await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "instancePage",
        })
    }

    pub async fn start_instance(
        &mut self,
        instance_id: &InstanceId,
        idempotency_key: &str,
    ) -> Result<TaskId, CoreConnectionError> {
        self.lifecycle_request(
            "instance.start",
            json!({ "instanceId": instance_id }),
            idempotency_key,
        )
        .await
    }

    pub async fn stop_instance(
        &mut self,
        instance_id: &InstanceId,
        timeout_seconds: Option<u16>,
        idempotency_key: &str,
    ) -> Result<TaskId, CoreConnectionError> {
        let mut params = json!({ "instanceId": instance_id });
        if let Some(timeout_seconds) = timeout_seconds {
            params["timeoutSeconds"] = json!(timeout_seconds);
        }

        self.lifecycle_request("instance.stop", params, idempotency_key)
            .await
    }

    pub async fn kill_instance(
        &mut self,
        instance_id: &InstanceId,
        idempotency_key: &str,
    ) -> Result<TaskId, CoreConnectionError> {
        self.lifecycle_request(
            "instance.kill",
            json!({
                "instanceId": instance_id,
                "confirmation": instance_id,
            }),
            idempotency_key,
        )
        .await
    }

    pub async fn send_instance_command(
        &mut self,
        instance_id: &InstanceId,
        command: &str,
    ) -> Result<String, CoreConnectionError> {
        self.send_instance_command_with_idempotency(instance_id, command, None)
            .await
    }

    pub async fn send_instance_command_with_idempotency(
        &mut self,
        instance_id: &InstanceId,
        command: &str,
        idempotency_key: Option<&str>,
    ) -> Result<String, CoreConnectionError> {
        let result = self
            .request_with_idempotency(
                "instance.command",
                json!({
                    "instanceId": instance_id,
                    "command": command,
                }),
                idempotency_key,
            )
            .await?;

        response_field(&result, "acceptedAt")?
            .as_str()
            .map(str::to_owned)
            .ok_or(CoreConnectionError::InvalidResponse {
                field: "acceptedAt",
            })
    }

    pub async fn get_instance_logs(
        &mut self,
        instance_id: &InstanceId,
        after: Option<&str>,
        before: Option<&str>,
        limit: Option<usize>,
    ) -> Result<InstanceLogPage, CoreConnectionError> {
        let mut params = json!({ "instanceId": instance_id });
        if let Some(after) = after {
            params["after"] = json!(after);
        }
        if let Some(before) = before {
            params["before"] = json!(before);
        }
        if let Some(limit) = limit {
            params["limit"] = json!(limit);
        }
        let result = self.request("instance.logs", params).await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "instanceLogPage",
        })
    }

    pub async fn get_instance_metrics(
        &mut self,
        instance_id: &InstanceId,
        range: Option<&str>,
        resolution: Option<&str>,
    ) -> Result<Vec<InstanceMetricSample>, CoreConnectionError> {
        let mut params = json!({ "instanceId": instance_id });
        if let Some(range) = range {
            params["range"] = json!(range);
        }
        if let Some(resolution) = resolution {
            params["resolution"] = json!(resolution);
        }
        let result = self.request("instance.metrics", params).await?;
        let series = response_field(&result, "series")?;

        from_value(series).map_err(|_| CoreConnectionError::InvalidResponse { field: "series" })
    }

    pub async fn list_instance_files(
        &mut self,
        instance_id: &InstanceId,
        path: &str,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<FilePage, CoreConnectionError> {
        let mut params = json!({
            "instanceId": instance_id,
            "path": path,
        });
        if let Some(cursor) = cursor {
            params["cursor"] = json!(cursor);
        }
        if let Some(limit) = limit {
            params["limit"] = json!(limit);
        }
        let result = self.request("file.list", params).await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse { field: "filePage" })
    }

    pub async fn read_instance_file(
        &mut self,
        instance_id: &InstanceId,
        path: &str,
        offset: u64,
        length: usize,
    ) -> Result<FileContent, CoreConnectionError> {
        let result = self
            .request(
                "file.read",
                json!({
                    "instanceId": instance_id,
                    "path": path,
                    "offset": offset,
                    "length": length,
                }),
            )
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "fileContent",
        })
    }

    pub async fn write_instance_file(
        &mut self,
        instance_id: &InstanceId,
        path: &str,
        content: &[u8],
        expected_sha256: Option<&str>,
        idempotency_key: &str,
    ) -> Result<FileEntry, CoreConnectionError> {
        let mut params = json!({
            "instanceId": instance_id,
            "path": path,
            "dataBase64": STANDARD.encode(content),
        });
        if let Some(expected_sha256) = expected_sha256 {
            params["expectedSha256"] = json!(expected_sha256);
        }
        let result = self
            .request_with_idempotency("file.write", params, Some(idempotency_key))
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse { field: "fileEntry" })
    }

    pub async fn create_instance_directory(
        &mut self,
        instance_id: &InstanceId,
        path: &str,
        recursive: bool,
        idempotency_key: &str,
    ) -> Result<FileEntry, CoreConnectionError> {
        let result = self
            .request_with_idempotency(
                "file.mkdir",
                json!({
                    "instanceId": instance_id,
                    "path": path,
                    "recursive": recursive,
                }),
                Some(idempotency_key),
            )
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse { field: "fileEntry" })
    }

    pub async fn move_instance_file(
        &mut self,
        instance_id: &InstanceId,
        from: &str,
        to: &str,
        overwrite: bool,
        idempotency_key: &str,
    ) -> Result<FileEntry, CoreConnectionError> {
        let result = self
            .request_with_idempotency(
                "file.move",
                json!({
                    "instanceId": instance_id,
                    "from": from,
                    "to": to,
                    "overwrite": overwrite,
                }),
                Some(idempotency_key),
            )
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse { field: "fileEntry" })
    }

    pub async fn delete_instance_file(
        &mut self,
        instance_id: &InstanceId,
        path: &str,
        recursive: bool,
        idempotency_key: &str,
    ) -> Result<TaskId, CoreConnectionError> {
        let result = self
            .request_with_idempotency(
                "file.delete",
                json!({
                    "instanceId": instance_id,
                    "path": path,
                    "recursive": recursive,
                    "confirmation": "DELETE",
                }),
                Some(idempotency_key),
            )
            .await?;
        let task_id = response_field(&result, "taskId")?;

        from_value(task_id).map_err(|_| CoreConnectionError::InvalidResponse { field: "taskId" })
    }

    pub async fn batch_instance_files(
        &mut self,
        instance_id: &InstanceId,
        operations: Vec<Value>,
        idempotency_key: &str,
    ) -> Result<TaskId, CoreConnectionError> {
        let result = self
            .request_with_idempotency(
                "file.batch",
                json!({
                    "instanceId": instance_id,
                    "operations": operations,
                }),
                Some(idempotency_key),
            )
            .await?;
        let task_id = response_field(&result, "taskId")?;

        from_value(task_id).map_err(|_| CoreConnectionError::InvalidResponse { field: "taskId" })
    }

    pub async fn get_file_task(&mut self, task_id: &TaskId) -> Result<Value, CoreConnectionError> {
        self.request("file.task.get", json!({ "taskId": task_id }))
            .await
    }

    pub async fn begin_file_upload(
        &mut self,
        instance_id: &InstanceId,
        path: &str,
        size_bytes: u64,
        sha256: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreConnectionError> {
        self.request_with_idempotency(
            "transfer.begin",
            json!({
                "instanceId": instance_id,
                "path": path,
                "size": size_bytes,
                "sha256": sha256,
                "mode": "UPLOAD",
            }),
            Some(idempotency_key),
        )
        .await
    }

    pub async fn upload_file_chunk(
        &mut self,
        transfer_id: &TaskId,
        offset: u64,
        content: &[u8],
        sha256: Option<&str>,
        idempotency_key: &str,
    ) -> Result<Value, CoreConnectionError> {
        let mut params = json!({
            "transferId": transfer_id,
            "offset": offset,
            "dataBase64": STANDARD.encode(content),
        });
        if let Some(sha256) = sha256 {
            params["sha256"] = json!(sha256);
        }
        self.request_with_idempotency("transfer.chunk", params, Some(idempotency_key))
            .await
    }

    pub async fn commit_file_upload(
        &mut self,
        transfer_id: &TaskId,
        idempotency_key: &str,
    ) -> Result<FileEntry, CoreConnectionError> {
        let result = self
            .request_with_idempotency(
                "transfer.commit",
                json!({ "transferId": transfer_id }),
                Some(idempotency_key),
            )
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse { field: "fileEntry" })
    }

    pub async fn abort_file_upload(
        &mut self,
        transfer_id: &TaskId,
        idempotency_key: &str,
    ) -> Result<(), CoreConnectionError> {
        self.request_with_idempotency(
            "transfer.abort",
            json!({ "transferId": transfer_id }),
            Some(idempotency_key),
        )
        .await?;
        Ok(())
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, CoreConnectionError> {
        self.request_with_idempotency(method, params, None).await
    }

    async fn request_with_idempotency(
        &mut self,
        method: &str,
        params: Value,
        idempotency_key: Option<&str>,
    ) -> Result<Value, CoreConnectionError> {
        let request_id = RequestId::new();
        let request = WireMessage::Request {
            request_id,
            method: method.to_owned(),
            params,
            deadline: None,
            idempotency_key: idempotency_key.map(str::to_owned),
        };

        self.transport.write_message(&request).await?;

        response_result(self.transport.read_message().await?, request_id)
    }

    async fn lifecycle_request(
        &mut self,
        method: &str,
        params: Value,
        idempotency_key: &str,
    ) -> Result<TaskId, CoreConnectionError> {
        let result = self
            .request_with_idempotency(method, params, Some(idempotency_key))
            .await?;
        let task_id = response_field(&result, "taskId")?;

        from_value(task_id).map_err(|_| CoreConnectionError::InvalidResponse { field: "taskId" })
    }
}

fn response_field(response: &Value, field: &'static str) -> Result<Value, CoreConnectionError> {
    response
        .get(field)
        .cloned()
        .ok_or(CoreConnectionError::InvalidResponse { field })
}

fn response_result(
    message: WireMessage,
    expected_request_id: RequestId,
) -> Result<Value, CoreConnectionError> {
    let WireMessage::Response {
        request_id,
        ok,
        result,
        error,
    } = message
    else {
        return Err(CoreConnectionError::InvalidResponse { field: "type" });
    };

    if request_id != expected_request_id {
        return Err(CoreConnectionError::RequestIdMismatch);
    }

    if !ok {
        let code = error
            .map(|response_error| response_error.code)
            .ok_or(CoreConnectionError::InvalidResponse { field: "error" })?;

        return Err(CoreConnectionError::Rejected { code });
    }

    result.ok_or(CoreConnectionError::InvalidResponse { field: "result" })
}
