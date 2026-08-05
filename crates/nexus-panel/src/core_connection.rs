//! Panel 到 Core 的长连接客户端和领域请求适配。
//!
//! 连接建立时完成 TLS、Noise PSK、会话问候、协议版本和证书指纹校验；公开方法
//! 只负责组装协议请求并解码领域响应，幂等操作必须由调用方提供稳定幂等键。

use std::net::SocketAddr;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use nexus_domain::BedrockHealth;
use nexus_domain::BedrockManagementProfile;
use nexus_domain::BedrockPortCheck;
use nexus_domain::CoreId;
use nexus_domain::CpuPolicy;
use nexus_domain::CpuReservation;
use nexus_domain::CpuTopology;
use nexus_domain::FileContent;
use nexus_domain::FileEntry;
use nexus_domain::FilePage;
use nexus_domain::Instance;
use nexus_domain::InstanceAuditPage;
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
use nexus_domain::ProxySubserverHealth;
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

const PANEL_CAPABILITIES: [&str; 16] = [
    "bedrock-health",
    "config",
    "cpu-topology",
    "cpu-policy",
    "cpu-reservations",
    "events",
    "files",
    "instances",
    "instance-audit",
    "metrics",
    "proxy-orchestration",
    "proxy-subservers",
    "provision",
    "runtimes",
    "settings",
    "transfer-v1",
];

/// 已完成安全握手、可复用的 Core 协议连接。
pub struct CoreConnection {
    capabilities: Vec<String>,
    core_id: CoreId,
    heartbeat_seconds: u64,
    protocol: ProtocolVersion,
    tls_certificate_sha256: String,
    transport: NoiseTransport<TlsClientStream<TcpStream>>,
}

impl CoreConnection {
    /// 连接到指定 Socket 地址并完成 Core 会话握手。
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

    /// 解析地址、建立 TCP/TLS/Noise 连接并完成会话握手。
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

    /// 使用已解析端点建立连接并校验 Core 欢迎消息。
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

    /// 返回 Core 在握手中声明的能力列表。
    #[must_use]
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    /// 返回远端 Core 标识。
    #[must_use]
    pub const fn core_id(&self) -> CoreId {
        self.core_id
    }

    /// 返回 Core 建议的心跳间隔，单位为秒。
    #[must_use]
    pub const fn heartbeat_seconds(&self) -> u64 {
        self.heartbeat_seconds
    }

    /// 返回协商后的协议版本。
    #[must_use]
    pub const fn protocol(&self) -> ProtocolVersion {
        self.protocol
    }

    /// 返回本次连接实际收到的 TLS 证书 SHA-256 指纹。
    #[must_use]
    pub fn tls_certificate_sha256(&self) -> &str {
        &self.tls_certificate_sha256
    }

    /// 发送系统 ping 并返回 Core 的接收时间。
    pub async fn ping(&mut self) -> Result<String, CoreConnectionError> {
        let result = self.request("system.ping", json!({})).await?;

        response_field(&result, "receivedAt")?
            .as_str()
            .map(str::to_owned)
            .ok_or(CoreConnectionError::InvalidResponse {
                field: "receivedAt",
            })
    }

    /// 获取 Core 系统信息 JSON。
    pub async fn system_info(&mut self) -> Result<Value, CoreConnectionError> {
        self.request("system.info", json!({})).await
    }

