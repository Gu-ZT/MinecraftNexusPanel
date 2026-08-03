use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use nexus_config::LocalCoreConfig;
use nexus_domain::CoreId;
use nexus_domain::FileContent;
use nexus_domain::FileEntry;
use nexus_domain::FilePage;
use nexus_domain::InstanceCreate;
use nexus_domain::InstanceId;
use nexus_domain::InstanceState;
use nexus_domain::InstanceUpdate;
use nexus_domain::ManagedRuntime;
use nexus_domain::PRODUCT_NAME;
use nexus_domain::ProvisionPlan;
use nexus_domain::ProxySubserver;
use nexus_domain::RuntimeInstallManifest;
use nexus_domain::TaskId;
use nexus_protocol::PresharedKey;
use nexus_protocol::ProtocolVersion;
use nexus_protocol::SessionError;
use nexus_storage::NewCore;
use nexus_storage::SqliteStore;
use nexus_storage::StoredCore;
use serde_json::Value;
use serde_json::from_str;
use serde_json::json;
use serde_json::to_string;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::select;
use tokio::spawn;
use tokio::sync::RwLock;
use tokio::sync::watch::Receiver;
use tokio::sync::watch::Sender;
use tokio::sync::watch::channel;
use tokio::task::spawn_blocking;
use tokio::time::sleep;
use tokio::time::timeout;
use tracing::warn;

use crate::CoreConnection;
use crate::CoreConnectionError;
use crate::CoreCreate;
use crate::CoreEndpoint;
use crate::CoreRegistryError;
use crate::CoreRuntime;
use crate::CoreStatus;
use crate::ManagedCore;
use crate::SecretCipher;

const INITIAL_RECONNECT_DELAY_SECONDS: u64 = 1;
const MAXIMUM_RECONNECT_DELAY_SECONDS: u64 = 30;

#[derive(Clone)]
pub struct CoreRegistry {
    store: SqliteStore,
    cipher: SecretCipher,
    panel_id: String,
    entries: Arc<RwLock<HashMap<CoreId, Arc<ManagedCore>>>>,
    shutdown: Sender<()>,
}

impl CoreRegistry {
    pub fn new(
        store: SqliteStore,
        cipher: SecretCipher,
        panel_id: String,
    ) -> Result<Self, CoreRegistryError> {
        let mut entries = HashMap::new();
        for registration in store.list_cores()? {
            let core_id = CoreId::from_str(registration.id()).map_err(|_| {
                CoreRegistryError::InvalidStoredCore {
                    core_id: registration.id().to_owned(),
                }
            })?;
            let encoded_secret = cipher.decrypt(core_id, registration.secret_envelope())?;
            let encoded_secret = String::from_utf8(encoded_secret).map_err(|_| {
                CoreRegistryError::InvalidStoredCore {
                    core_id: registration.id().to_owned(),
                }
            })?;
            let pre_shared_key = PresharedKey::from_base64url(&encoded_secret).map_err(|_| {
                CoreRegistryError::InvalidStoredCore {
                    core_id: registration.id().to_owned(),
                }
            })?;
            from_str::<Vec<String>>(registration.tags_json())?;
            entries.insert(
                core_id,
                Arc::new(ManagedCore::new(
                    registration,
                    pre_shared_key,
                    None,
                    CoreRuntime::unknown(),
                )),
            );
        }
        let existing: Vec<_> = entries.values().cloned().collect();
        let (shutdown, _) = channel(());
        let registry = Self {
            store,
            cipher,
            panel_id,
            entries: Arc::new(RwLock::new(entries)),
            shutdown,
        };
        for core in existing {
            registry.spawn_connection_monitor(core);
        }

        Ok(registry)
    }

