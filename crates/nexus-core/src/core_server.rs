//! Core TCP/TLS 服务和协议请求分发入口。
//!
//! 连接依次经过 TLS、Noise PSK 和会话问候校验；通过后才可访问实例、文件、运行时、
//! 代理后端和基岩健康检查等管理器。

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use nexus_config::CoreConfig;
use nexus_domain::BedrockBindAddressSource;
use nexus_domain::BedrockHealth;
use nexus_domain::BedrockHealthStatus;
use nexus_domain::BedrockManagementKind;
use nexus_domain::BedrockManagementProfile;
use nexus_domain::BedrockPortCheck;
use nexus_domain::BedrockPortCheckState;
use nexus_domain::BedrockPortSource;
use nexus_domain::CoreId;
use nexus_domain::Instance;
use nexus_domain::InstanceCreate;
use nexus_domain::InstanceId;
use nexus_domain::InstancePage;
use nexus_domain::InstanceState;
use nexus_domain::InstanceUpdate;
use nexus_domain::PRODUCT_VERSION;
use nexus_domain::ProvisionPlan;
use nexus_domain::ProxySubserver;
use nexus_domain::ProxySubserverHealth;
use nexus_domain::ProxySubserverHealthStatus;
use nexus_domain::ProxySubserverProtocolStatus;
use nexus_domain::RequestId;
use nexus_domain::RuntimeInstallManifest;
use nexus_domain::TaskId;
use nexus_protocol::CURRENT_PROTOCOL_VERSION;
use nexus_protocol::NoiseTransport;
use nexus_protocol::PresharedKey;
use nexus_protocol::ProtocolVersion;
use nexus_protocol::WireError;
use nexus_protocol::WireMessage;
use serde_json::Value;
use serde_json::from_value;
use serde_json::json;
use serde_yaml::Value as YamlValue;
use serde_yaml::from_slice as yaml_from_slice;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::spawn;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;

use crate::CoreError;
use crate::CoreRequestState;
use crate::CoreTlsIdentity;
use crate::FileBatchOperation;
use crate::FileManager;
use crate::FileManagerError;
use crate::InstanceProcessError;
use crate::InstanceProcessManager;
use crate::InstanceRepository;
use crate::InstanceRepositoryError;
use crate::ProvisionManager;
use crate::ProvisionManagerError;
use crate::ProxySubserverRepository;
use crate::ProxySubserverRepositoryError;
use crate::RuntimeManager;
use crate::RuntimeManagerError;
use crate::file_manager::FILE_TRANSFER_CHUNK_BYTES;
use crate::file_manager::MAXIMUM_FILE_ARCHIVE_PATHS;
use crate::file_manager::MAXIMUM_FILE_BATCH_OPERATIONS;
use crate::file_manager::MAXIMUM_FILE_READ_BYTES;

const CORE_CAPABILITIES: [&str; 12] = [
    "bedrock-health",
    "config",
    "events",
    "files",
    "instances",
    "metrics",
    "proxy-orchestration",
    "proxy-subservers",
    "provision",
    "runtimes",
    "settings",
    "transfer-v1",
];
const CORE_ID_FILE_NAME: &str = "core-id";
const EVENT_TOPICS: [&str; 2] = ["instance.console", "instance.state"];
const HEARTBEAT_SECONDS: u64 = 20;
const INSTANCE_LIST_DEFAULT_LIMIT: usize = 50;
const INSTANCE_LIST_MAXIMUM_LIMIT: usize = 200;
const INSTANCE_LOG_DEFAULT_LIMIT: usize = 50;
const INSTANCE_LOG_MAXIMUM_LIMIT: usize = 200;
const PROXY_SUBSERVER_HEALTH_TIMEOUT: Duration = Duration::from_secs(3);
const BEDROCK_HEALTH_TIMEOUT: Duration = Duration::from_secs(3);
const MINECRAFT_STATUS_PROTOCOL_VERSION: i32 = 767;
const MAXIMUM_MINECRAFT_STATUS_PACKET_BYTES: usize = 2 * 1024 * 1024;
const MAXIMUM_BEDROCK_STATUS_PACKET_BYTES: usize = 4096;
const RAKNET_UNCONNECTED_PING_ID: u8 = 0x01;
const RAKNET_UNCONNECTED_PONG_ID: u8 = 0x1c;
const RAKNET_MAGIC: [u8; 16] = [
    0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];

/// 绑定到一个 TCP 地址并提供 Core 管理协议的服务器。
///
/// 服务器持有本地各领域管理器，`serve` 会为每个已接受连接创建独立请求状态，
/// 但管理器数据仍由内部共享句柄统一协调。
pub struct CoreServer {
    core_id: CoreId,
    certificate_sha256: Arc<str>,
    listen_address: SocketAddr,
    listener: TcpListener,
    pre_shared_key: PresharedKey,
    instances: InstanceRepository,
    processes: InstanceProcessManager,
    proxy_subservers: ProxySubserverRepository,
    provision: ProvisionManager,
    runtimes: RuntimeManager,
    files: FileManager,
    tls_acceptor: TlsAcceptor,
}

#[derive(Clone)]
struct CoreResources {
    instances: InstanceRepository,
    processes: InstanceProcessManager,
    proxy_subservers: ProxySubserverRepository,
    provision: ProvisionManager,
    runtimes: RuntimeManager,
    files: FileManager,
}

impl CoreServer {
    /// 根据 Core 配置创建监听器、TLS 身份和领域管理器。
    ///
    /// 该操作会创建数据目录并加载或生成稳定 Core 标识；缺少预共享密钥时拒绝启动。
    pub async fn bind(config: &CoreConfig) -> Result<Self, CoreError> {
        let pre_shared_key = config
            .pre_shared_key()
            .ok_or(CoreError::MissingPreSharedKey)?
            .clone();
        let tls_identity = CoreTlsIdentity::load_or_create(config)?;
        let core_id = load_or_create_core_id(config.data_directory())?;
        let listener = TcpListener::bind(config.listen_address())
            .await
            .map_err(|source| CoreError::Bind {
                address: config.listen_address(),
                source,
            })?;
        let listen_address = listener.local_addr().map_err(|source| CoreError::Bind {
            address: config.listen_address(),
            source,
        })?;

        let instances = InstanceRepository::new();
        let processes =
            InstanceProcessManager::new(config.data_directory().to_path_buf(), instances.clone());
        let runtimes =
            RuntimeManager::new(config.data_directory()).map_err(CoreError::RuntimeManager)?;
        let provision = ProvisionManager::new(config.data_directory(), runtimes.clone())?;
        let files = FileManager::new(config.data_directory());

        Ok(Self {
            core_id,
            certificate_sha256: Arc::from(tls_identity.certificate_sha256()),
            listen_address,
            listener,
            pre_shared_key,
            instances,
            processes,
            proxy_subservers: ProxySubserverRepository::new(),
            provision,
            runtimes,
            files,
            tls_acceptor: tls_identity.acceptor(),
        })
    }

    /// 返回稳定 Core 标识。
    #[must_use]
    pub const fn core_id(&self) -> CoreId {
        self.core_id
    }

    /// 返回当前 TLS 证书的 SHA-256 指纹。
    #[must_use]
    pub fn certificate_sha256(&self) -> &str {
        &self.certificate_sha256
    }

    /// 返回实际绑定的 TCP 地址。
    #[must_use]
    pub const fn listen_address(&self) -> SocketAddr {
        self.listen_address
    }

    /// 返回实例仓库的共享句柄，供本地嵌入式调用使用。
    #[must_use]
    pub fn instance_repository(&self) -> InstanceRepository {
        self.instances.clone()
    }

    /// 接受连接并持续服务，直到监听器或协议处理返回错误。
    pub async fn serve(self) -> Result<(), CoreError> {
        tracing::info!(
            core_id = %self.core_id,
            listen_address = %self.listen_address,
            "Core TCP listener is ready"
        );

        loop {
            let (stream, peer_address) = self.listener.accept().await.map_err(CoreError::Accept)?;
            let core_id = self.core_id;
            let certificate_sha256 = self.certificate_sha256.clone();
            let pre_shared_key = self.pre_shared_key.clone();
            let resources = CoreResources {
                instances: self.instances.clone(),
                processes: self.processes.clone(),
                proxy_subservers: self.proxy_subservers.clone(),
                provision: self.provision.clone(),
                runtimes: self.runtimes.clone(),
                files: self.files.clone(),
            };
            let tls_acceptor = self.tls_acceptor.clone();

            spawn(async move {
                let stream = match tls_acceptor.accept(stream).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::debug!(%peer_address, %error, "Core TLS handshake failed");
                        return;
                    }
                };
                if handle_connection(
                    stream,
                    &pre_shared_key,
                    core_id,
                    &certificate_sha256,
                    resources,
                )
                .await
                .is_err()
                {
                    tracing::debug!(%peer_address, "Core TCP connection closed during protocol handling");
                }
            });
        }
    }
}

/// 按配置绑定并运行 Core 服务。
pub async fn run(config: &CoreConfig) -> Result<(), CoreError> {
    CoreServer::bind(config).await?.serve().await
}

async fn handle_connection<S>(
    stream: S,
    pre_shared_key: &PresharedKey,
    core_id: CoreId,
    certificate_sha256: &str,
    resources: CoreResources,
) -> Result<(), CoreError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut transport = NoiseTransport::accept(stream, pre_shared_key).await?;
    let first_message = transport.read_message().await?;
    let Some((request_id, method, params, _)) = request_parts(first_message) else {
        return Ok(());
    };
    let (response, session_established) =
        session_hello_response(request_id, &method, &params, core_id, certificate_sha256);

    transport.write_message(&response).await?;
    if !session_established {
        return Ok(());
    }

    let mut request_state = CoreRequestState::new(
        core_id,
        resources.instances,
        resources.processes,
        resources.proxy_subservers,
        resources.provision,
        resources.runtimes,
        resources.files,
    );
    let mut event_receiver = request_state.processes().subscribe();

    loop {
        select! {
            message = transport.read_message() => {
                let Some((request_id, method, params, idempotency_key)) = request_parts(message?) else {
                    return Ok(());
                };
                let response = request_response(
                    request_id,
                    &method,
                    &params,
                    idempotency_key.as_deref(),
                    &mut request_state,
                ).await;

                transport.write_message(&response).await?;
            }
            event = event_receiver.recv(), if request_state.is_subscribed_to_events() => {
                match event {
                    Ok(event) => {
                        if event_topic(&event)
                            .is_some_and(|topic| request_state.is_subscribed_to_topic(topic))
                        {
                            transport.write_message(&event).await?;
                        }
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        tracing::debug!(skipped, "Core event subscriber lagged behind");
                    }
                    Err(RecvError::Closed) => return Ok(()),
                }
            }
        }
    }
}

fn load_or_create_core_id(data_directory: &Path) -> Result<CoreId, CoreError> {
    fs::create_dir_all(data_directory).map_err(|source| CoreError::CreateDataDirectory {
        path: data_directory.to_path_buf(),
        source,
    })?;

    let identity_path = data_directory.join(CORE_ID_FILE_NAME);
    if identity_path.exists() {
        let content =
            fs::read_to_string(&identity_path).map_err(|source| CoreError::ReadCoreIdentity {
                path: identity_path.clone(),
                source,
            })?;

        return content
            .trim()
            .parse()
            .map_err(|_| CoreError::InvalidStoredCoreId {
                path: identity_path,
            });
    }

    let core_id = CoreId::new();
    fs::write(&identity_path, core_id.to_string()).map_err(|source| {
        CoreError::WriteCoreIdentity {
            path: identity_path,
            source,
        }
    })?;

    Ok(core_id)
}

