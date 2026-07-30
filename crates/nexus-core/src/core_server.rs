use std::fs;
use std::net::SocketAddr;
use std::path::Path;

use nexus_config::CoreConfig;
use nexus_domain::CoreId;
use nexus_domain::PRODUCT_VERSION;
use nexus_domain::RequestId;
use nexus_protocol::CURRENT_PROTOCOL_VERSION;
use nexus_protocol::NoiseTransport;
use nexus_protocol::PresharedKey;
use nexus_protocol::ProtocolVersion;
use nexus_protocol::WireError;
use nexus_protocol::WireMessage;
use serde_json::Value;
use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

use crate::CoreError;

const CORE_CAPABILITIES: [&str; 4] = ["events", "files", "metrics", "transfer-v1"];
const CORE_ID_FILE_NAME: &str = "core-id";
const HEARTBEAT_SECONDS: u64 = 20;

pub struct CoreServer {
    core_id: CoreId,
    listen_address: SocketAddr,
    listener: TcpListener,
    pre_shared_key: PresharedKey,
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

        Ok(Self {
            core_id,
            listen_address,
            listener,
            pre_shared_key,
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

            tokio::spawn(async move {
                if handle_connection(stream, &pre_shared_key, core_id)
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
) -> Result<(), CoreError> {
    let mut transport = NoiseTransport::accept(stream, pre_shared_key).await?;
    let first_message = transport.read_message().await?;
    let Some((request_id, method, params)) = request_parts(first_message) else {
        return Ok(());
    };
    let (response, session_established) =
        session_hello_response(request_id, &method, &params, core_id);

    transport.write_message(&response).await?;
    if !session_established {
        return Ok(());
    }

    loop {
        let message = transport.read_message().await?;
        let Some((request_id, method, params)) = request_parts(message) else {
            return Ok(());
        };
        let response = request_response(request_id, &method, &params, core_id);

        transport.write_message(&response).await?;
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

fn request_parts(message: WireMessage) -> Option<(RequestId, String, Value)> {
    match message {
        WireMessage::Request {
            request_id,
            method,
            params,
            ..
        } => Some((request_id, method, params)),
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

fn request_response(
    request_id: RequestId,
    method: &str,
    _params: &Value,
    core_id: CoreId,
) -> WireMessage {
    match method {
        "system.info" => success_response(
            request_id,
            json!({
                "coreId": core_id,
                "serverVersion": PRODUCT_VERSION,
                "protocol": CURRENT_PROTOCOL_VERSION,
                "capabilities": CORE_CAPABILITIES,
            }),
        ),
        "system.ping" => success_response(request_id, json!({ "receivedAt": current_timestamp() })),
        _ => error_response(
            request_id,
            "METHOD_NOT_SUPPORTED",
            "The requested Core method is not supported",
        ),
    }
}

fn protocol_from_hello(params: &Value) -> Option<ProtocolVersion> {
    params
        .get("protocol")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
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
    WireMessage::Response {
        request_id,
        ok: false,
        result: None,
        error: Some(WireError {
            code: code.to_owned(),
            message: message.to_owned(),
            retryable: false,
            details: None,
        }),
    }
}