    pub async fn register(&self, request: &CoreCreate) -> Result<Value, CoreRegistryError> {
        if let Some(field) = request.invalid_field() {
            return Err(CoreRegistryError::InvalidRequest { field });
        }
        let pre_shared_key = PresharedKey::from_base64url(request.secret())
            .map_err(CoreRegistryError::InvalidSecret)?;
        let (connection, runtime) = establish_connection(
            request.address(),
            request.skip_certificate_verification(),
            request.connect_timeout_seconds(),
            &pre_shared_key,
            &self.panel_id,
        )
        .await?;
        let core_id = connection.core_id();
        let now = current_timestamp();
        let new_core = NewCore {
            id: core_id.to_string(),
            name: request.name().to_owned(),
            address: request.address().to_owned(),
            secret_envelope: self.cipher.encrypt(core_id, request.secret().as_bytes())?,
            secret_updated_at: now.clone(),
            connect_timeout_seconds: request.connect_timeout_seconds(),
            skip_certificate_verification: request.skip_certificate_verification(),
            tags_json: to_string(&request.normalized_tags())?,
            created_at: now,
        };
        let registration = StoredCore::from_new(&new_core);
        let store = self.store.clone();
        let inserted = spawn_blocking(move || store.insert_core(&new_core)).await??;
        if !inserted {
            return Err(CoreRegistryError::AlreadyExists { core_id });
        }
        let core = Arc::new(ManagedCore::new(
            registration,
            pre_shared_key,
            Some(connection),
            runtime,
        ));
        self.entries.write().await.insert(core_id, core.clone());
        self.spawn_connection_monitor(core.clone());

        core_json(&core).await
    }

    pub async fn ensure_local_core(
        &self,
        config: &LocalCoreConfig,
    ) -> Result<(), CoreRegistryError> {
        let pre_shared_key = PresharedKey::from_base64url(config.encoded_pre_shared_key())
            .map_err(CoreRegistryError::InvalidSecret)?;
        let address = config.listen_address().to_string();
        let (connection, runtime) =
            establish_connection(&address, false, 10, &pre_shared_key, &self.panel_id).await?;
        if connection.core_id() != config.core_id() {
            return Err(CoreRegistryError::LocalCoreIdMismatch {
                expected: config.core_id(),
                actual: connection.core_id(),
            });
        }
        let now = current_timestamp();
        let new_core = NewCore {
            id: config.core_id().to_string(),
            name: "Loopback Core".to_owned(),
            address,
            secret_envelope: self
                .cipher
                .encrypt(config.core_id(), config.encoded_pre_shared_key().as_bytes())?,
            secret_updated_at: now.clone(),
            connect_timeout_seconds: 10,
            skip_certificate_verification: false,
            tags_json: to_string(&["local", "loopback"])?,
            created_at: now,
        };
        let registration = StoredCore::from_new(&new_core);
        let store = self.store.clone();
        let inserted = spawn_blocking(move || store.insert_core(&new_core)).await??;
        if !inserted && self.entries.read().await.contains_key(&config.core_id()) {
            return Ok(());
        }
        let core = Arc::new(ManagedCore::new(
            registration,
            pre_shared_key,
            Some(connection),
            runtime,
        ));
        self.entries
            .write()
            .await
            .insert(config.core_id(), core.clone());
        self.spawn_connection_monitor(core);

        Ok(())
    }

    pub async fn list(&self) -> Result<Value, CoreRegistryError> {
        let mut cores: Vec<_> = self.entries.read().await.values().cloned().collect();
        cores.sort_by(|left, right| left.registration.id().cmp(right.registration.id()));
        let mut items = Vec::with_capacity(cores.len());
        for core in cores {
            items.push(core_json(&core).await?);
        }

        Ok(json!({
            "items": items,
            "nextCursor": null,
        }))
    }