    /// 获取 Core 启动时缓存的 CPU 拓扑快照。
    pub async fn cpu_topology(&mut self) -> Result<CpuTopology, CoreConnectionError> {
        let result = self.request("cpu.topology", json!({})).await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "cpuTopology",
        })
    }

    /// 请求 Core 预览 CPU policy 的候选和建议集合。
    pub async fn resolve_cpu_policy(
        &mut self,
        policy: &CpuPolicy,
    ) -> Result<Value, CoreConnectionError> {
        let policy = to_value(policy)
            .map_err(|_| CoreConnectionError::InvalidResponse { field: "cpuPolicy" })?;
        self.request("cpu.policy.resolve", policy).await
    }

    /// 列出 Core 当前登记的 CPU 独占预留。
    pub async fn list_cpu_reservations(
        &mut self,
    ) -> Result<Vec<CpuReservation>, CoreConnectionError> {
        let result = self.request("cpu.reservation.list", json!({})).await?;
        let items = response_field(&result, "items")?;

        from_value(items).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "cpuReservations",
        })
    }

    /// 请求 Core 为实例登记 CPU 独占预留。
    ///
    /// Core 返回预留记录和实际选中的 policy；该结果只代表登记成功，不能
    /// 推断宿主机 affinity 已经应用。
    pub async fn reserve_cpu(
        &mut self,
        instance_id: &InstanceId,
        revision: u64,
        policy: &CpuPolicy,
        idempotency_key: &str,
    ) -> Result<Value, CoreConnectionError> {
        let policy = to_value(policy)
            .map_err(|_| CoreConnectionError::InvalidResponse { field: "cpuPolicy" })?;
        self.request_with_idempotency(
            "cpu.reserve",
            json!({
                "instanceId": instance_id,
                "revision": revision,
                "policy": policy,
            }),
            Some(idempotency_key),
        )
        .await
    }

    /// 请求 Core 释放指定 CPU 独占预留。
    pub async fn release_cpu(
        &mut self,
        reservation_id: &TaskId,
        idempotency_key: &str,
    ) -> Result<(), CoreConnectionError> {
        self.request_with_idempotency(
            "cpu.release",
            json!({ "reservationId": reservation_id }),
            Some(idempotency_key),
        )
        .await?;

        Ok(())
    }

    /// 列出 Core 已发现且验证的受管运行时。
    pub async fn list_managed_runtimes(
        &mut self,
    ) -> Result<Vec<ManagedRuntime>, CoreConnectionError> {
        let result = self.request("runtime.list", json!({})).await?;
        let items = response_field(&result, "items")?;

        from_value(items).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "managedRuntimes",
        })
    }

    /// 请求安装受管运行时；操作通过幂等键去重并返回任务信息。
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

    /// 请求验证受管运行时并返回任务信息。
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

    /// 查询运行时安装、验证或删除任务。
    pub async fn get_runtime_task(
        &mut self,
        task_id: &TaskId,
    ) -> Result<Value, CoreConnectionError> {
        self.request("runtime.task.get", json!({ "taskId": task_id }))
            .await
    }

    /// 请求删除受管运行时并返回任务标识。
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

    /// 解析一键搭建计划并取得稳定计划哈希。
    pub async fn resolve_provision(
        &mut self,
        plan: &ProvisionPlan,
    ) -> Result<Value, CoreConnectionError> {
        let plan = to_value(plan).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "provisionPlan",
        })?;
        self.request("provision.resolve", plan).await
    }

    /// 按已确认的计划哈希执行一键搭建。
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

    /// 查询一键搭建任务。
    pub async fn get_provision_task(
        &mut self,
        task_id: &TaskId,
    ) -> Result<Value, CoreConnectionError> {
        self.request("provision.task.get", json!({ "taskId": task_id }))
            .await
    }

    /// 获取基岩服务端或 Geyser 的专用管理画像。
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

    /// 检查基岩 UDP 端口绑定状态。
    pub async fn check_bedrock_port(
        &mut self,
        instance_id: &InstanceId,
    ) -> Result<BedrockPortCheck, CoreConnectionError> {
        let result = self
            .request("bedrock.port.check", json!({ "instanceId": instance_id }))
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "bedrockPortCheck",
        })
    }

    /// 执行基岩 RakNet 健康检查。
    pub async fn check_bedrock_health(
        &mut self,
        instance_id: &InstanceId,
    ) -> Result<BedrockHealth, CoreConnectionError> {
        let result = self
            .request("bedrock.health.check", json!({ "instanceId": instance_id }))
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "bedrockHealth",
        })
    }

    /// 列出代理实例的后端关系。
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

    /// 新增或替换代理后端关系。
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

    /// 删除代理后端关系。
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

    /// 检查代理后端的 TCP 和 Minecraft Status 健康状态。
    pub async fn check_proxy_subserver(
        &mut self,
        proxy_instance_id: &InstanceId,
        subserver_id: &str,
    ) -> Result<ProxySubserverHealth, CoreConnectionError> {
        let result = self
            .request(
                "proxy.subserver.check",
                json!({
                    "proxyInstanceId": proxy_instance_id,
                    "subserverId": subserver_id,
                }),
            )
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "proxySubserverHealth",
        })
    }

    /// 启动代理，可按参数同时编排其后端实例。
    pub async fn start_proxy(
        &mut self,
        proxy_instance_id: &InstanceId,
        include_backends: bool,
        idempotency_key: &str,
    ) -> Result<Value, CoreConnectionError> {
        self.proxy_orchestration_request(
            "proxy.start",
            proxy_instance_id,
            include_backends,
            None,
            idempotency_key,
        )
        .await
    }

    /// 停止代理，可按参数同时编排其后端实例。
    pub async fn stop_proxy(
        &mut self,
        proxy_instance_id: &InstanceId,
        include_backends: bool,
        timeout_seconds: Option<u16>,
        idempotency_key: &str,
    ) -> Result<Value, CoreConnectionError> {
        self.proxy_orchestration_request(
            "proxy.stop",
            proxy_instance_id,
            include_backends,
            timeout_seconds,
            idempotency_key,
        )
        .await
    }

    /// 创建实例；不提供幂等键时由 Core 按普通请求处理。
    pub async fn create_instance(
        &mut self,
        instance: &InstanceCreate,
    ) -> Result<Instance, CoreConnectionError> {
        self.create_instance_with_idempotency(instance, None).await
    }

    /// 创建实例并使用幂等键避免重试重复产生副作用。
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

    /// 获取单个实例快照。
    pub async fn get_instance(
        &mut self,
        instance_id: &InstanceId,
    ) -> Result<Instance, CoreConnectionError> {
        let result = self
            .request("instance.get", json!({ "instanceId": instance_id }))
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse { field: "instance" })
    }

    /// 按期望配置修订号更新实例。
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

    /// 列出全部实例的默认分页。
    pub async fn list_instances(&mut self) -> Result<InstancePage, CoreConnectionError> {
        self.list_instances_with_filters(None, None, None).await
    }

    /// 按游标、数量和生命周期状态筛选实例分页。
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

    /// 启动实例并返回异步任务标识。
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

    /// 请求优雅停止实例，可覆盖默认停止超时。
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

    /// 经过实例 ID 确认后强制终止实例。
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

    /// 显式复位处于 `FAILED` 或 `UNKNOWN` 的实例。
    ///
    /// `UNKNOWN` 只能由管理员明确确认旧进程不再由当前 Core 接管后复位；
    /// 请求使用幂等键，避免 Panel 在连接超时后重复提交复位动作。
    pub async fn reset_instance(
        &mut self,
        instance_id: &InstanceId,
        idempotency_key: &str,
    ) -> Result<Instance, CoreConnectionError> {
        let result = self
            .request_with_idempotency(
                "instance.reset",
                json!({
                    "instanceId": instance_id,
                    "confirmation": "RESET",
                }),
                Some(idempotency_key),
            )
            .await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse { field: "instance" })
    }

    /// 向实例发送命令，不提供幂等键。
    pub async fn send_instance_command(
        &mut self,
        instance_id: &InstanceId,
        command: &str,
    ) -> Result<String, CoreConnectionError> {
        self.send_instance_command_with_idempotency(instance_id, command, None)
            .await
    }

    /// 向实例发送命令并按可选幂等键去重。
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

    /// 按游标读取实例控制台日志。
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

    /// 读取指定实例最新的生命周期审计记录。
    pub async fn list_instance_audit(
        &mut self,
        instance_id: &InstanceId,
        limit: Option<usize>,
    ) -> Result<InstanceAuditPage, CoreConnectionError> {
        let mut params = json!({ "instanceId": instance_id });
        if let Some(limit) = limit {
            params["limit"] = json!(limit);
        }
        let result = self.request("instance.audit.list", params).await?;

        from_value(result).map_err(|_| CoreConnectionError::InvalidResponse {
            field: "instanceAuditPage",
        })
    }

    /// 读取实例资源指标序列。
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

    /// 扫描实例支持的配置文档摘要。
    pub async fn scan_config_documents(
        &mut self,
        instance_id: &InstanceId,
    ) -> Result<Value, CoreConnectionError> {
        self.request("config.scan", json!({ "instanceId": instance_id }))
            .await
    }

    /// 校验实例配置文档之间的端口、EULA 和基岩端监听约束。
    pub async fn validate_config_documents(
        &mut self,
        instance_id: &InstanceId,
    ) -> Result<Value, CoreConnectionError> {
        self.request("config.validate", json!({ "instanceId": instance_id }))
            .await
    }

    /// 获取单个配置文档及其 schema 和修订号。
    pub async fn get_config_document(
        &mut self,
        instance_id: &InstanceId,
        document_id: &str,
    ) -> Result<Value, CoreConnectionError> {
        self.request(
            "config.get",
            json!({
                "instanceId": instance_id,
                "documentId": document_id,
            }),
        )
        .await
    }

    /// 按修订号应用配置 Merge Patch，可显式允许有损格式重写。
    pub async fn patch_config_document(
        &mut self,
        instance_id: &InstanceId,
        document_id: &str,
        revision: &str,
        patch: &Value,
        idempotency_key: &str,
        allow_lossy: bool,
    ) -> Result<Value, CoreConnectionError> {
        self.request_with_idempotency(
            "config.patch",
            json!({
                "instanceId": instance_id,
                "documentId": document_id,
                "revision": revision,
                "patch": patch,
                "allowLossy": allow_lossy,
            }),
            Some(idempotency_key),
        )
        .await
    }

    /// 分页列出实例工作目录中的文件条目。
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

    /// 读取实例文件的 Base64 内容分块。
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

    /// 原子写入实例文件并可校验旧 SHA-256。
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

    /// 创建实例目录。
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

    /// 移动实例目录中的文件或目录。
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

    /// 启动删除实例文件或目录的异步任务。
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

    /// 启动一组文件批处理操作。
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

    /// 创建实例文件 ZIP 归档任务。
    pub async fn create_file_archive(
        &mut self,
        instance_id: &InstanceId,
        paths: Vec<String>,
        output_path: &str,
        idempotency_key: &str,
    ) -> Result<TaskId, CoreConnectionError> {
        let result = self
            .request_with_idempotency(
                "file.archive.create",
                json!({
                    "instanceId": instance_id,
                    "format": "ZIP",
                    "paths": paths,
                    "outputPath": output_path,
                }),
                Some(idempotency_key),
            )
            .await?;
        let task_id = response_field(&result, "taskId")?;

        from_value(task_id).map_err(|_| CoreConnectionError::InvalidResponse { field: "taskId" })
    }

    /// 查询文件操作任务。
    pub async fn get_file_task(&mut self, task_id: &TaskId) -> Result<Value, CoreConnectionError> {
        self.request("file.task.get", json!({ "taskId": task_id }))
            .await
    }

    /// 开始按完整大小和 SHA-256 校验的文件上传。
    pub async fn begin_file_upload(
        &mut self,
        instance_id: &InstanceId,
        path: &str,
        size_bytes: u64,
        sha256: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreConnectionError> {
        self.begin_file_upload_with_expected(
            instance_id,
            path,
            size_bytes,
            sha256,
            None,
            idempotency_key,
        )
        .await
    }

    /// 开始文件上传并同时校验目标文件旧摘要。
    pub async fn begin_file_upload_with_expected(
        &mut self,
        instance_id: &InstanceId,
        path: &str,
        size_bytes: u64,
        sha256: &str,
        expected_sha256: Option<&str>,
        idempotency_key: &str,
    ) -> Result<Value, CoreConnectionError> {
        let mut params = json!({
            "instanceId": instance_id,
            "path": path,
            "size": size_bytes,
            "sha256": sha256,
            "mode": "UPLOAD",
        });
        if let Some(expected_sha256) = expected_sha256 {
            params["expectedSha256"] = json!(expected_sha256);
        }
        self.request_with_idempotency("transfer.begin", params, Some(idempotency_key))
            .await
    }

    /// 开始读取实例文件的分块下载。
    pub async fn begin_file_download(
        &mut self,
        instance_id: &InstanceId,
        path: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreConnectionError> {
        self.request_with_idempotency(
            "transfer.begin",
            json!({
                "instanceId": instance_id,
                "path": path,
                "mode": "DOWNLOAD",
            }),
            Some(idempotency_key),
        )
        .await
    }

    /// 上传一个带连续偏移的文件块。
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

    /// 读取文件下载传输的指定偏移块。
    pub async fn read_file_download_chunk(
        &mut self,
        transfer_id: &TaskId,
        offset: u64,
    ) -> Result<Value, CoreConnectionError> {
        self.request(
            "transfer.chunk",
            json!({
                "transferId": transfer_id,
                "offset": offset,
            }),
        )
        .await
    }

    /// 提交完整文件上传并返回文件条目。
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

    /// 放弃文件上传并释放 Core 传输状态。
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

    /// 提交文件下载传输。
    pub async fn commit_file_download(
        &mut self,
        transfer_id: &TaskId,
        idempotency_key: &str,
    ) -> Result<(), CoreConnectionError> {
        self.request_with_idempotency(
            "transfer.commit",
            json!({ "transferId": transfer_id }),
            Some(idempotency_key),
        )
        .await?;
        Ok(())
    }

    /// 放弃文件下载传输。
    pub async fn abort_file_download(
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

    async fn proxy_orchestration_request(
        &mut self,
        method: &str,
        proxy_instance_id: &InstanceId,
        include_backends: bool,
        timeout_seconds: Option<u16>,
        idempotency_key: &str,
    ) -> Result<Value, CoreConnectionError> {
        let mut params = json!({
            "proxyInstanceId": proxy_instance_id,
            "includeBackends": include_backends,
        });
        if let Some(timeout_seconds) = timeout_seconds {
            params["timeoutSeconds"] = json!(timeout_seconds);
        }
        self.request_with_idempotency(method, params, Some(idempotency_key))
            .await
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