fn request_parts(message: WireMessage) -> Option<(RequestId, String, Value, Option<String>)> {
    match message {
        WireMessage::Request {
            request_id,
            method,
            params,
            idempotency_key,
            ..
        } => Some((request_id, method, params, idempotency_key)),
        WireMessage::Response { .. } | WireMessage::Event { .. } => None,
    }
}

fn session_hello_response(
    request_id: RequestId,
    method: &str,
    params: &Value,
    core_id: CoreId,
    certificate_sha256: &str,
) -> (WireMessage, bool) {
    if method != "session.hello" {
        return (
            error_response(
                request_id,
                "SESSION_HELLO_REQUIRED",
                "session.hello must be the first request",
            ),
            false,
        );
    }

    let Some(remote_protocol) = protocol_from_hello(params) else {
        return (
            error_response(
                request_id,
                "INVALID_SESSION_HELLO",
                "session.hello must include a valid protocol version",
            ),
            false,
        );
    };
    let Ok(protocol) = CURRENT_PROTOCOL_VERSION.negotiate(remote_protocol) else {
        return (
            error_response(
                request_id,
                "PROTOCOL_VERSION_UNSUPPORTED",
                "Core and Panel protocol major versions are incompatible",
            ),
            false,
        );
    };

    (
        success_response(
            request_id,
            json!({
                "protocol": protocol,
                "coreId": core_id,
                "coreName": "MCNP Core",
                "serverVersion": PRODUCT_VERSION,
                "tlsCertificateSha256": certificate_sha256,
                "capabilities": negotiated_capabilities(params),
                "sessionId": RequestId::new(),
                "heartbeatSeconds": HEARTBEAT_SECONDS,
            }),
        ),
        true,
    )
}

async fn request_response(
    request_id: RequestId,
    method: &str,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &mut CoreRequestState,
) -> WireMessage {
    match method {
        "system.info" => success_response(
            request_id,
            json!({
                "coreId": state.core_id(),
                "serverVersion": PRODUCT_VERSION,
                "protocol": CURRENT_PROTOCOL_VERSION,
                "capabilities": CORE_CAPABILITIES,
            }),
        ),
        "system.ping" => success_response(request_id, json!({ "receivedAt": current_timestamp() })),
        "runtime.list" => environment_list_response(request_id, state.runtimes()).await,
        "runtime.install" => {
            runtime_install_response(request_id, params, idempotency_key, state.runtimes())
        }
        "runtime.verify" => {
            runtime_verify_response(request_id, params, idempotency_key, state.runtimes())
        }
        "runtime.delete" => runtime_delete_response(request_id, params, idempotency_key, state),
        "runtime.task.get" => runtime_task_response(request_id, params, state.runtimes()),
        "provision.resolve" => provision_resolve_response(request_id, params, state.provision()),
        "provision.execute" => {
            provision_execute_response(request_id, params, idempotency_key, state)
        }
        "provision.task.get" => provision_task_response(request_id, params, state.provision()),
        "bedrock.profile" => bedrock_profile_response(request_id, params, state.instances()),
        "bedrock.port.check" => {
            bedrock_port_check_response(request_id, params, state.instances(), state.files()).await
        }
        "bedrock.health.check" => {
            bedrock_health_response(request_id, params, state.instances(), state.files()).await
        }
        "proxy.subserver.list" => proxy_subserver_list_response(
            request_id,
            params,
            state.instances(),
            state.proxy_subservers(),
        ),
        "proxy.subserver.upsert" => proxy_subserver_upsert_response(
            request_id,
            params,
            idempotency_key,
            state.instances(),
            state.proxy_subservers(),
        ),
        "proxy.subserver.delete" => proxy_subserver_delete_response(
            request_id,
            params,
            idempotency_key,
            state.instances(),
            state.proxy_subservers(),
        ),
        "proxy.subserver.check" => {
            proxy_subserver_check_response(
                request_id,
                params,
                state.instances(),
                state.proxy_subservers(),
            )
            .await
        }
        "proxy.start" => {
            proxy_orchestration_response(request_id, params, idempotency_key, state, true).await
        }
        "proxy.stop" => {
            proxy_orchestration_response(request_id, params, idempotency_key, state, false).await
        }
        "instance.command" => {
            instance_command_response(request_id, params, state.processes()).await
        }
        "instance.create" => instance_create_response(request_id, params, state.instances()),
        "instance.get" => instance_get_response(request_id, params, state.instances()),
        "instance.kill" => {
            instance_kill_response(request_id, params, idempotency_key, state.processes()).await
        }
        "instance.list" => instance_list_response(request_id, params, state.instances()),
        "instance.logs" => instance_logs_response(request_id, params, state.processes()),
        "instance.metrics" => instance_metrics_response(request_id, params, state.processes()),
        "config.scan" => config_scan_response(request_id, params, state),
        "config.get" => config_get_response(request_id, params, state),
        "config.patch" => config_patch_response(request_id, params, idempotency_key, state),
        "file.list" => file_list_response(request_id, params, state),
        "file.read" => file_read_response(request_id, params, state),
        "file.mkdir" => file_mkdir_response(request_id, params, idempotency_key, state),
        "file.move" => file_move_response(request_id, params, idempotency_key, state),
        "file.delete" => file_delete_response(request_id, params, idempotency_key, state),
        "file.batch" => file_batch_response(request_id, params, idempotency_key, state),
        "file.archive.create" => {
            file_archive_create_response(request_id, params, idempotency_key, state)
        }
        "file.task.get" => file_task_response(request_id, params, state),
        "file.write" => file_write_response(request_id, params, idempotency_key, state),
        "transfer.begin" => transfer_begin_response(request_id, params, idempotency_key, state),
        "transfer.chunk" => transfer_chunk_response(request_id, params, idempotency_key, state),
        "transfer.commit" => transfer_commit_response(request_id, params, idempotency_key, state),
        "transfer.abort" => transfer_abort_response(request_id, params, idempotency_key, state),
        "instance.start" => {
            instance_start_response(request_id, params, idempotency_key, state.processes()).await
        }
        "instance.stop" => {
            instance_stop_response(request_id, params, idempotency_key, state.processes()).await
        }
        "instance.update" => instance_update_response(request_id, params, state.instances()),
        "event.subscribe" => event_subscribe_response(request_id, params, state),
        "event.unsubscribe" => event_unsubscribe_response(request_id, params, state),
        _ => error_response(
            request_id,
            "METHOD_NOT_SUPPORTED",
            "The requested Core method is not supported",
        ),
    }
}

async fn environment_list_response(
    request_id: RequestId,
    runtimes: &RuntimeManager,
) -> WireMessage {
    success_response(request_id, json!({ "items": runtimes.discover().await }))
}

fn runtime_install_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    runtimes: &RuntimeManager,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(manifest) = params
        .get("manifest")
        .cloned()
        .and_then(|value| from_value::<RuntimeInstallManifest>(value).ok())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "runtime.install requires a valid manifest",
        );
    };
    let set_as_default = params
        .get("setAsDefault")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    match runtimes.start_install(&manifest, set_as_default) {
        Ok(task_id) => task_accepted_response(request_id, task_id),
        Err(error) => runtime_manager_error_response(request_id, error),
    }
}

fn runtime_verify_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    runtimes: &RuntimeManager,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(runtime_id) = params
        .get("runtimeId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "runtime.verify requires a runtimeId",
        );
    };
    match runtimes.start_verify(runtime_id) {
        Ok(task_id) => task_accepted_response(request_id, task_id),
        Err(error) => runtime_manager_error_response(request_id, error),
    }
}

fn runtime_delete_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(runtime_id) = params
        .get("runtimeId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "runtime.delete requires a runtimeId",
        );
    };
    match state.runtimes().start_delete(runtime_id, state.instances()) {
        Ok(task_id) => task_accepted_response(request_id, task_id),
        Err(error) => runtime_manager_error_response(request_id, error),
    }
}

fn runtime_task_response(
    request_id: RequestId,
    params: &Value,
    runtimes: &RuntimeManager,
) -> WireMessage {
    let Some(task_id) = params
        .get("taskId")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<TaskId>().ok())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "runtime.task.get requires a valid taskId",
        );
    };
    match runtimes.task(task_id) {
        Ok(Some(task)) => success_response(request_id, task),
        Ok(None) => error_response(request_id, "TASK_NOT_FOUND", "Runtime task does not exist"),
        Err(error) => runtime_manager_error_response(request_id, error),
    }
}

fn runtime_manager_error_response(
    request_id: RequestId,
    error: RuntimeManagerError,
) -> WireMessage {
    let (code, message) = match error {
        RuntimeManagerError::AlreadyExists { .. } => {
            ("RUNTIME_ALREADY_EXISTS", "The runtime is already installed")
        }
        RuntimeManagerError::InUse { .. } => {
            ("RUNTIME_IN_USE", "The runtime is referenced by an instance")
        }
        RuntimeManagerError::NotFound { .. } => ("RUNTIME_NOT_FOUND", "The runtime does not exist"),
        RuntimeManagerError::InvalidRuntimeId
        | RuntimeManagerError::InvalidManifest { .. }
        | RuntimeManagerError::UnsafeArchiveEntry { .. } => {
            ("BAD_REQUEST", "The runtime manifest or archive is invalid")
        }
        _ => {
            tracing::error!(%error, "Runtime management operation failed");
            (
                "RUNTIME_OPERATION_FAILED",
                "Runtime management operation failed",
            )
        }
    };
    error_response(request_id, code, message)
}

fn provision_resolve_response(
    request_id: RequestId,
    params: &Value,
    provision: &ProvisionManager,
) -> WireMessage {
    let Some(plan) = from_value::<ProvisionPlan>(params.clone()).ok() else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "provision.resolve requires a valid plan",
        );
    };
    match provision.resolve(&plan) {
        Ok(result) => success_response(request_id, result),
        Err(error) => provision_manager_error_response(request_id, error),
    }
}

fn provision_execute_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(plan) = params
        .get("resolvedPlan")
        .cloned()
        .and_then(|value| from_value::<ProvisionPlan>(value).ok())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "provision.execute requires a valid resolvedPlan",
        );
    };
    let Some(plan_hash) = params.get("planHash").and_then(Value::as_str) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "provision.execute requires a planHash",
        );
    };
    match state
        .provision()
        .start_execute(&plan, plan_hash, state.instances())
    {
        Ok(task_id) => success_response(
            request_id,
            json!({
                "taskId": task_id,
                "instanceId": plan.instance_id(),
                "acceptedAt": current_timestamp(),
            }),
        ),
        Err(error) => provision_manager_error_response(request_id, error),
    }
}

fn provision_task_response(
    request_id: RequestId,
    params: &Value,
    provision: &ProvisionManager,
) -> WireMessage {
    let Some(task_id) = params
        .get("taskId")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<TaskId>().ok())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "provision.task.get requires a valid taskId",
        );
    };
    match provision.task(task_id) {
        Ok(Some(task)) => success_response(request_id, task),
        Ok(None) => error_response(
            request_id,
            "PROVISION_TASK_NOT_FOUND",
            "Provision task does not exist",
        ),
        Err(error) => provision_manager_error_response(request_id, error),
    }
}