    pub async fn get(&self, core_id: CoreId) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        core_json(&core).await
    }

    pub async fn test_connection(&self, core_id: CoreId) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let started_at = Instant::now();
        let (connection, _) = establish_connection(
            core.registration.address(),
            core.registration.skip_certificate_verification(),
            core.registration.connect_timeout_seconds(),
            &core.pre_shared_key,
            &self.panel_id,
        )
        .await?;

        Ok(json!({
            "success": true,
            "latencyMs": elapsed_milliseconds(started_at),
            "protocolVersion": protocol_text(connection.protocol()),
        }))
    }

    pub async fn reconnect(&self, core_id: CoreId) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        core.connection.lock().await.take();
        core.runtime.write().await.mark_reconnecting();
        core.reconnect.notify_waiters();

        core_json(&core).await
    }

    pub async fn create_instance(
        &self,
        core_id: CoreId,
        request: &InstanceCreate,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let instance = connection
            .create_instance_with_idempotency(request, Some(idempotency_key))
            .await?;

        Ok(instance_json(core_id, &json!(instance)))
    }

    pub async fn get_instance(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let instance = connection.get_instance(instance_id).await?;

        Ok(instance_json(core_id, &json!(instance)))
    }

    pub async fn list_managed_runtimes(&self, core_id: CoreId) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let runtimes: Vec<ManagedRuntime> = connection.list_managed_runtimes().await?;

        Ok(json!({ "items": runtimes }))
    }

    pub async fn install_runtime(
        &self,
        core_id: CoreId,
        manifest: &RuntimeInstallManifest,
        set_as_default: bool,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .install_runtime(manifest, set_as_default, idempotency_key)
            .await?)
    }

    pub async fn verify_runtime(
        &self,
        core_id: CoreId,
        runtime_id: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .verify_runtime(runtime_id, idempotency_key)
            .await?)
    }

    pub async fn get_runtime_task(
        &self,
        core_id: CoreId,
        task_id: &TaskId,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection.get_runtime_task(task_id).await?)
    }

    pub async fn delete_runtime(
        &self,
        core_id: CoreId,
        runtime_id: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let task_id = connection
            .delete_runtime(runtime_id, idempotency_key)
            .await?;

        Ok(task_accepted_json(task_id))
    }

    pub async fn resolve_provision(
        &self,
        core_id: CoreId,
        plan: &ProvisionPlan,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection.resolve_provision(plan).await?)
    }

    pub async fn execute_provision(
        &self,
        core_id: CoreId,
        plan: &ProvisionPlan,
        plan_hash: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .execute_provision(plan, plan_hash, idempotency_key)
            .await?)
    }

    pub async fn get_provision_task(
        &self,
        core_id: CoreId,
        task_id: &TaskId,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection.get_provision_task(task_id).await?)
    }

    pub async fn get_bedrock_profile(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let profile = connection.get_bedrock_profile(instance_id).await?;

        Ok(json!(profile))
    }

    pub async fn list_proxy_subservers(
        &self,
        core_id: CoreId,
        proxy_instance_id: &InstanceId,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let items = connection.list_proxy_subservers(proxy_instance_id).await?;

        Ok(json!({ "items": items }))
    }

    pub async fn upsert_proxy_subserver(
        &self,
        core_id: CoreId,
        proxy_instance_id: &InstanceId,
        subserver: &ProxySubserver,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let item = connection
            .upsert_proxy_subserver(proxy_instance_id, subserver, idempotency_key)
            .await?;

        Ok(json!(item))
    }

    pub async fn delete_proxy_subserver(
        &self,
        core_id: CoreId,
        proxy_instance_id: &InstanceId,
        subserver_id: &str,
        idempotency_key: &str,
    ) -> Result<(), CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        connection
            .delete_proxy_subserver(proxy_instance_id, subserver_id, idempotency_key)
            .await?;

        Ok(())
    }

    pub async fn update_instance(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        expected_revision: u64,
        update: &InstanceUpdate,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let instance = connection
            .update_instance(instance_id, expected_revision, update)
            .await?;

        Ok(instance_json(core_id, &json!(instance)))
    }

    pub async fn list_instances(
        &self,
        core_id: CoreId,
        cursor: Option<&InstanceId>,
        limit: Option<usize>,
        state: Option<InstanceState>,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let page = connection
            .list_instances_with_filters(cursor, limit, state)
            .await?;

        Ok(instance_page_json(core_id, &json!(page)))
    }

    pub async fn start_instance(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let task_id = connection
            .start_instance(instance_id, idempotency_key)
            .await?;

        Ok(task_accepted_json(task_id))
    }

    pub async fn stop_instance(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        timeout_seconds: Option<u16>,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let task_id = connection
            .stop_instance(instance_id, timeout_seconds, idempotency_key)
            .await?;

        Ok(task_accepted_json(task_id))
    }

    pub async fn kill_instance(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let task_id = connection
            .kill_instance(instance_id, idempotency_key)
            .await?;

        Ok(task_accepted_json(task_id))
    }

    pub async fn send_instance_command(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        command: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let accepted_at = connection
            .send_instance_command_with_idempotency(instance_id, command, Some(idempotency_key))
            .await?;

        Ok(json!({ "acceptedAt": accepted_at }))
    }

    pub async fn get_instance_logs(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        after: Option<&str>,
        before: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let page = connection
            .get_instance_logs(instance_id, after, before, limit)
            .await?;

        Ok(json!(page))
    }

    pub async fn get_instance_metrics(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        range: Option<&str>,
        resolution: Option<&str>,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let series = connection
            .get_instance_metrics(instance_id, range, resolution)
            .await?;

        Ok(json!({ "series": series }))
    }

    pub async fn scan_config_documents(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection.scan_config_documents(instance_id).await?)
    }

    pub async fn get_config_document(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        document_id: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .get_config_document(instance_id, document_id)
            .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn patch_config_document(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        document_id: &str,
        revision: &str,
        patch: &Value,
        idempotency_key: &str,
        allow_lossy: bool,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .patch_config_document(
                instance_id,
                document_id,
                revision,
                patch,
                idempotency_key,
                allow_lossy,
            )
            .await?)
    }

    pub async fn list_instance_files(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        path: &str,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<FilePage, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .list_instance_files(instance_id, path, cursor, limit)
            .await?)
    }

    pub async fn read_instance_file(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        path: &str,
        offset: u64,
        length: usize,
    ) -> Result<FileContent, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .read_instance_file(instance_id, path, offset, length)
            .await?)
    }

    pub async fn write_instance_file(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        path: &str,
        content: &[u8],
        expected_sha256: Option<&str>,
        idempotency_key: &str,
    ) -> Result<FileEntry, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .write_instance_file(instance_id, path, content, expected_sha256, idempotency_key)
            .await?)
    }

    pub async fn create_instance_directory(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        path: &str,
        recursive: bool,
        idempotency_key: &str,
    ) -> Result<FileEntry, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .create_instance_directory(instance_id, path, recursive, idempotency_key)
            .await?)
    }

    pub async fn move_instance_file(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        from: &str,
        to: &str,
        overwrite: bool,
        idempotency_key: &str,
    ) -> Result<FileEntry, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .move_instance_file(instance_id, from, to, overwrite, idempotency_key)
            .await?)
    }

    pub async fn delete_instance_file(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        path: &str,
        recursive: bool,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        let task_id = connection
            .delete_instance_file(instance_id, path, recursive, idempotency_key)
            .await?;
        Ok(task_accepted_json(task_id))
    }

    pub async fn batch_instance_files(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        operations: Vec<Value>,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        let task_id = connection
            .batch_instance_files(instance_id, operations, idempotency_key)
            .await?;
        Ok(task_accepted_json(task_id))
    }

    pub async fn create_file_archive(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        paths: Vec<String>,
        output_path: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        let task_id = connection
            .create_file_archive(instance_id, paths, output_path, idempotency_key)
            .await?;
        Ok(task_accepted_json(task_id))
    }

    pub async fn get_file_task(
        &self,
        core_id: CoreId,
        task_id: &TaskId,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection.get_file_task(task_id).await?)
    }

    pub async fn begin_file_upload(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        path: &str,
        size_bytes: u64,
        sha256: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .begin_file_upload(instance_id, path, size_bytes, sha256, idempotency_key)
            .await?)
    }

    pub async fn begin_file_download(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        path: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .begin_file_download(instance_id, path, idempotency_key)
            .await?)
    }

    pub async fn upload_file_chunk(
        &self,
        core_id: CoreId,
        transfer_id: &TaskId,
        offset: u64,
        content: &[u8],
        sha256: &str,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .upload_file_chunk(transfer_id, offset, content, Some(sha256), idempotency_key)
            .await?)
    }

    pub async fn read_file_download_chunk(
        &self,
        core_id: CoreId,
        transfer_id: &TaskId,
        offset: u64,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .read_file_download_chunk(transfer_id, offset)
            .await?)
    }

    pub async fn commit_file_upload(
        &self,
        core_id: CoreId,
        transfer_id: &TaskId,
        idempotency_key: &str,
    ) -> Result<FileEntry, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .commit_file_upload(transfer_id, idempotency_key)
            .await?)
    }

    pub async fn abort_file_upload(
        &self,
        core_id: CoreId,
        transfer_id: &TaskId,
        idempotency_key: &str,
    ) -> Result<(), CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .abort_file_upload(transfer_id, idempotency_key)
            .await?)
    }

    pub async fn commit_file_download(
        &self,
        core_id: CoreId,
        transfer_id: &TaskId,
        idempotency_key: &str,
    ) -> Result<(), CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        connection
            .commit_file_download(transfer_id, idempotency_key)
            .await?;
        Ok(())
    }

    pub async fn abort_file_download(
        &self,
        core_id: CoreId,
        transfer_id: &TaskId,
        idempotency_key: &str,
    ) -> Result<(), CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        connection
            .abort_file_download(transfer_id, idempotency_key)
            .await?;
        Ok(())
    }

    async fn find(&self, core_id: CoreId) -> Result<Arc<ManagedCore>, CoreRegistryError> {
        self.entries
            .read()
            .await
            .get(&core_id)
            .cloned()
            .ok_or(CoreRegistryError::NotFound { core_id })
    }

    fn spawn_connection_monitor(&self, core: Arc<ManagedCore>) {
        let panel_id = self.panel_id.clone();
        let shutdown = self.shutdown.subscribe();
        spawn(async move {
            monitor_connection(core, panel_id, shutdown).await;
        });
    }
}

