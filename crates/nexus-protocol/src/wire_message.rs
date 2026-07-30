use nexus_domain::EventId;
use nexus_domain::RequestId;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::WireError;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WireMessage {
    Request {
        #[serde(rename = "requestId")]
        request_id: RequestId,
        method: String,
        params: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        deadline: Option<String>,
        #[serde(
            rename = "idempotencyKey",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        idempotency_key: Option<String>,
    },
    Response {
        #[serde(rename = "requestId")]
        request_id: RequestId,
        ok: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<WireError>,
    },
    Event {
        #[serde(rename = "eventId")]
        event_id: EventId,
        topic: String,
        sequence: u64,
        #[serde(rename = "occurredAt")]
        occurred_at: String,
        data: Value,
    },
}