fn provision_manager_error_response(
    request_id: RequestId,
    error: ProvisionManagerError,
) -> WireMessage {
    match error {
        ProvisionManagerError::InvalidPlan { .. }
        | ProvisionManagerError::Serialization(_)
        | ProvisionManagerError::Instance(_) => {
            error_response(request_id, "BAD_REQUEST", "Provision plan is invalid")
        }
        ProvisionManagerError::PlanHashMismatch => error_response(
            request_id,
            "PROVISION_PLAN_EXPIRED",
            "Provision plan hash does not match",
        ),
        ProvisionManagerError::AlreadyExists { .. } => error_response(
            request_id,
            "INSTANCE_ALREADY_EXISTS",
            "Instance already exists",
        ),
        ProvisionManagerError::Repository(crate::InstanceRepositoryError::AlreadyExists {
            ..
        }) => error_response(
            request_id,
            "INSTANCE_ALREADY_EXISTS",
            "Instance already exists",
        ),
        ProvisionManagerError::Repository(crate::InstanceRepositoryError::NotFound { .. }) => {
            error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist")
        }
        ProvisionManagerError::Runtime(RuntimeManagerError::NotFound { .. }) => {
            error_response(request_id, "RUNTIME_NOT_FOUND", "Runtime does not exist")
        }
        ProvisionManagerError::Runtime(RuntimeManagerError::InvalidManifest { .. }) => {
            error_response(request_id, "RUNTIME_INVALID", "Selected runtime is invalid")
        }
        ProvisionManagerError::TaskStorePoisoned
        | ProvisionManagerError::Archive { .. }
        | ProvisionManagerError::Download(_)
        | ProvisionManagerError::Repository(_)
        | ProvisionManagerError::Runtime(_)
        | ProvisionManagerError::Storage { .. } => {
            tracing::error!(%error, "Provision operation failed");
            error_response(request_id, "PROVISION_FAILED", "Provision operation failed")
        }
    }
}

fn bedrock_profile_response(
    request_id: RequestId,
    params: &Value,
    instances: &InstanceRepository,
) -> WireMessage {
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "bedrock.profile requires a valid instanceId",
        );
    };
    let instance = match instances.get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };
    let Some(profile) = instance.kind().bedrock_management_profile() else {
        return error_response(
            request_id,
            "BEDROCK_PROFILE_UNSUPPORTED",
            "The instance does not expose a Bedrock operations profile",
        );
    };

    success_response(request_id, json!(profile))
}

async fn bedrock_port_check_response(
    request_id: RequestId,
    params: &Value,
    instances: &InstanceRepository,
    files: &FileManager,
) -> WireMessage {
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "bedrock.port.check requires a valid instanceId",
        );
    };
    let instance = match instances.get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };
    let Some(profile) = instance.kind().bedrock_management_profile() else {
        return error_response(
            request_id,
            "BEDROCK_PROFILE_UNSUPPORTED",
            "The instance does not expose a Bedrock operations profile",
        );
    };

    let (bind_address, bind_address_source, port, port_source, configuration_error) =
        configured_bedrock_endpoint(&instance, &profile, files);

    let (state, available, bind_error) = match UdpSocket::bind((bind_address, port)).await {
        Ok(_socket) => (BedrockPortCheckState::Available, true, None),
        Err(error) if error.kind() == ErrorKind::AddrInUse => {
            (BedrockPortCheckState::InUse, false, None)
        }
        Err(_) => (
            BedrockPortCheckState::Unavailable,
            false,
            Some("BIND_FAILED".to_owned()),
        ),
    };
    let error = bind_error.or(configuration_error.map(str::to_owned));

    success_response(
        request_id,
        json!(BedrockPortCheck::new(
            instance_id,
            profile.management_kind(),
            profile.transport(),
            bind_address.to_string(),
            bind_address_source,
            port,
            port_source,
            state,
            available,
            current_timestamp(),
            error,
        )),
    )
}

async fn bedrock_health_response(
    request_id: RequestId,
    params: &Value,
    instances: &InstanceRepository,
    files: &FileManager,
) -> WireMessage {
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "bedrock.health.check requires a valid instanceId",
        );
    };
    let instance = match instances.get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };
    let Some(profile) = instance.kind().bedrock_management_profile() else {
        return error_response(
            request_id,
            "BEDROCK_PROFILE_UNSUPPORTED",
            "The instance does not expose a Bedrock operations profile",
        );
    };

    let (bind_address, bind_address_source, port, port_source, configuration_error) =
        configured_bedrock_endpoint(&instance, &profile, files);
    let probe_address = bedrock_probe_address(bind_address);
    let started_at = Instant::now();
    let result = timeout(
        BEDROCK_HEALTH_TIMEOUT,
        bedrock_raknet_probe(probe_address, port),
    )
    .await;
    let latency_ms = Some(started_at.elapsed().as_millis() as u64);
    let (status, reachable, server_identity, probe_error) = match result {
        Ok(Ok(server_identity)) => (
            BedrockHealthStatus::Responded,
            true,
            Some(server_identity),
            None,
        ),
        Ok(Err("INVALID_RESPONSE")) => (
            BedrockHealthStatus::InvalidResponse,
            true,
            None,
            Some("INVALID_RESPONSE".to_owned()),
        ),
        Ok(Err(_)) => (
            BedrockHealthStatus::Unavailable,
            false,
            None,
            Some("PROBE_FAILED".to_owned()),
        ),
        Err(_) => (
            BedrockHealthStatus::Unreachable,
            false,
            None,
            Some("TIMEOUT".to_owned()),
        ),
    };
    let error = probe_error.or(configuration_error.map(str::to_owned));

    success_response(
        request_id,
        json!(BedrockHealth::new(
            instance_id,
            profile.management_kind(),
            profile.transport(),
            bind_address.to_string(),
            bind_address_source,
            port,
            port_source,
            probe_address.to_string(),
            status,
            reachable,
            latency_ms,
            server_identity,
            current_timestamp(),
            error,
        )),
    )
}

async fn bedrock_raknet_probe(probe_address: IpAddr, port: u16) -> Result<String, &'static str> {
    let local_address = match probe_address {
        IpAddr::V4(_) => SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
        IpAddr::V6(_) => SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)),
    };
    let socket = UdpSocket::bind(local_address)
        .await
        .map_err(|_| "PROBE_SOCKET_FAILED")?;
    socket
        .connect(SocketAddr::new(probe_address, port))
        .await
        .map_err(|_| "PROBE_CONNECT_FAILED")?;

    let mut ping = Vec::with_capacity(33);
    ping.push(RAKNET_UNCONNECTED_PING_ID);
    ping.extend_from_slice(&0_u64.to_be_bytes());
    ping.extend_from_slice(&RAKNET_MAGIC);
    ping.extend_from_slice(&0_u64.to_be_bytes());
    socket.send(&ping).await.map_err(|_| "PROBE_SEND_FAILED")?;

    let mut response = [0_u8; MAXIMUM_BEDROCK_STATUS_PACKET_BYTES];
    let length = socket
        .recv(&mut response)
        .await
        .map_err(|_| "PROBE_RECEIVE_FAILED")?;
    parse_raknet_unconnected_pong(&response[..length])
}

fn parse_raknet_unconnected_pong(packet: &[u8]) -> Result<String, &'static str> {
    if packet.len() < 35
        || packet[0] != RAKNET_UNCONNECTED_PONG_ID
        || packet[17..33] != RAKNET_MAGIC
    {
        return Err("INVALID_RESPONSE");
    }
    let identity_length = usize::from(u16::from_be_bytes([packet[33], packet[34]]));
    let identity_end = 35_usize
        .checked_add(identity_length)
        .ok_or("INVALID_RESPONSE")?;
    let identity = packet.get(35..identity_end).ok_or("INVALID_RESPONSE")?;
    String::from_utf8(identity.to_owned()).map_err(|_| "INVALID_RESPONSE")
}

fn bedrock_probe_address(bind_address: IpAddr) -> IpAddr {
    match bind_address {
        IpAddr::V4(address) if address.is_unspecified() => Ipv4Addr::LOCALHOST.into(),
        IpAddr::V6(address) if address.is_unspecified() => Ipv6Addr::LOCALHOST.into(),
        address => address,
    }
}

fn configured_bedrock_endpoint(
    instance: &Instance,
    profile: &BedrockManagementProfile,
    files: &FileManager,
) -> (
    IpAddr,
    BedrockBindAddressSource,
    u16,
    BedrockPortSource,
    Option<&'static str>,
) {
    let default_address = profile
        .default_bind_address()
        .parse::<IpAddr>()
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    let default_port = profile.default_port();
    let Some(configuration_path) = profile.configuration_files().first() else {
        return (
            default_address,
            BedrockBindAddressSource::Default,
            default_port,
            BedrockPortSource::Default,
            None,
        );
    };
    let content = match files.read(instance, configuration_path, 0, MAXIMUM_FILE_READ_BYTES) {
        Ok(content) => content,
        Err(_) => {
            return (
                default_address,
                BedrockBindAddressSource::Default,
                default_port,
                BedrockPortSource::Default,
                Some("CONFIGURATION_UNAVAILABLE"),
            );
        }
    };
    let bytes = match STANDARD.decode(content.data_base64()) {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                default_address,
                BedrockBindAddressSource::Default,
                default_port,
                BedrockPortSource::Default,
                Some("CONFIGURATION_INVALID"),
            );
        }
    };
    let address = match profile.management_kind() {
        BedrockManagementKind::Geyser => parse_geyser_address(&bytes),
        BedrockManagementKind::DedicatedServer
        | BedrockManagementKind::PocketMine
        | BedrockManagementKind::Nukkit => parse_server_properties_address(&bytes),
    };
    let port = match profile.management_kind() {
        BedrockManagementKind::Geyser => parse_geyser_port(&bytes),
        BedrockManagementKind::DedicatedServer
        | BedrockManagementKind::PocketMine
        | BedrockManagementKind::Nukkit => parse_server_properties_port(&bytes),
    };
    let (address, address_source, address_error) = match address {
        Ok(Some(address)) => (address, BedrockBindAddressSource::Configured, None),
        Ok(None) => (default_address, BedrockBindAddressSource::Default, None),
        Err(error) => (
            default_address,
            BedrockBindAddressSource::Default,
            Some(error),
        ),
    };
    let (port, port_source, port_error) = match port {
        Ok(port) => (port, BedrockPortSource::Configured, None),
        Err(error) => (default_port, BedrockPortSource::Default, Some(error)),
    };

    (
        address,
        address_source,
        port,
        port_source,
        address_error.or(port_error),
    )
}

fn parse_server_properties_port(content: &[u8]) -> Result<u16, &'static str> {
    let value = parse_server_properties_value(content, "server-port")?
        .ok_or("CONFIGURATION_PORT_MISSING")?;
    let port = value
        .parse::<u16>()
        .map_err(|_| "CONFIGURATION_PORT_INVALID")?;
    (port > 0)
        .then_some(port)
        .ok_or("CONFIGURATION_PORT_INVALID")
}

fn parse_server_properties_address(content: &[u8]) -> Result<Option<IpAddr>, &'static str> {
    let Some(value) = parse_server_properties_value(content, "server-ip")? else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }

    value
        .parse::<IpAddr>()
        .map(Some)
        .map_err(|_| "CONFIGURATION_ADDRESS_INVALID")
}

fn parse_server_properties_value(
    content: &[u8],
    target_key: &str,
) -> Result<Option<String>, &'static str> {
    let content = std::str::from_utf8(content).map_err(|_| "CONFIGURATION_INVALID")?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let Some((key, value)) = line.split_once('=').or_else(|| line.split_once(':')) else {
            continue;
        };
        if key.trim() == target_key {
            return Ok(Some(value.trim().to_owned()));
        }
    }
    Ok(None)
}

fn parse_geyser_port(content: &[u8]) -> Result<u16, &'static str> {
    let document: YamlValue = yaml_from_slice(content).map_err(|_| "CONFIGURATION_INVALID")?;
    let bedrock = yaml_field(&document, "bedrock").ok_or("CONFIGURATION_PORT_MISSING")?;
    let port = yaml_field(bedrock, "port")
        .and_then(YamlValue::as_u64)
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port > 0)
        .ok_or("CONFIGURATION_PORT_INVALID")?;
    Ok(port)
}