fn task_accepted_json(task_id: TaskId) -> Value {
    json!({
        "taskId": task_id,
        "acceptedAt": current_timestamp(),
    })
}

fn instance_page_json(core_id: CoreId, page: &Value) -> Value {
    let mut page = page.clone();
    if let Some(items) = page.get_mut("items").and_then(Value::as_array_mut) {
        for item in items {
            add_core_id(core_id, item);
        }
    }

    page
}

fn instance_json(core_id: CoreId, instance: &Value) -> Value {
    let mut instance = instance.clone();
    add_core_id(core_id, &mut instance);

    instance
}

fn add_core_id(core_id: CoreId, value: &mut Value) {
    if let Some(object) = value.as_object_mut() {
        object.insert("coreId".to_owned(), json!(core_id));
    }
}

async fn monitor_connection(core: Arc<ManagedCore>, panel_id: String, mut shutdown: Receiver<()>) {
    let mut reconnect_delay_seconds = INITIAL_RECONNECT_DELAY_SECONDS;
    loop {
        if core.connection.lock().await.is_none() {
            match establish_connection(
                core.registration.address(),
                core.registration.skip_certificate_verification(),
                core.registration.connect_timeout_seconds(),
                &core.pre_shared_key,
                &panel_id,
            )
            .await
            {
                Ok((connection, runtime)) => {
                    *core.connection.lock().await = Some(connection);
                    *core.runtime.write().await = runtime;
                    reconnect_delay_seconds = INITIAL_RECONNECT_DELAY_SECONDS;
                }
                Err(error) => {
                    core.runtime
                        .write()
                        .await
                        .mark_failure(status_for_error(&error));
                    warn!(
                        core_id = core.registration.id(),
                        error = %error,
                        "Core connection attempt failed"
                    );
                    select! {
                        () = sleep(Duration::from_secs(reconnect_delay_seconds)) => {}
                        () = core.reconnect.notified() => {}
                        result = shutdown.changed() => {
                            if result.is_err() {
                                return;
                            }
                        }
                    }
                    reconnect_delay_seconds =
                        (reconnect_delay_seconds * 2).min(MAXIMUM_RECONNECT_DELAY_SECONDS);
                    continue;
                }
            }
        }

        let heartbeat_seconds = core.connection.lock().await.as_ref().map_or(
            INITIAL_RECONNECT_DELAY_SECONDS,
            CoreConnection::heartbeat_seconds,
        );
        select! {
            () = sleep(Duration::from_secs(heartbeat_seconds)) => {}
            () = core.reconnect.notified() => {
                core.connection.lock().await.take();
                core.runtime.write().await.mark_reconnecting();
                continue;
            }
            result = shutdown.changed() => {
                if result.is_err() {
                    return;
                }
            }
        }

        let started_at = Instant::now();
        let ping_result = {
            let mut connection = core.connection.lock().await;
            let result = match connection.as_mut() {
                Some(connection) => timeout(
                    Duration::from_secs(u64::from(core.registration.connect_timeout_seconds())),
                    connection.ping(),
                )
                .await
                .map_err(|_| CoreRegistryError::ConnectionTimeout)
                .and_then(|result| result.map_err(CoreRegistryError::from)),
                None => continue,
            };
            if result.is_err() {
                connection.take();
            }
            result
        };
        match ping_result {
            Ok(_) => core
                .runtime
                .write()
                .await
                .mark_ping(elapsed_milliseconds(started_at), current_timestamp()),
            Err(error) => {
                core.runtime
                    .write()
                    .await
                    .mark_failure(status_for_error(&error));
                warn!(
                    core_id = core.registration.id(),
                    error = %error,
                    "Core heartbeat failed"
                );
            }
        }
    }
}

