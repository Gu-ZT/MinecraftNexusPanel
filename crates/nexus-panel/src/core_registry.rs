//! Panel 端 Core 注册、持久化和自动重连服务。
//!
//! 注册信息保存加密后的预共享秘密，内存中的连接由后台监视器维护；所有领域请求
//! 都先取得对应 Core 的连接锁，连接不可用时返回明确错误而不会伪造成功结果。

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use nexus_config::LocalCoreConfig;
use nexus_domain::CoreId;
use nexus_domain::CpuPolicy;
use nexus_domain::CpuTopology;
use nexus_domain::ExtensionInstall;
use nexus_domain::ExtensionKind;
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
use nexus_storage::NewExtensionInstall;
use nexus_storage::SqliteStore;
use nexus_storage::StoredCore;
use nexus_storage::StoredExtensionInstall;
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

/// 管理已注册 Core、连接状态和跨请求重连任务的服务。
#[derive(Clone)]
pub struct CoreRegistry {
    store: SqliteStore,
    cipher: SecretCipher,
    panel_id: String,
    entries: Arc<RwLock<HashMap<CoreId, Arc<ManagedCore>>>>,
    shutdown: Sender<()>,
}

impl CoreRegistry {
    /// 从 SQLite 注册记录恢复 Core 并启动连接监视器。
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

    /// 校验请求、建立连接并持久化新的 Core 注册。
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

    /// 确保配置指定的本地 Core 已注册且身份匹配。
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

    /// 列出所有已注册 Core 及其运行时状态。
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