fn parse_geyser_address(content: &[u8]) -> Result<Option<IpAddr>, &'static str> {
    let document: YamlValue = yaml_from_slice(content).map_err(|_| "CONFIGURATION_INVALID")?;
    let Some(bedrock) = yaml_field(&document, "bedrock") else {
        return Ok(None);
    };
    let Some(value) = yaml_field(bedrock, "address") else {
        return Ok(None);
    };
    let value = value
        .as_str()
        .ok_or("CONFIGURATION_ADDRESS_INVALID")?
        .trim();
    if value.is_empty() {
        return Ok(None);
    }

    value
        .parse::<IpAddr>()
        .map(Some)
        .map_err(|_| "CONFIGURATION_ADDRESS_INVALID")
}

fn yaml_field<'a>(value: &'a YamlValue, key: &str) -> Option<&'a YamlValue> {
    value.as_mapping()?.get(YamlValue::String(key.to_owned()))
}

fn proxy_subserver_list_response(
    request_id: RequestId,
    params: &Value,
    instances: &InstanceRepository,
    subservers: &ProxySubserverRepository,
) -> WireMessage {
    let proxy = match find_proxy_instance(request_id, params, instances) {
        Ok(proxy) => proxy,
        Err(response) => return *response,
    };

    match subservers.list(&proxy) {
        Ok(items) => success_response(request_id, json!({ "items": items })),
        Err(error) => proxy_subserver_repository_failure_response(request_id, &error),
    }
}

fn proxy_subserver_upsert_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    instances: &InstanceRepository,
    subservers: &ProxySubserverRepository,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let proxy = match find_proxy_instance(request_id, params, instances) {
        Ok(proxy) => proxy,
        Err(response) => return *response,
    };
    let Some(subserver) = params
        .get("subserver")
        .cloned()
        .and_then(|value| from_value::<ProxySubserver>(value).ok())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "proxy.subserver.upsert requires a valid subserver",
        );
    };
    if let Err(error) = subserver.validate() {
        return error_response(request_id, "BAD_REQUEST", error.to_string().as_str());
    }
    if subserver.target_instance_id() == proxy.id() {
        return error_response(
            request_id,
            "PROXY_TARGET_INVALID",
            "A proxy cannot target itself",
        );
    }
    match instances.get(subserver.target_instance_id()) {
        Ok(Some(target)) if target.kind().proxy_topology().allows_backend_count(1) => {
            return error_response(
                request_id,
                "PROXY_TARGET_INVALID",
                "A proxy subserver target must be a server instance",
            );
        }
        Ok(Some(_)) => {}
        Ok(None) => {
            return error_response(
                request_id,
                "PROXY_TARGET_NOT_FOUND",
                "Proxy subserver target does not exist",
            );
        }
        Err(error) => return repository_failure_response(request_id, &error),
    }

    match subservers.upsert(&proxy, subserver) {
        Ok(item) => success_response(request_id, json!(item)),
        Err(error) => proxy_subserver_repository_failure_response(request_id, &error),
    }
}

fn proxy_subserver_delete_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    instances: &InstanceRepository,
    subservers: &ProxySubserverRepository,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let proxy = match find_proxy_instance(request_id, params, instances) {
        Ok(proxy) => proxy,
        Err(response) => return *response,
    };
    let Some(subserver_id) = params
        .get("subserverId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "proxy.subserver.delete requires a subserverId",
        );
    };

    match subservers.delete(&proxy, subserver_id) {
        Ok(()) => success_response(request_id, json!({})),
        Err(error) => proxy_subserver_repository_failure_response(request_id, &error),
    }
}

async fn proxy_subserver_check_response(
    request_id: RequestId,
    params: &Value,
    instances: &InstanceRepository,
    subservers: &ProxySubserverRepository,
) -> WireMessage {
    let proxy = match find_proxy_instance(request_id, params, instances) {
        Ok(proxy) => proxy,
        Err(response) => return *response,
    };
    let Some(subserver_id) = params
        .get("subserverId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "proxy.subserver.check requires a subserverId",
        );
    };
    let subserver = match subservers.get(&proxy, subserver_id) {
        Ok(subserver) => subserver,
        Err(error) => return proxy_subserver_repository_failure_response(request_id, &error),
    };

    if !subserver.enabled() {
        return success_response(
            request_id,
            json!(ProxySubserverHealth::new(
                subserver.id().to_owned(),
                subserver.target_instance_id().clone(),
                subserver.host().to_owned(),
                subserver.port(),
                false,
                ProxySubserverHealthStatus::Disabled,
                ProxySubserverProtocolStatus::Disabled,
                None,
                None,
                current_timestamp(),
                None,
            )),
        );
    }

    let started_at = Instant::now();
    let result = timeout(
        PROXY_SUBSERVER_HEALTH_TIMEOUT,
        minecraft_status_probe(&subserver),
    )
    .await;
    let latency_ms = Some(started_at.elapsed().as_millis() as u64);
    let (status, protocol_status, reachable, error) = match result {
        Ok(Ok(())) => (
            ProxySubserverHealthStatus::Reachable,
            ProxySubserverProtocolStatus::Responded,
            Some(true),
            None,
        ),
        Ok(Err(error)) if is_network_probe_error(error) => (
            ProxySubserverHealthStatus::Unreachable,
            ProxySubserverProtocolStatus::Unavailable,
            Some(false),
            Some(error.to_owned()),
        ),
        Ok(Err(error)) => (
            ProxySubserverHealthStatus::Reachable,
            ProxySubserverProtocolStatus::InvalidResponse,
            Some(true),
            Some(error.to_owned()),
        ),
        Err(_) => (
            ProxySubserverHealthStatus::Unreachable,
            ProxySubserverProtocolStatus::Unavailable,
            Some(false),
            Some("TIMEOUT".to_owned()),
        ),
    };

    success_response(
        request_id,
        json!(ProxySubserverHealth::new(
            subserver.id().to_owned(),
            subserver.target_instance_id().clone(),
            subserver.host().to_owned(),
            subserver.port(),
            true,
            status,
            protocol_status,
            reachable,
            latency_ms,
            current_timestamp(),
            error,
        )),
    )
}

async fn proxy_orchestration_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
    start: bool,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let proxy = match find_proxy_instance(request_id, params, state.instances()) {
        Ok(proxy) => proxy,
        Err(response) => return *response,
    };
    let include_backends = match params.get("includeBackends") {
        Some(Value::Bool(value)) => *value,
        Some(_) => {
            return error_response(
                request_id,
                "BAD_REQUEST",
                "proxy orchestration includeBackends must be a boolean",
            );
        }
        None => true,
    };
    let timeout_seconds = if start {
        None
    } else {
        match optional_timeout_seconds(params) {
            Ok(timeout_seconds) => timeout_seconds,
            Err(()) => {
                return error_response(
                    request_id,
                    "BAD_REQUEST",
                    "proxy.stop timeoutSeconds must be between 1 and 300",
                );
            }
        }
    };
    let subservers = match state.proxy_subservers().list(&proxy) {
        Ok(subservers) => subservers,
        Err(error) => return proxy_subserver_repository_failure_response(request_id, &error),
    };
    let action = if start { "START" } else { "STOP" };
    let mut steps = Vec::new();
    let mut failed = false;

    if start {
        if include_backends {
            let mut target_ids = BTreeSet::new();
            for subserver in subservers.iter().filter(|subserver| subserver.enabled()) {
                if target_ids.insert(subserver.target_instance_id().clone()) {
                    let (step, step_failed) = proxy_instance_action_step(
                        subserver.target_instance_id(),
                        "BACKEND",
                        action,
                        timeout_seconds,
                        state,
                        true,
                    )
                    .await;
                    steps.push(step);
                    failed |= step_failed;
                }
            }
        }

        if failed {
            steps.push(orchestration_step(
                proxy.id(),
                "PROXY",
                action,
                "BLOCKED_BACKEND_FAILURE",
                None,
                Some("PROXY_BACKEND_FAILED"),
                None,
            ));
        } else {
            let (step, step_failed) = proxy_instance_action_step(
                proxy.id(),
                "PROXY",
                action,
                timeout_seconds,
                state,
                true,
            )
            .await;
            steps.push(step);
            failed |= step_failed;
        }
    } else {
        let (step, step_failed) =
            proxy_instance_action_step(proxy.id(), "PROXY", action, timeout_seconds, state, false)
                .await;
        steps.push(step);
        failed |= step_failed;

        if !failed && include_backends {
            let mut target_ids = BTreeSet::new();
            for subserver in subservers.iter().filter(|subserver| subserver.enabled()) {
                if target_ids.insert(subserver.target_instance_id().clone()) {
                    let (step, step_failed) = proxy_instance_action_step(
                        subserver.target_instance_id(),
                        "BACKEND",
                        action,
                        timeout_seconds,
                        state,
                        false,
                    )
                    .await;
                    steps.push(step);
                    failed |= step_failed;
                }
            }
        }
    }

    success_response(
        request_id,
        json!({
            "proxyInstanceId": proxy.id(),
            "operation": action,
            "includeBackends": include_backends,
            "state": if failed { "PARTIAL" } else { "SUCCEEDED" },
            "steps": steps,
            "completedAt": current_timestamp(),
        }),
    )
}

async fn proxy_instance_action_step(
    instance_id: &InstanceId,
    role: &str,
    action: &str,
    timeout_seconds: Option<u16>,
    state: &CoreRequestState,
    start: bool,
) -> (Value, bool) {
    let instance = match state.instances().get(instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return (
                orchestration_step(
                    instance_id,
                    role,
                    action,
                    "FAILED",
                    None,
                    Some("INSTANCE_NOT_FOUND"),
                    None,
                ),
                true,
            );
        }
        Err(error) => {
            tracing::error!(%error, %instance_id, "Proxy orchestration could not read an instance");
            return (
                orchestration_step(
                    instance_id,
                    role,
                    action,
                    "FAILED",
                    None,
                    Some("INSTANCE_REPOSITORY_UNAVAILABLE"),
                    None,
                ),
                true,
            );
        }
    };
    let current_state = instance.runtime().state();
    let already_in_target_state = if start {
        current_state == InstanceState::Running
    } else {
        matches!(
            current_state,
            InstanceState::Created | InstanceState::Stopped | InstanceState::Failed
        )
    };
    if already_in_target_state {
        return (
            orchestration_step(
                instance_id,
                role,
                action,
                if start {
                    "ALREADY_RUNNING"
                } else {
                    "ALREADY_STOPPED"
                },
                None,
                None,
                Some(current_state),
            ),
            false,
        );
    }
    if (!start && current_state != InstanceState::Running)
        || (start
            && !matches!(
                current_state,
                InstanceState::Created | InstanceState::Stopped | InstanceState::Failed
            ))
    {
        return (
            orchestration_step(
                instance_id,
                role,
                action,
                "FAILED",
                None,
                Some("INSTANCE_STATE_CONFLICT"),
                Some(current_state),
            ),
            true,
        );
    }

    let result = if start {
        state.processes().start(instance_id).await
    } else {
        state.processes().stop(instance_id, timeout_seconds).await
    };
    match result {
        Ok(task_id) => (
            orchestration_step(
                instance_id,
                role,
                action,
                if start { "STARTED" } else { "STOPPED" },
                Some(task_id),
                None,
                Some(current_state),
            ),
            false,
        ),
        Err(error) => {
            let (error_code, state) = orchestration_process_error(&error);
            (
                orchestration_step(
                    instance_id,
                    role,
                    action,
                    "FAILED",
                    None,
                    Some(error_code),
                    state,
                ),
                true,
            )
        }
    }
}