async fn establish_connection(
    address: &str,
    skip_certificate_verification: bool,
    connect_timeout_seconds: u32,
    pre_shared_key: &PresharedKey,
    panel_id: &str,
) -> Result<(CoreConnection, CoreRuntime), CoreRegistryError> {
    let endpoint = CoreEndpoint::parse(address, skip_certificate_verification)
        .map_err(CoreConnectionError::from)?;
    let started_at = Instant::now();
    let mut connection = timeout(
        Duration::from_secs(u64::from(connect_timeout_seconds)),
        CoreConnection::connect_endpoint(&endpoint, pre_shared_key, panel_id, PRODUCT_NAME),
    )
    .await
    .map_err(|_| CoreRegistryError::ConnectionTimeout)??;
    let system_info = timeout(
        Duration::from_secs(u64::from(connect_timeout_seconds)),
        connection.system_info(),
    )
    .await
    .map_err(|_| CoreRegistryError::ConnectionTimeout)??;
    let runtime = CoreRuntime {
        status: CoreStatus::Online,
        latency_milliseconds: Some(elapsed_milliseconds(started_at)),
        last_seen_at: Some(current_timestamp()),
        version: system_info
            .get("serverVersion")
            .and_then(Value::as_str)
            .map(str::to_owned),
        protocol_version: Some(protocol_text(connection.protocol())),
        capabilities: connection.capabilities().to_vec(),
        certificate_verified: Some(endpoint.verify_certificate()),
        tls_certificate_sha256: Some(connection.tls_certificate_sha256().to_owned()),
    };

    Ok((connection, runtime))
}

