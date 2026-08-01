use std::fs;
use std::net::SocketAddr;
use std::path::Path;

use nexus_config::CoreConfig;
use nexus_domain::CoreId;
use nexus_domain::InstanceCreate;
use nexus_domain::InstanceId;
use nexus_domain::InstancePage;
use nexus_domain::InstanceState;
use nexus_domain::PRODUCT_VERSION;
use nexus_domain::RequestId;
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
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::select;
use tokio::spawn;
use tokio::sync::broadcast::error::RecvError;

use crate::CoreError;
use crate::CoreRequestState;
use crate::InstanceProcessError;
use crate::InstanceProcessManager;
use crate::InstanceRepository;
use crate::InstanceRepositoryError;

const CORE_CAPABILITIES: [&str; 2] = ["events", "instances"];
const CORE_ID_FILE_NAME: &str = "core-id";
const HEARTBEAT_SECONDS: u64 = 20;
const INSTANCE_LIST_DEFAULT_LIMIT: usize = 50;
const INSTANCE_LIST_MAXIMUM_LIMIT: usize = 200;

pub struct CoreServer {
    core_id: CoreId,
    listen_address: SocketAddr,
    listener: TcpListener,
    pre_shared_key: PresharedKey,
    instances: InstanceRepository,
    processes: InstanceProcessManager,
}

impl CoreServer {
    pub async fn bind(config: &CoreConfig) -> Result<Self, CoreError> {
        let pre_shared_key = config
            .pre_shared_key()
            .ok_or(CoreError::MissingPreSharedKey)?
            .clone();
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

        Ok(Self {
            core_id,
            listen_address,
            listener,
            pre_shared_key,
            instances,
            processes,
        })
    }

    #[must_use]
    pub const fn core_id(&self) -> CoreId {
        self.core_id
    }

    #[must_use]
    pub const fn listen_address(&self) -> SocketAddr {
        self.listen_address
    }

    #[must_use]
    pub fn instance_repository(&self) -> InstanceRepository {
        self.instances.clone()
    }

    pub async fn serve(self) -> Result<(), CoreError> {
        tracing::info!(
            core_id = %self.core_id,
            listen_address = %self.listen_address,
            "Core TCP listener is ready"
        );

        loop {
            let (stream, peer_address) = self.listener.accept().await.map_err(CoreError::Accept)?;
            let core_id = self.core_id;
            let pre_shared_key = self.pre_shared_key.clone();
            let instances = self.instances.clone();
            let processes = self.processes.clone();

            spawn(async move {
                if handle_connection(stream, &pre_shared_key, core_id, instances, processes)
                    .await
                    .is_err()
                {
                    tracing::debug!(%peer_address, "Core TCP connection closed during protocol handling");
                }
            });
        }
    }
}

pub async fn run(config: &CoreConfig) -> Result<(), CoreError> {
    CoreServer::bind(config).await?.serve().await
}

async fn handle_connection(
    stream: TcpStream,
    pre_shared_key: &PresharedKey,
    core_id: CoreId,
    instances: InstanceRepository,
    processes: InstanceProcessManager,
) -> Result<(), CoreError> {
    let mut transport = NoiseTransport::accept(stream, pre_shared_key).await?;
    let first_message = transport.read_message().await?;
    let Some((request_id, method, params, _)) = request_parts(first_message) else {
        return Ok(());
    };
    let (response, session_established) =
        session_hello_response(request_id, &method, &params, core_id);

    transport.write_message(&response).await?;
    if !session_established {
        return Ok(());
    }

    let mut request_state = CoreRequestState::new(core_id, instances, processes);
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
                    Ok(event) => transport.write_message(&event).await?,
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
        "instance.create" => instance_create_response(request_id, params, state.instances()),
        "instance.get" => instance_get_response(request_id, params, state.instances()),
        "instance.kill" => {
            instance_kill_response(request_id, params, idempotency_key, state.processes()).await
        }
        "instance.list" => instance_list_response(request_id, params, state.instances()),
        "instance.start" => {
            instance_start_response(request_id, params, idempotency_key, state.processes()).await
        }
        "instance.stop" => {
            instance_stop_response(request_id, params, idempotency_key, state.processes()).await
        }
        "event.subscribe" => event_subscribe_response(request_id, params, state),
        "event.unsubscribe" => event_unsubscribe_response(request_id, params, state),
        _ => error_response(
            request_id,
            "METHOD_NOT_SUPPORTED",
            "The requested Core method is not supported",
        ),
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
    let subscribes_to_instance_state =
        params
            .get("topics")
            .and_then(Value::as_array)
            .is_some_and(|topics| {
                topics
                    .iter()
                    .any(|topic| topic.as_str() == Some("instance.state"))
            });
    if !subscribes_to_instance_state {
        return error_response(
            request_id,
            "BAD_REQUEST",
            "event.subscribe requires the instance.state topic",
        );
    }
    let subscription_id = RequestId::new();
    state.subscribe_to_events(subscription_id);

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