fn orchestration_step(
    instance_id: &InstanceId,
    role: &str,
    action: &str,
    state: &str,
    task_id: Option<TaskId>,
    error_code: Option<&str>,
    instance_state: Option<InstanceState>,
) -> Value {
    let mut step = json!({
        "instanceId": instance_id,
        "role": role,
        "action": action,
        "state": state,
    });
    if let Some(task_id) = task_id {
        step["taskId"] = json!(task_id);
    }
    if let Some(error_code) = error_code {
        step["errorCode"] = json!(error_code);
    }
    if let Some(instance_state) = instance_state {
        step["instanceState"] = json!(instance_state);
    }
    step
}

fn orchestration_process_error(
    error: &InstanceProcessError,
) -> (&'static str, Option<InstanceState>) {
    match error {
        InstanceProcessError::Repository(InstanceRepositoryError::NotFound { .. }) => {
            ("INSTANCE_NOT_FOUND", None)
        }
        InstanceProcessError::Repository(InstanceRepositoryError::StateConflict {
            state, ..
        }) => ("INSTANCE_STATE_CONFLICT", Some(*state)),
        _ => ("INSTANCE_PROCESS_FAILED", None),
    }
}

async fn minecraft_status_probe(subserver: &ProxySubserver) -> Result<(), &'static str> {
    let mut stream = TcpStream::connect((subserver.host(), subserver.port()))
        .await
        .map_err(|error| proxy_health_error(error.kind()))?;
    let mut handshake = Vec::new();
    write_varint(0, &mut handshake);
    write_varint(MINECRAFT_STATUS_PROTOCOL_VERSION, &mut handshake);
    write_string(subserver.host(), &mut handshake);
    handshake.extend_from_slice(&subserver.port().to_be_bytes());
    write_varint(1, &mut handshake);
    write_minecraft_packet(&mut stream, &handshake).await?;
    write_minecraft_packet(&mut stream, &[0]).await?;
    read_minecraft_status_response(&mut stream).await
}

async fn write_minecraft_packet(
    stream: &mut TcpStream,
    payload: &[u8],
) -> Result<(), &'static str> {
    let length = i32::try_from(payload.len()).map_err(|_| "PROTOCOL_PACKET_TOO_LARGE")?;
    let mut packet = Vec::with_capacity(payload.len().saturating_add(5));
    write_varint(length, &mut packet);
    packet.extend_from_slice(payload);
    stream
        .write_all(&packet)
        .await
        .map_err(|_| "PROTOCOL_WRITE_FAILED")
}

async fn read_minecraft_status_response(stream: &mut TcpStream) -> Result<(), &'static str> {
    let packet_length = read_varint(stream).await?;
    let packet_length = usize::try_from(packet_length)
        .ok()
        .filter(|length| (1..=MAXIMUM_MINECRAFT_STATUS_PACKET_BYTES).contains(length))
        .ok_or("PROTOCOL_INVALID_RESPONSE")?;
    let mut packet = vec![0; packet_length];
    stream
        .read_exact(&mut packet)
        .await
        .map_err(|_| "PROTOCOL_READ_FAILED")?;
    let (packet_id, offset) = read_varint_from_slice(&packet)?;
    if packet_id != 0 {
        return Err("PROTOCOL_INVALID_RESPONSE");
    }
    let (json_length, json_offset) = read_varint_from_slice(&packet[offset..])?;
    let json_length = usize::try_from(json_length)
        .ok()
        .filter(|length| *length <= MAXIMUM_MINECRAFT_STATUS_PACKET_BYTES)
        .ok_or("PROTOCOL_INVALID_RESPONSE")?;
    let json_start = offset
        .checked_add(json_offset)
        .ok_or("PROTOCOL_INVALID_RESPONSE")?;
    let json_end = json_start
        .checked_add(json_length)
        .ok_or("PROTOCOL_INVALID_RESPONSE")?;
    let json_bytes = packet
        .get(json_start..json_end)
        .ok_or("PROTOCOL_INVALID_RESPONSE")?;
    let value: Value =
        serde_json::from_slice(json_bytes).map_err(|_| "PROTOCOL_INVALID_RESPONSE")?;
    let object = value.as_object().ok_or("PROTOCOL_INVALID_RESPONSE")?;
    if object.get("version").and_then(Value::as_object).is_none()
        || !object.contains_key("description")
    {
        return Err("PROTOCOL_INVALID_RESPONSE");
    }

    Ok(())
}

async fn read_varint(stream: &mut TcpStream) -> Result<i32, &'static str> {
    let mut value = 0_u32;
    for index in 0..5 {
        let byte = stream.read_u8().await.map_err(|_| "PROTOCOL_READ_FAILED")?;
        value |= u32::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return i32::try_from(value).map_err(|_| "PROTOCOL_INVALID_RESPONSE");
        }
    }

    Err("PROTOCOL_INVALID_RESPONSE")
}

fn read_varint_from_slice(bytes: &[u8]) -> Result<(i32, usize), &'static str> {
    let mut value = 0_u32;
    for (index, byte) in bytes.iter().copied().enumerate().take(5) {
        value |= u32::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return Ok((
                i32::try_from(value).map_err(|_| "PROTOCOL_INVALID_RESPONSE")?,
                index + 1,
            ));
        }
    }

    Err("PROTOCOL_INVALID_RESPONSE")
}

fn write_varint(value: i32, bytes: &mut Vec<u8>) {
    let mut value = value as u32;
    while value & !0x7f != 0 {
        bytes.push(((value & 0x7f) as u8) | 0x80);
        value >>= 7;
    }
    bytes.push(value as u8);
}

fn write_string(value: &str, bytes: &mut Vec<u8>) {
    write_varint(i32::try_from(value.len()).unwrap_or(i32::MAX), bytes);
    bytes.extend_from_slice(value.as_bytes());
}

fn is_network_probe_error(error: &str) -> bool {
    matches!(
        error,
        "CONNECTION_REFUSED"
            | "CONNECTION_RESET"
            | "ADDRESS_UNAVAILABLE"
            | "TIMEOUT"
            | "CONNECT_FAILED"
    )
}

fn proxy_health_error(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::ConnectionRefused => "CONNECTION_REFUSED",
        ErrorKind::ConnectionReset => "CONNECTION_RESET",
        ErrorKind::AddrNotAvailable => "ADDRESS_UNAVAILABLE",
        ErrorKind::TimedOut => "TIMEOUT",
        _ => "CONNECT_FAILED",
    }
}

fn find_proxy_instance(
    request_id: RequestId,
    params: &Value,
    instances: &InstanceRepository,
) -> Result<Instance, Box<WireMessage>> {
    let Some(proxy_instance_id) = params
        .get("proxyInstanceId")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<InstanceId>().ok())
    else {
        return Err(Box::new(error_response(
            request_id,
            "BAD_REQUEST",
            "proxy operation requires a valid proxyInstanceId",
        )));
    };
    match instances.get(&proxy_instance_id) {
        Ok(Some(instance)) => Ok(instance),
        Ok(None) => Err(Box::new(error_response(
            request_id,
            "INSTANCE_NOT_FOUND",
            "Proxy instance does not exist",
        ))),
        Err(error) => Err(Box::new(repository_failure_response(request_id, &error))),
    }
}

async fn instance_command_response(
    request_id: RequestId,
    params: &Value,
    processes: &InstanceProcessManager,
) -> WireMessage {
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.command requires a valid instanceId",
        );
    };
    let Some(command) = params.get("command").and_then(Value::as_str) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.command requires a command string",
        );
    };

    match processes.command(&instance_id, command).await {
        Ok(accepted_at) => success_response(request_id, json!({ "acceptedAt": accepted_at })),
        Err(error) => process_error_response(request_id, &error),
    }
}

fn instance_logs_response(
    request_id: RequestId,
    params: &Value,
    processes: &InstanceProcessManager,
) -> WireMessage {
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.logs requires a valid instanceId",
        );
    };
    let (after, before, limit) = match instance_log_parameters(params) {
        Ok(parameters) => parameters,
        Err(()) => {
            return error_response(
                request_id,
                "BAD_REQUEST",
                "instance.logs cursor or limit is invalid",
            );
        }
    };

    match processes.logs(&instance_id, after, before, limit) {
        Ok(page) => success_response(request_id, json!(page)),
        Err(error) => process_error_response(request_id, &error),
    }
}

fn instance_metrics_response(
    request_id: RequestId,
    params: &Value,
    processes: &InstanceProcessManager,
) -> WireMessage {
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.metrics requires a valid instanceId",
        );
    };
    if !optional_non_empty_string(params, "range")
        || !optional_non_empty_string(params, "resolution")
    {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.metrics range and resolution must be non-empty strings",
        );
    }

    match processes.metrics(&instance_id) {
        Ok(sample) => success_response(request_id, json!({ "series": [sample] })),
        Err(error) => process_error_response(request_id, &error),
    }
}