async fn core_json(core: &ManagedCore) -> Result<Value, CoreRegistryError> {
    let runtime = core.runtime.read().await.clone();
    let tags: Vec<String> = from_str(core.registration.tags_json())?;

    Ok(json!({
        "id": core.registration.id(),
        "name": core.registration.name(),
        "address": core.registration.address(),
        "status": runtime.status.as_str(),
        "latencyMs": runtime.latency_milliseconds,
        "lastSeenAt": runtime.last_seen_at,
        "version": runtime.version,
        "protocolVersion": runtime.protocol_version,
        "capabilities": runtime.capabilities,
        "secretConfigured": true,
        "secretUpdatedAt": core.registration.secret_updated_at(),
        "skipCertificateVerification": core.registration.skip_certificate_verification(),
        "certificateVerified": runtime.certificate_verified,
        "tlsCertificateSha256": runtime.tls_certificate_sha256,
        "tags": tags,
        "revision": core.registration.revision(),
    }))
}

fn status_for_error(error: &CoreRegistryError) -> CoreStatus {
    match error {
        CoreRegistryError::Connection(CoreConnectionError::ProtocolVersion(_)) => {
            CoreStatus::Incompatible
        }
        CoreRegistryError::Connection(
            CoreConnectionError::CertificateFingerprintMismatch
            | CoreConnectionError::Session(SessionError::Noise(_))
            | CoreConnectionError::Tls(_),
        ) => CoreStatus::AuthFailed,
        CoreRegistryError::Connection(_) | CoreRegistryError::ConnectionTimeout => {
            CoreStatus::Offline
        }
        _ => CoreStatus::Unknown,
    }
}

fn elapsed_milliseconds(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn protocol_text(protocol: ProtocolVersion) -> String {
    format!("{}.{}", protocol.major, protocol.minor)
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}