    /// 获取指定 Core 的注册和运行时状态。
    pub async fn get(&self, core_id: CoreId) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        core_json(&core).await
    }

    /// 建立一次性连接并返回延迟与协议版本。
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

    /// 查询指定 Core 的 CPU 拓扑快照。
    pub async fn cpu_topology(&self, core_id: CoreId) -> Result<CpuTopology, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection.cpu_topology().await?)
    }

    /// 通过指定 Core 预览 CPU policy 的候选和建议集合。
    pub async fn resolve_cpu_policy(
        &self,
        core_id: CoreId,
        policy: &CpuPolicy,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection.resolve_cpu_policy(policy).await?)
    }

    /// 丢弃当前连接并请求后台监视器立即重连。
    pub async fn reconnect(&self, core_id: CoreId) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        core.connection.lock().await.take();
        core.runtime.write().await.mark_reconnecting();
        core.reconnect.notify_waiters();

        core_json(&core).await
    }

    /// 通过已注册 Core 创建实例并持久化幂等请求结果。
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

    /// 获取指定 Core 上的实例。
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

    /// 列出指定 Core 的受管运行时。
    pub async fn list_managed_runtimes(&self, core_id: CoreId) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let runtimes: Vec<ManagedRuntime> = connection.list_managed_runtimes().await?;

        Ok(json!({ "items": runtimes }))
    }

    /// 请求指定 Core 安装受管运行时。
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

    /// 请求指定 Core 验证受管运行时。
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

    /// 查询指定 Core 的运行时任务。
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

    /// 请求指定 Core 删除受管运行时。
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

    /// 解析指定 Core 的一键搭建计划。
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

    /// 按已确认哈希在指定 Core 执行一键搭建。
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

    /// 查询指定 Core 的搭建任务。
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

    /// 获取指定 Core 上实例的基岩管理画像。
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

    /// 检查指定 Core 上基岩 UDP 端口。
    pub async fn check_bedrock_port(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let check = connection.check_bedrock_port(instance_id).await?;

        Ok(json!(check))
    }

    /// 检查指定 Core 上基岩 RakNet 健康状态。
    pub async fn check_bedrock_health(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let health = connection.check_bedrock_health(instance_id).await?;

        Ok(json!(health))
    }

    /// 检查指定 Core 上代理后端健康状态。
    pub async fn check_proxy_subserver(
        &self,
        core_id: CoreId,
        proxy_instance_id: &InstanceId,
        subserver_id: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;
        let health = connection
            .check_proxy_subserver(proxy_instance_id, subserver_id)
            .await?;

        Ok(json!(health))
    }

    /// 启动指定 Core 上的代理及可选后端。
    pub async fn start_proxy(
        &self,
        core_id: CoreId,
        proxy_instance_id: &InstanceId,
        include_backends: bool,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .start_proxy(proxy_instance_id, include_backends, idempotency_key)
            .await?)
    }

    /// 停止指定 Core 上的代理及可选后端。
    pub async fn stop_proxy(
        &self,
        core_id: CoreId,
        proxy_instance_id: &InstanceId,
        include_backends: bool,
        timeout_seconds: Option<u16>,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .stop_proxy(
                proxy_instance_id,
                include_backends,
                timeout_seconds,
                idempotency_key,
            )
            .await?)
    }

    /// 在 Panel 存储中写入或更新扩展安装记录。
    pub async fn upsert_extension_install(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        install: &ExtensionInstall,
    ) -> Result<ExtensionInstall, CoreRegistryError> {
        let new_install = NewExtensionInstall {
            id: install.id().to_owned(),
            core_id: core_id.to_string(),
            instance_id: instance_id.to_string(),
            kind: extension_kind_text(install.kind()).to_owned(),
            path: install.path().to_owned(),
            sha256: install.sha256().to_owned(),
            source: install.source().to_owned(),
            project_id: install.project_id().map(str::to_owned),
            version: install.version().map(str::to_owned),
            installed_at: install.installed_at().to_owned(),
        };
        let store = self.store.clone();
        let stored = spawn_blocking(move || store.upsert_extension_install(&new_install)).await??;

        extension_install_from_stored(stored)
    }

    /// 按扩展种类列出实例安装记录。
    pub async fn list_extension_installs(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        kind: ExtensionKind,
    ) -> Result<Vec<ExtensionInstall>, CoreRegistryError> {
        let core_id_text = core_id.to_string();
        let instance_id_text = instance_id.to_string();
        let kind_text = extension_kind_text(kind).to_owned();
        let store = self.store.clone();
        let stored = spawn_blocking(move || {
            store.list_extension_installs(&core_id_text, &instance_id_text, &kind_text)
        })
        .await??;

        stored
            .into_iter()
            .map(extension_install_from_stored)
            .collect()
    }

    /// 删除指定路径的扩展安装记录。
    pub async fn delete_extension_install(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        path: &str,
    ) -> Result<(), CoreRegistryError> {
        let core_id_text = core_id.to_string();
        let instance_id_text = instance_id.to_string();
        let path = path.to_owned();
        let store = self.store.clone();
        spawn_blocking(move || {
            store.delete_extension_install(&core_id_text, &instance_id_text, &path)
        })
        .await??;

        Ok(())
    }

    /// 列出指定 Core 代理的后端关系。
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

    /// 在指定 Core 上新增或替换代理后端关系。
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

    /// 删除指定 Core 上的代理后端关系。
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

    /// 按修订号更新指定 Core 上的实例配置。
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

    /// 按游标、数量和状态列出指定 Core 的实例。
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

    /// 启动指定 Core 上的实例。
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

    /// 优雅停止指定 Core 上的实例。
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

    /// 强制终止指定 Core 上的实例。
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

    /// 向指定 Core 上的实例发送命令。
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

    /// 读取指定 Core 上实例的控制台日志。
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

    /// 读取指定 Core 上实例的指标序列。
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

    /// 扫描指定 Core 上实例的配置文档。
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

    /// 获取指定 Core 上的配置文档。
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
    /// 按修订号修改指定 Core 上的配置文档。
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

    /// 分页列出指定 Core 上实例的文件。
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

    /// 读取指定 Core 上实例文件的内容分块。
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

    /// 写入指定 Core 上的实例文件。
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

    /// 创建指定 Core 上的实例目录。
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

    /// 移动指定 Core 上实例目录中的文件。
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

    /// 启动删除指定 Core 上实例文件的任务。
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

    /// 启动指定 Core 上的文件批处理任务。
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

    /// 启动指定 Core 上的 ZIP 归档任务。
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

    /// 查询指定 Core 上的文件任务。
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

    #[allow(clippy::too_many_arguments)]
    /// 开始指定 Core 上的文件上传。
    pub async fn begin_file_upload_with_expected(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        path: &str,
        size_bytes: u64,
        sha256: &str,
        expected_sha256: Option<&str>,
        idempotency_key: &str,
    ) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let mut connection = core.connection.lock().await;
        let connection = connection
            .as_mut()
            .ok_or(CoreRegistryError::ConnectionUnavailable)?;

        Ok(connection
            .begin_file_upload_with_expected(
                instance_id,
                path,
                size_bytes,
                sha256,
                expected_sha256,
                idempotency_key,
            )
            .await?)
    }

    /// 开始指定 Core 上的文件下载。
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

    /// 上传指定 Core 文件传输的一块内容。
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

    /// 读取指定 Core 文件下载传输的一块内容。
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

    /// 提交指定 Core 上的文件上传。
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

    /// 放弃指定 Core 上的文件上传。
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

    /// 提交指定 Core 上的文件下载。
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

    /// 放弃指定 Core 上的文件下载。
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

fn extension_kind_text(kind: ExtensionKind) -> &'static str {
    match kind {
        ExtensionKind::Plugin => "PLUGIN",
        ExtensionKind::Mod => "MOD",
    }
}

fn extension_install_from_stored(
    stored: StoredExtensionInstall,
) -> Result<ExtensionInstall, CoreRegistryError> {
    let kind = match stored.kind() {
        "PLUGIN" => ExtensionKind::Plugin,
        "MOD" => ExtensionKind::Mod,
        _ => {
            return Err(CoreRegistryError::InvalidStoredExtension {
                path: stored.path().to_owned(),
            });
        }
    };

    Ok(ExtensionInstall::new(
        stored.id().to_owned(),
        kind,
        stored.path().to_owned(),
        stored.sha256().to_owned(),
        stored.source().to_owned(),
        stored.project_id().map(str::to_owned),
        stored.version().map(str::to_owned),
        stored.installed_at().to_owned(),
    ))
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