fn file_list_response(
    request_id: RequestId,
    params: &Value,
    state: &CoreRequestState,
) -> WireMessage {
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.list requires a valid instanceId",
        );
    };
    let path = match params.get("path") {
        None => "",
        Some(Value::String(path)) => path.as_str(),
        Some(_) => {
            return error_response(request_id, "BAD_REQUEST", "file.list path is invalid");
        }
    };
    let cursor = match params.get("cursor") {
        None => None,
        Some(Value::String(cursor)) => Some(cursor.as_str()),
        Some(_) => {
            return error_response(request_id, "BAD_REQUEST", "file.list cursor is invalid");
        }
    };
    let limit = match params.get("limit") {
        None => None,
        Some(value) => {
            let Some(limit) = value.as_u64().and_then(|value| usize::try_from(value).ok()) else {
                return error_response(request_id, "BAD_REQUEST", "file.list limit is invalid");
            };
            Some(limit)
        }
    };
    let instance = match state.instances().get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };

    match state.files().list(&instance, path, cursor, limit) {
        Ok(page) => success_response(request_id, json!(page)),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn config_scan_response(
    request_id: RequestId,
    params: &Value,
    state: &CoreRequestState,
) -> WireMessage {
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "config.scan requires a valid instanceId",
        );
    };
    let instance = match state.instances().get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };

    match state.files().scan_config_documents(&instance) {
        Ok(documents) => success_response(request_id, documents),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn config_get_response(
    request_id: RequestId,
    params: &Value,
    state: &CoreRequestState,
) -> WireMessage {
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "config.get requires a valid instanceId",
        );
    };
    let Some(document_id) = params
        .get("documentId")
        .and_then(Value::as_str)
        .filter(|document_id| !document_id.is_empty())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "config.get requires a documentId",
        );
    };
    let instance = match state.instances().get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };

    match state.files().get_config_document(&instance, document_id) {
        Ok(document) => success_response(request_id, document),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn config_patch_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "config.patch requires a valid instanceId",
        );
    };
    let Some(document_id) = params
        .get("documentId")
        .and_then(Value::as_str)
        .filter(|document_id| !document_id.is_empty())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "config.patch requires a documentId",
        );
    };
    let Some(revision) = params.get("revision").and_then(Value::as_str) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "config.patch requires a revision",
        );
    };
    let Some(patch) = params.get("patch") else {
        return error_response(request_id, "BAD_REQUEST", "config.patch requires a patch");
    };
    let allow_lossy = match params.get("allowLossy") {
        None => false,
        Some(Value::Bool(value)) => *value,
        Some(_) => {
            return error_response(
                request_id,
                "BAD_REQUEST",
                "config.patch allowLossy is invalid",
            );
        }
    };
    let instance = match state.instances().get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };

    match state
        .files()
        .patch_config_document(&instance, document_id, revision, patch, allow_lossy)
    {
        Ok(document) => success_response(request_id, document),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn file_read_response(
    request_id: RequestId,
    params: &Value,
    state: &CoreRequestState,
) -> WireMessage {
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.read requires a valid instanceId",
        );
    };
    let Some(path) = params
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    else {
        return error_response(request_id, "BAD_REQUEST", "file.read requires a path");
    };
    let Some(offset) = params.get("offset").and_then(Value::as_u64) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.read requires a valid offset",
        );
    };
    let Some(length) = params
        .get("length")
        .and_then(Value::as_u64)
        .and_then(|length| usize::try_from(length).ok())
        .filter(|length| (1..=MAXIMUM_FILE_READ_BYTES).contains(length))
    else {
        return error_response(request_id, "BAD_REQUEST", "file.read length is invalid");
    };
    let instance = match state.instances().get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };

    match state.files().read(&instance, path, offset, length) {
        Ok(content) => success_response(request_id, json!(content)),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn file_write_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.write requires a valid instanceId",
        );
    };
    let Some(path) = params
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    else {
        return error_response(request_id, "BAD_REQUEST", "file.write requires a path");
    };
    let Some(data_base64) = params.get("dataBase64").and_then(Value::as_str) else {
        return error_response(request_id, "BAD_REQUEST", "file.write requires dataBase64");
    };
    let Ok(content) = STANDARD.decode(data_base64) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.write dataBase64 is invalid",
        );
    };
    let expected_sha256 = match params.get("expectedSha256") {
        None => None,
        Some(Value::String(value)) => Some(value.as_str()),
        Some(_) => {
            return error_response(
                request_id,
                "BAD_REQUEST",
                "file.write expectedSha256 is invalid",
            );
        }
    };
    let instance = match state.instances().get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };

    match state
        .files()
        .write(&instance, path, &content, expected_sha256)
    {
        Ok(entry) => success_response(request_id, json!(entry)),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn file_mkdir_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.mkdir requires a valid instanceId",
        );
    };
    let Some(path) = params
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    else {
        return error_response(request_id, "BAD_REQUEST", "file.mkdir requires a path");
    };
    let recursive = match params.get("recursive") {
        None => false,
        Some(Value::Bool(recursive)) => *recursive,
        Some(_) => {
            return error_response(request_id, "BAD_REQUEST", "file.mkdir recursive is invalid");
        }
    };
    let instance = match state.instances().get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };

    match state.files().mkdir(&instance, path, recursive) {
        Ok(entry) => success_response(request_id, json!(entry)),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn file_move_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.move requires a valid instanceId",
        );
    };
    let Some(from) = params
        .get("from")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    else {
        return error_response(request_id, "BAD_REQUEST", "file.move requires from");
    };
    let Some(to) = params
        .get("to")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    else {
        return error_response(request_id, "BAD_REQUEST", "file.move requires to");
    };
    let overwrite = match params.get("overwrite") {
        None => false,
        Some(Value::Bool(overwrite)) => *overwrite,
        Some(_) => {
            return error_response(request_id, "BAD_REQUEST", "file.move overwrite is invalid");
        }
    };
    let instance = match state.instances().get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };

    match state.files().move_entry(&instance, from, to, overwrite) {
        Ok(entry) => success_response(request_id, json!(entry)),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn file_delete_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.delete requires a valid instanceId",
        );
    };
    let Some(path) = params
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    else {
        return error_response(request_id, "BAD_REQUEST", "file.delete requires a path");
    };
    if params.get("confirmation").and_then(Value::as_str) != Some("DELETE") {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.delete requires DELETE confirmation",
        );
    }
    let recursive = match params.get("recursive") {
        None => false,
        Some(Value::Bool(recursive)) => *recursive,
        Some(_) => {
            return error_response(
                request_id,
                "BAD_REQUEST",
                "file.delete recursive is invalid",
            );
        }
    };
    let instance = match state.instances().get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };

    match state.files().start_delete(&instance, path, recursive) {
        Ok(task_id) => task_accepted_response(request_id, task_id),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn file_batch_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.batch requires a valid instanceId",
        );
    };
    let Some(operation_values) = params.get("operations").and_then(Value::as_array) else {
        return error_response(request_id, "BAD_REQUEST", "file.batch requires operations");
    };
    if operation_values.is_empty() || operation_values.len() > MAXIMUM_FILE_BATCH_OPERATIONS {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.batch operation count is invalid",
        );
    }
    let Ok(operations) = operation_values
        .iter()
        .map(FileBatchOperation::from_value)
        .collect::<Result<Vec<_>, _>>()
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.batch contains an invalid operation",
        );
    };
    let instance = match state.instances().get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };

    match state.files().start_batch(&instance, operations) {
        Ok(task_id) => task_accepted_response(request_id, task_id),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn file_archive_create_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    if params.get("format").and_then(Value::as_str) != Some("ZIP") {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.archive.create format is invalid",
        );
    }
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.archive.create requires a valid instanceId",
        );
    };
    let Some(path_values) = params.get("paths").and_then(Value::as_array) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.archive.create requires paths",
        );
    };
    if path_values.is_empty() || path_values.len() > MAXIMUM_FILE_ARCHIVE_PATHS {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.archive.create path count is invalid",
        );
    }
    let Some(paths) = path_values
        .iter()
        .map(|value| value.as_str().map(|value| value.to_owned()))
        .collect::<Option<Vec<_>>>()
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.archive.create contains an invalid path",
        );
    };
    let Some(output_path) = params
        .get("outputPath")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.archive.create requires outputPath",
        );
    };
    let instance = match state.instances().get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };

    match state
        .files()
        .start_archive(&instance, paths, output_path.to_owned())
    {
        Ok(task_id) => task_accepted_response(request_id, task_id),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn file_task_response(
    request_id: RequestId,
    params: &Value,
    state: &CoreRequestState,
) -> WireMessage {
    let Some(task_id) = params
        .get("taskId")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<TaskId>().ok())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "file.task.get requires a valid taskId",
        );
    };

    match state.files().task(task_id) {
        Ok(Some(task)) => success_response(request_id, task),
        Ok(None) => error_response(
            request_id,
            "FILE_TASK_NOT_FOUND",
            "File task does not exist",
        ),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn transfer_begin_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(mode) = params.get("mode").and_then(Value::as_str) else {
        return error_response(request_id, "BAD_REQUEST", "transfer.begin mode is invalid");
    };
    if !matches!(mode, "UPLOAD" | "DOWNLOAD") {
        return error_response(request_id, "BAD_REQUEST", "transfer.begin mode is invalid");
    }
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "transfer.begin requires a valid instanceId",
        );
    };
    let Some(path) = params
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    else {
        return error_response(request_id, "BAD_REQUEST", "transfer.begin requires a path");
    };
    let instance = match state.instances().get(&instance_id) {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            return error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist");
        }
        Err(error) => return repository_failure_response(request_id, &error),
    };

    let result = match mode {
        "UPLOAD" => {
            let Some(size) = params.get("size").and_then(Value::as_u64) else {
                return error_response(
                    request_id,
                    "BAD_REQUEST",
                    "transfer.begin requires a valid size",
                );
            };
            let Some(sha256) = params.get("sha256").and_then(Value::as_str) else {
                return error_response(
                    request_id,
                    "BAD_REQUEST",
                    "transfer.begin requires a sha256",
                );
            };
            let expected_target_sha256 = match params.get("expectedSha256") {
                None => None,
                Some(Value::String(value)) => Some(value.as_str()),
                Some(_) => {
                    return error_response(
                        request_id,
                        "BAD_REQUEST",
                        "transfer.begin expectedSha256 is invalid",
                    );
                }
            };
            state
                .files()
                .begin_upload(&instance, path, size, sha256, expected_target_sha256)
        }
        "DOWNLOAD" => state.files().begin_download(&instance, path),
        _ => unreachable!("transfer mode was validated above"),
    };
    match result {
        Ok(result) => success_response(request_id, result),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn transfer_chunk_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    let Some(transfer_id) = params
        .get("transferId")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<TaskId>().ok())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "transfer.chunk requires a valid transferId",
        );
    };
    let Some(offset) = params.get("offset").and_then(Value::as_u64) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "transfer.chunk requires a valid offset",
        );
    };
    if params.get("dataBase64").is_none() {
        match state.files().read_download_chunk(transfer_id, offset) {
            Ok(result) => return success_response(request_id, result),
            Err(error) => return file_manager_error_response(request_id, error),
        }
    }
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(data_base64) = params.get("dataBase64").and_then(Value::as_str) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "transfer.chunk dataBase64 is invalid",
        );
    };
    let Ok(content) = STANDARD.decode(data_base64) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "transfer.chunk dataBase64 is invalid",
        );
    };
    let expected_sha256 = match params.get("sha256") {
        None => None,
        Some(Value::String(value)) => Some(value.as_str()),
        Some(_) => {
            return error_response(
                request_id,
                "BAD_REQUEST",
                "transfer.chunk sha256 is invalid",
            );
        }
    };
    if content.len() > FILE_TRANSFER_CHUNK_BYTES {
        return error_response(
            request_id,
            "PAYLOAD_TOO_LARGE",
            "File transfer chunk exceeds the maximum size",
        );
    }

    match state
        .files()
        .write_upload_chunk(transfer_id, offset, &content, expected_sha256)
    {
        Ok(result) => success_response(request_id, result),
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn transfer_commit_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(transfer_id) = params
        .get("transferId")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<TaskId>().ok())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "transfer.commit requires a valid transferId",
        );
    };

    match state.files().commit_upload(transfer_id) {
        Ok(entry) => success_response(request_id, json!(entry)),
        Err(FileManagerError::TransferNotFound { .. }) => {
            match state.files().commit_download(transfer_id) {
                Ok(()) => success_response(request_id, json!({})),
                Err(error) => file_manager_error_response(request_id, error),
            }
        }
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn transfer_abort_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    state: &CoreRequestState,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(transfer_id) = params
        .get("transferId")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<TaskId>().ok())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "transfer.abort requires a valid transferId",
        );
    };

    match state.files().abort_upload(transfer_id) {
        Ok(()) => success_response(request_id, json!({})),
        Err(FileManagerError::TransferNotFound { .. }) => {
            match state.files().abort_download(transfer_id) {
                Ok(()) => success_response(request_id, json!({})),
                Err(error) => file_manager_error_response(request_id, error),
            }
        }
        Err(error) => file_manager_error_response(request_id, error),
    }
}

