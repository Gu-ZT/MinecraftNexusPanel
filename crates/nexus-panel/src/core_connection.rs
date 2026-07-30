use std::net::SocketAddr;

use nexus_domain::CoreId;
use nexus_domain::PRODUCT_VERSION;
use nexus_domain::RequestId;
use nexus_protocol::CURRENT_PROTOCOL_VERSION;
use nexus_protocol::NoiseTransport;
use nexus_protocol::PresharedKey;
use nexus_protocol::ProtocolVersion;
use nexus_protocol::WireMessage;
use serde_json::Value;
use serde_json::json;
use tokio::net::TcpStream;

use crate::CoreConnectionError;

const PANEL_CAPABILITIES: [&str; 2] = ["events", "transfer-v1"];

pub struct CoreConnection {
    capabilities: Vec<String>,
    core_id: CoreId,
    heartbeat_seconds: u64,
    protocol: ProtocolVersion,
    transport: NoiseTransport<TcpStream>,
}

impl CoreConnection {
    pub async fn connect(
        address: SocketAddr,
        pre_shared_key: &PresharedKey,
        panel_id: &str,
        panel_name: &str,
    ) -> Result<Self, CoreConnectionError> {
        let stream = TcpStream::connect(address)
            .await
            .map_err(|source| CoreConnectionError::Connect { address, source })?;
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
        let protocol = serde_json::from_value(protocol)
            .map_err(|_| CoreConnectionError::InvalidResponse { field: "protocol" })?;
        let protocol = CURRENT_PROTOCOL_VERSION.negotiate(protocol)?;
        let core_id = response_field(&welcome, "coreId")?;
        let core_id = serde_json::from_value(core_id)
            .map_err(|_| CoreConnectionError::InvalidResponse { field: "coreId" })?;
        let capabilities = response_field(&welcome, "capabilities")?;
        let capabilities = serde_json::from_value(capabilities).map_err(|_| {
            CoreConnectionError::InvalidResponse {
                field: "capabilities",
            }
        })?;
        let heartbeat_seconds = response_field(&welcome, "heartbeatSeconds")?
            .as_u64()
            .ok_or(CoreConnectionError::InvalidResponse {
                field: "heartbeatSeconds",
            })?;

        Ok(Self {
            capabilities,
            core_id,
            heartbeat_seconds,
            protocol,
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

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, CoreConnectionError> {
        let request_id = RequestId::new();
        let request = WireMessage::Request {
            request_id,
            method: method.to_owned(),
            params,
            deadline: None,
            idempotency_key: None,
        };

        self.transport.write_message(&request).await?;

        response_result(self.transport.read_message().await?, request_id)
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