fn file_manager_error_response(request_id: RequestId, error: FileManagerError) -> WireMessage {
    match error {
        FileManagerError::InvalidPath { .. } | FileManagerError::InvalidHash { .. } => {
            error_response(
                request_id,
                "BAD_REQUEST",
                "File operation parameters are invalid",
            )
        }
        FileManagerError::ContentTooLarge { .. } => error_response(
            request_id,
            "PAYLOAD_TOO_LARGE",
            "File content exceeds the maximum size",
        ),
        FileManagerError::ArchiveTooLarge { .. } => error_response(
            request_id,
            "PAYLOAD_TOO_LARGE",
            "File archive exceeds the maximum size",
        ),
        FileManagerError::TransferChunkTooLarge { .. } => error_response(
            request_id,
            "PAYLOAD_TOO_LARGE",
            "File transfer chunk exceeds the maximum size",
        ),
        FileManagerError::NotFound { .. } => {
            error_response(request_id, "FILE_NOT_FOUND", "File does not exist")
        }
        FileManagerError::NotDirectory { .. } => {
            error_response(request_id, "FILE_NOT_DIRECTORY", "Path is not a directory")
        }
        FileManagerError::NotFile { .. } => {
            error_response(request_id, "FILE_NOT_REGULAR", "Path is not a regular file")
        }
        FileManagerError::SymlinkNotAllowed { .. } | FileManagerError::PathEscapes { .. } => {
            error_response(
                request_id,
                "FILE_PATH_FORBIDDEN",
                "File path is not allowed",
            )
        }
        FileManagerError::HashMismatch { expected, actual } => error_response_with_details(
            request_id,
            "FILE_REVISION_MISMATCH",
            "File hash does not match",
            false,
            Some(json!({ "expectedSha256": expected, "actualSha256": actual })),
        ),
        FileManagerError::TransferHashMismatch { expected, actual }
        | FileManagerError::TransferChunkHashMismatch { expected, actual } => {
            error_response_with_details(
                request_id,
                "FILE_TRANSFER_HASH_MISMATCH",
                "File transfer hash does not match",
                false,
                Some(json!({ "expectedSha256": expected, "actualSha256": actual })),
            )
        }
        FileManagerError::TransferOffsetMismatch { expected, actual } => {
            error_response_with_details(
                request_id,
                "FILE_TRANSFER_OFFSET_MISMATCH",
                "File transfer offset is invalid",
                false,
                Some(json!({ "expectedOffset": expected, "actualOffset": actual })),
            )
        }
        FileManagerError::TransferIncomplete { expected, actual }
        | FileManagerError::TransferSizeMismatch { expected, actual } => {
            error_response_with_details(
                request_id,
                "FILE_TRANSFER_SIZE_MISMATCH",
                "File transfer size is invalid",
                false,
                Some(json!({ "expectedBytes": expected, "actualBytes": actual })),
            )
        }
        FileManagerError::TransferNotFound { .. } => error_response(
            request_id,
            "FILE_TRANSFER_NOT_FOUND",
            "File transfer does not exist",
        ),
        FileManagerError::TooManyTransfers => error_response(
            request_id,
            "FILE_TRANSFER_LIMIT_REACHED",
            "Too many active file transfers",
        ),
        FileManagerError::ConfigDocumentNotFound { .. } => error_response(
            request_id,
            "CONFIG_DOCUMENT_NOT_FOUND",
            "Configuration document does not exist",
        ),
        FileManagerError::ConfigParse { .. } => error_response(
            request_id,
            "CONFIG_PARSE_FAILED",
            "Configuration document could not be parsed",
        ),
        FileManagerError::ConfigPatchInvalid { .. } => error_response(
            request_id,
            "CONFIG_PATCH_INVALID",
            "Configuration patch is invalid",
        ),
        FileManagerError::ConfigRevisionMismatch { expected, actual } => {
            error_response_with_details(
                request_id,
                "CONFIG_REVISION_MISMATCH",
                "Configuration document changed",
                false,
                Some(json!({ "expectedRevision": expected, "actualRevision": actual })),
            )
        }
        FileManagerError::ConfigScanTooLarge { .. } => error_response(
            request_id,
            "CONFIG_SCAN_TOO_LARGE",
            "Too many configuration documents",
        ),
        FileManagerError::AlreadyExists { .. } => error_response(
            request_id,
            "FILE_ALREADY_EXISTS",
            "File target already exists",
        ),
        FileManagerError::DirectoryNotEmpty { .. } => error_response(
            request_id,
            "FILE_DIRECTORY_NOT_EMPTY",
            "Target directory is not empty",
        ),
        FileManagerError::TaskStorePoisoned => error_response_with_details(
            request_id,
            "FILE_OPERATION_FAILED",
            "File operation failed",
            true,
            None,
        ),
        error => {
            tracing::error!(%error, "Core file operation failed");
            error_response_with_details(
                request_id,
                "FILE_OPERATION_FAILED",
                "File operation failed",
                true,
                None,
            )
        }
    }
}

async fn instance_start_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    processes: &InstanceProcessManager,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.start requires a valid instanceId",
        );
    };

    match processes.start(&instance_id).await {
        Ok(task_id) => task_accepted_response(request_id, task_id),
        Err(error) => process_error_response(request_id, &error),
    }
}

async fn instance_stop_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    processes: &InstanceProcessManager,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.stop requires a valid instanceId",
        );
    };
    let timeout_seconds = match optional_timeout_seconds(params) {
        Ok(timeout_seconds) => timeout_seconds,
        Err(()) => {
            return error_response(
                request_id,
                "BAD_REQUEST",
                "instance.stop timeoutSeconds must be between 1 and 300",
            );
        }
    };

    match processes.stop(&instance_id, timeout_seconds).await {
        Ok(task_id) => task_accepted_response(request_id, task_id),
        Err(error) => process_error_response(request_id, &error),
    }
}

async fn instance_kill_response(
    request_id: RequestId,
    params: &Value,
    idempotency_key: Option<&str>,
    processes: &InstanceProcessManager,
) -> WireMessage {
    if idempotency_key.is_none() {
        return missing_idempotency_key_response(request_id);
    }
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.kill requires a valid instanceId",
        );
    };
    let confirmation_matches = params
        .get("confirmation")
        .and_then(Value::as_str)
        .is_some_and(|confirmation| confirmation == instance_id.as_str());
    if !confirmation_matches {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.kill confirmation must match instanceId",
        );
    }

    match processes.kill(&instance_id).await {
        Ok(task_id) => task_accepted_response(request_id, task_id),
        Err(error) => process_error_response(request_id, &error),
    }
}

fn event_subscribe_response(
    request_id: RequestId,
    params: &Value,
    state: &mut CoreRequestState,
) -> WireMessage {
    let Some(topics) = event_topics_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "event.subscribe requires one or more supported topics",
        );
    };
    let subscription_id = RequestId::new();
    state.subscribe_to_events(subscription_id, topics);

    success_response(request_id, json!({ "subscriptionId": subscription_id }))
}

fn event_unsubscribe_response(
    request_id: RequestId,
    params: &Value,
    state: &mut CoreRequestState,
) -> WireMessage {
    let subscription_id = params
        .get("subscriptionId")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<RequestId>().ok());
    if subscription_id.is_none() || subscription_id != state.event_subscription() {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "event.unsubscribe requires the active subscriptionId",
        );
    }
    state.unsubscribe_from_events();

    success_response(request_id, json!({}))
}

fn instance_create_response(
    request_id: RequestId,
    params: &Value,
    instances: &InstanceRepository,
) -> WireMessage {
    let definition = match from_value::<InstanceCreate>(params.clone()) {
        Ok(definition) => definition,
        Err(_) => {
            return error_response(
                request_id,
                "BAD_REQUEST",
                "instance.create requires a valid instance definition",
            );
        }
    };

    match instances.create(definition) {
        Ok(instance) => success_response(request_id, json!(instance)),
        Err(InstanceRepositoryError::AlreadyExists { .. }) => error_response(
            request_id,
            "INSTANCE_ALREADY_EXISTS",
            "An instance with this ID already exists",
        ),
        Err(InstanceRepositoryError::InvalidInstance(_)) => error_response(
            request_id,
            "BAD_REQUEST",
            "instance.create requires a valid instance definition",
        ),
        Err(error) => repository_failure_response(request_id, &error),
    }
}

fn instance_get_response(
    request_id: RequestId,
    params: &Value,
    instances: &InstanceRepository,
) -> WireMessage {
    let Some(instance_id) = params
        .get("instanceId")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<InstanceId>().ok())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.get requires a valid instanceId",
        );
    };

    match instances.get(&instance_id) {
        Ok(Some(instance)) => success_response(request_id, json!(instance)),
        Ok(None) => error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist"),
        Err(error) => repository_failure_response(request_id, &error),
    }
}

fn instance_update_response(
    request_id: RequestId,
    params: &Value,
    instances: &InstanceRepository,
) -> WireMessage {
    let Some(instance_id) = instance_id_parameter(params) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.update requires a valid instanceId",
        );
    };
    let Some(expected_revision) = params.get("expectedRevision").and_then(Value::as_u64) else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.update requires an expectedRevision",
        );
    };
    let Some(patch) = params
        .get("patch")
        .cloned()
        .and_then(|value| from_value::<InstanceUpdate>(value).ok())
    else {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "instance.update requires a valid patch",
        );
    };

    match instances.update(&instance_id, expected_revision, &patch) {
        Ok(instance) => success_response(request_id, json!(instance)),
        Err(InstanceRepositoryError::InvalidUpdate(_)) => error_response(
            request_id,
            "BAD_REQUEST",
            "instance.update requires a valid patch",
        ),
        Err(InstanceRepositoryError::NotFound { .. }) => {
            error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist")
        }
        Err(InstanceRepositoryError::RevisionMismatch {
            actual_revision, ..
        }) => error_response_with_details(
            request_id,
            "REVISION_MISMATCH",
            "Instance revision does not match",
            false,
            Some(json!({ "actualRevision": actual_revision })),
        ),
        Err(InstanceRepositoryError::StateConflict { state, .. }) => error_response_with_details(
            request_id,
            "INSTANCE_STATE_CONFLICT",
            "Instance state does not allow settings changes",
            false,
            Some(json!({ "state": state })),
        ),
        Err(error) => repository_failure_response(request_id, &error),
    }
}

fn instance_list_response(
    request_id: RequestId,
    params: &Value,
    instances: &InstanceRepository,
) -> WireMessage {
    let (cursor, limit, state) = match instance_list_parameters(params) {
        Ok(parameters) => parameters,
        Err(()) => {
            return error_response(
                request_id,
                "BAD_REQUEST",
                "instance.list parameters are invalid",
            );
        }
    };
    let instances = match instances.list() {
        Ok(instances) => instances,
        Err(error) => return repository_failure_response(request_id, &error),
    };
    let filtered = match state {
        Some(state) => instances
            .into_iter()
            .filter(|instance| instance.runtime().state() == state)
            .collect::<Vec<_>>(),
        None => instances,
    };
    let start_index = cursor.as_ref().map_or(0, |cursor| {
        filtered.partition_point(|instance| instance.id() <= cursor)
    });
    let has_more = filtered.len().saturating_sub(start_index) > limit;
    let page = filtered
        .into_iter()
        .skip(start_index)
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = if has_more {
        page.last().map(|instance| instance.id().to_string())
    } else {
        None
    };

    success_response(request_id, json!(InstancePage::new(page, next_cursor)))
}

fn instance_list_parameters(
    params: &Value,
) -> Result<(Option<InstanceId>, usize, Option<InstanceState>), ()> {
    let cursor = match params.get("cursor") {
        Some(value) => Some(
            value
                .as_str()
                .ok_or(())?
                .parse::<InstanceId>()
                .map_err(|_| ())?,
        ),
        None => None,
    };
    let limit = match params.get("limit") {
        Some(value) => value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| (1..=INSTANCE_LIST_MAXIMUM_LIMIT).contains(value))
            .ok_or(())?,
        None => INSTANCE_LIST_DEFAULT_LIMIT,
    };
    let state = match params.get("state") {
        Some(value) => Some(from_value(value.clone()).map_err(|_| ())?),
        None => None,
    };

    Ok((cursor, limit, state))
}

fn instance_log_parameters(params: &Value) -> Result<(Option<u64>, Option<u64>, usize), ()> {
    let after = optional_log_cursor(params, "after")?;
    let before = optional_log_cursor(params, "before")?;
    let limit = match params.get("limit") {
        Some(value) => value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| (1..=INSTANCE_LOG_MAXIMUM_LIMIT).contains(value))
            .ok_or(())?,
        None => INSTANCE_LOG_DEFAULT_LIMIT,
    };

    Ok((after, before, limit))
}

fn optional_log_cursor(params: &Value, name: &str) -> Result<Option<u64>, ()> {
    match params.get(name) {
        Some(value) => value.as_str().ok_or(())?.parse().map(Some).map_err(|_| ()),
        None => Ok(None),
    }
}

fn optional_non_empty_string(params: &Value, name: &str) -> bool {
    params
        .get(name)
        .is_none_or(|value| value.as_str().is_some_and(|value| !value.is_empty()))
}

fn event_topics_parameter(params: &Value) -> Option<BTreeSet<String>> {
    let topics = params.get("topics")?.as_array()?;
    if topics.is_empty() {
        return None;
    }

    topics
        .iter()
        .map(Value::as_str)
        .map(|topic| {
            topic
                .filter(|topic| EVENT_TOPICS.contains(topic))
                .map(str::to_owned)
        })
        .collect()
}

fn instance_id_parameter(params: &Value) -> Option<InstanceId> {
    params
        .get("instanceId")
        .and_then(Value::as_str)
        .and_then(|value| value.parse().ok())
}

fn optional_timeout_seconds(params: &Value) -> Result<Option<u16>, ()> {
    match params.get("timeoutSeconds") {
        Some(value) => value
            .as_u64()
            .and_then(|value| u16::try_from(value).ok())
            .filter(|value| (1..=300).contains(value))
            .map(Some)
            .ok_or(()),
        None => Ok(None),
    }
}

fn missing_idempotency_key_response(request_id: RequestId) -> WireMessage {
    error_response(
        request_id,
        "PRECONDITION_REQUIRED",
        "Instance lifecycle operations require idempotencyKey",
    )
}

fn task_accepted_response(request_id: RequestId, task_id: TaskId) -> WireMessage {
    success_response(
        request_id,
        json!({
            "taskId": task_id,
            "acceptedAt": current_timestamp(),
        }),
    )
}

fn process_error_response(request_id: RequestId, error: &InstanceProcessError) -> WireMessage {
    match error {
        InstanceProcessError::CommandContainsNul
        | InstanceProcessError::CommandEmpty
        | InstanceProcessError::CommandTooLong { .. } => {
            error_response(request_id, "BAD_REQUEST", "Instance command is invalid")
        }
        InstanceProcessError::Repository(InstanceRepositoryError::NotFound { .. }) => {
            error_response(request_id, "INSTANCE_NOT_FOUND", "Instance does not exist")
        }
        InstanceProcessError::Repository(InstanceRepositoryError::StateConflict {
            state, ..
        }) => error_response_with_details(
            request_id,
            "INSTANCE_STATE_CONFLICT",
            "Instance state does not allow this operation",
            false,
            Some(json!({ "state": state })),
        ),
        _ => {
            tracing::error!(%error, "Instance process operation failed");
            error_response_with_details(
                request_id,
                "INSTANCE_PROCESS_FAILED",
                "Instance process operation failed",
                true,
                None,
            )
        }
    }
}

fn event_topic(message: &WireMessage) -> Option<&str> {
    match message {
        WireMessage::Event { topic, .. } => Some(topic),
        WireMessage::Request { .. } | WireMessage::Response { .. } => None,
    }
}

fn repository_failure_response(
    request_id: RequestId,
    error: &InstanceRepositoryError,
) -> WireMessage {
    tracing::error!(error = %error, "Core instance repository is unavailable");

    error_response(
        request_id,
        "INTERNAL_ERROR",
        "Core instance repository is unavailable",
    )
}

fn proxy_subserver_repository_failure_response(
    request_id: RequestId,
    error: &ProxySubserverRepositoryError,
) -> WireMessage {
    match error {
        ProxySubserverRepositoryError::UnsupportedProxy { .. } => error_response(
            request_id,
            "PROXY_TOPOLOGY_UNSUPPORTED",
            "The instance does not support proxy subservers",
        ),
        ProxySubserverRepositoryError::TopologyLimit { .. } => error_response(
            request_id,
            "PROXY_SUBSERVER_LIMIT_REACHED",
            "The proxy topology does not allow another subserver",
        ),
        ProxySubserverRepositoryError::NotFound { .. } => error_response(
            request_id,
            "PROXY_SUBSERVER_NOT_FOUND",
            "Proxy subserver does not exist",
        ),
        ProxySubserverRepositoryError::Invalid(error) => {
            error_response(request_id, "BAD_REQUEST", error.to_string().as_str())
        }
        ProxySubserverRepositoryError::LockPoisoned => {
            tracing::error!(%error, "Core proxy subserver repository is unavailable");
            error_response(
                request_id,
                "INTERNAL_ERROR",
                "Core proxy subserver repository is unavailable",
            )
        }
    }
}

fn protocol_from_hello(params: &Value) -> Option<ProtocolVersion> {
    params
        .get("protocol")
        .cloned()
        .and_then(|value| from_value(value).ok())
}

fn negotiated_capabilities(params: &Value) -> Vec<&'static str> {
    let Some(remote_capabilities) = params.get("capabilities").and_then(Value::as_array) else {
        return Vec::new();
    };

    CORE_CAPABILITIES
        .iter()
        .copied()
        .filter(|capability| {
            remote_capabilities
                .iter()
                .any(|value| value.as_str() == Some(*capability))
        })
        .collect()
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

fn success_response(request_id: RequestId, result: Value) -> WireMessage {
    WireMessage::Response {
        request_id,
        ok: true,
        result: Some(result),
        error: None,
    }
}

fn error_response(request_id: RequestId, code: &str, message: &str) -> WireMessage {
    error_response_with_details(request_id, code, message, false, None)
}

fn error_response_with_details(
    request_id: RequestId,
    code: &str,
    message: &str,
    retryable: bool,
    details: Option<Value>,
) -> WireMessage {
    WireMessage::Response {
        request_id,
        ok: false,
        result: None,
        error: Some(WireError {
            code: code.to_owned(),
            message: message.to_owned(),
            retryable,
            details,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::net::Ipv4Addr;
    use std::net::Ipv6Addr;
    use std::time::Duration;

    use super::RAKNET_MAGIC;
    use super::RAKNET_UNCONNECTED_PING_ID;
    use super::RAKNET_UNCONNECTED_PONG_ID;
    use super::bedrock_probe_address;
    use super::bedrock_raknet_probe;
    use super::minecraft_status_probe;
    use super::parse_geyser_address;
    use super::parse_geyser_port;
    use super::parse_server_properties_address;
    use super::parse_server_properties_port;
    use nexus_domain::InstanceId;
    use nexus_domain::ProxySubserver;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;
    use tokio::net::UdpSocket;
    use tokio::spawn;
    use tokio::time::timeout;

    #[tokio::test]
    async fn rejects_a_non_minecraft_status_response() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test status listener binds");
        let port = listener
            .local_addr()
            .expect("test status listener address is available")
            .port();
        let server_task = spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("status connection arrives");
            let handshake_length = super::read_varint(&mut stream)
                .await
                .expect("status handshake has a length");
            let mut handshake = vec![0; handshake_length as usize];
            stream
                .read_exact(&mut handshake)
                .await
                .expect("status handshake is readable");
            let request_length = super::read_varint(&mut stream)
                .await
                .expect("status request has a length");
            let mut request = vec![0; request_length as usize];
            stream
                .read_exact(&mut request)
                .await
                .expect("status request is readable");
            let status_json = br#"{"description":"not a Minecraft status"}"#;
            let mut payload = Vec::new();
            super::write_varint(0, &mut payload);
            super::write_varint(status_json.len() as i32, &mut payload);
            payload.extend_from_slice(status_json);
            let mut response = Vec::new();
            super::write_varint(payload.len() as i32, &mut response);
            response.extend_from_slice(&payload);
            stream
                .write_all(&response)
                .await
                .expect("invalid status response is writable");
        });
        let subserver = ProxySubserver::new(
            "status".to_owned(),
            "Status".to_owned(),
            InstanceId::new("backend".to_owned()).expect("backend ID is valid"),
            "127.0.0.1".to_owned(),
            port,
            true,
        )
        .expect("test subserver is valid");

        assert_eq!(
            minecraft_status_probe(&subserver).await,
            Err("PROTOCOL_INVALID_RESPONSE")
        );
        server_task.await.expect("invalid status server completes");
    }

    #[tokio::test]
    async fn probes_a_raknet_unconnected_pong() {
        let socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("RakNet test listener binds");
        let address = socket
            .local_addr()
            .expect("RakNet test listener address is available");
        let server_task = spawn(async move {
            let mut ping = [0_u8; 128];
            let (length, peer) = socket
                .recv_from(&mut ping)
                .await
                .expect("RakNet ping arrives");
            assert_eq!(ping[0], RAKNET_UNCONNECTED_PING_ID);
            assert_eq!(&ping[9..25], &RAKNET_MAGIC);

            let identity = b"MCPE;MCNP;1;0;0;0;0;Bedrock;1";
            let mut pong = Vec::with_capacity(35 + identity.len());
            pong.push(RAKNET_UNCONNECTED_PONG_ID);
            pong.extend_from_slice(&0_u64.to_be_bytes());
            pong.extend_from_slice(&0_u64.to_be_bytes());
            pong.extend_from_slice(&RAKNET_MAGIC);
            pong.extend_from_slice(&(identity.len() as u16).to_be_bytes());
            pong.extend_from_slice(identity);
            socket
                .send_to(&pong, peer)
                .await
                .expect("RakNet pong is sent");
            length
        });

        let identity = timeout(
            Duration::from_secs(1),
            bedrock_raknet_probe(address.ip(), address.port()),
        )
        .await
        .expect("RakNet probe completes")
        .expect("RakNet probe receives a valid pong");
        assert_eq!(identity, "MCPE;MCNP;1;0;0;0;0;Bedrock;1");
        assert_eq!(server_task.await.expect("RakNet server completes"), 33);
    }

    #[test]
    fn maps_unspecified_bedrock_bind_addresses_to_loopback_probes() {
        assert_eq!(
            bedrock_probe_address(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        );
        assert_eq!(
            bedrock_probe_address(IpAddr::V6(Ipv6Addr::UNSPECIFIED)),
            IpAddr::V6(Ipv6Addr::LOCALHOST)
        );
    }

    #[test]
    fn reads_the_bedrock_port_from_server_properties() {
        assert_eq!(
            parse_server_properties_port(b"motd=MCNP\nserver-port=19133\n"),
            Ok(19133)
        );
    }

    #[test]
    fn reads_the_bedrock_port_from_geyser_yaml() {
        assert_eq!(
            parse_geyser_port(b"bedrock:\n  address: 0.0.0.0\n  port: 19134\n"),
            Ok(19134)
        );
    }

    #[test]
    fn reads_the_bedrock_bind_address_from_server_properties() {
        assert_eq!(
            parse_server_properties_address(b"server-ip=192.0.2.10\nserver-port=19133\n")
                .expect("server-ip is valid")
                .map(|address| address.to_string()),
            Some("192.0.2.10".to_owned())
        );
    }

    #[test]
    fn reads_the_bedrock_bind_address_from_geyser_yaml() {
        assert_eq!(
            parse_geyser_address(b"bedrock:\n  address: '::'\n  port: 19134\n")
                .expect("Geyser address is valid")
                .map(|address| address.to_string()),
            Some("::".to_owned())
        );
    }

    #[test]
    fn rejects_invalid_bedrock_ports() {
        assert_eq!(
            parse_server_properties_port(b"server-port=0\n"),
            Err("CONFIGURATION_PORT_INVALID")
        );
        assert_eq!(
            parse_geyser_port(b"bedrock:\n  port: invalid\n"),
            Err("CONFIGURATION_PORT_INVALID")
        );
    }

    #[test]
    fn rejects_invalid_bedrock_bind_addresses() {
        assert_eq!(
            parse_server_properties_address(b"server-ip=example.invalid\n"),
            Err("CONFIGURATION_ADDRESS_INVALID")
        );
        assert_eq!(
            parse_geyser_address(b"bedrock:\n  address: example.invalid\n"),
            Err("CONFIGURATION_ADDRESS_INVALID")
        );
    }
}
