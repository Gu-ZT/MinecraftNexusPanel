use nexus_domain::EventId;
use nexus_domain::RequestId;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::WireError;

/// MCNP 应用层 JSON 消息的三种顶层形态。
///
/// 请求和响应通过 `requestId` 关联；事件使用独立 `eventId`、主题和单调序号，
/// 不应被当作请求响应处理。字段的具体业务结构位于 `params`、`result` 或 `data` 中。
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WireMessage {
    /// 请求 Core 或 Panel 执行一个方法。
    Request {
        /// 与响应关联的请求标识。
        #[serde(rename = "requestId")]
        request_id: RequestId,
        /// 稳定的方法名。
        method: String,
        /// 方法参数对象或值。
        params: Value,
        /// 可选的 RFC 3339 截止时间。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        deadline: Option<String>,
        /// 可选的幂等键，用于重试不重复执行副作用。
        #[serde(
            rename = "idempotencyKey",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        idempotency_key: Option<String>,
    },
    /// 对请求的成功或失败响应。
    Response {
        /// 被响应的请求标识。
        #[serde(rename = "requestId")]
        request_id: RequestId,
        /// 表示响应是否成功；失败时通常同时提供 `error`。
        ok: bool,
        /// 成功结果。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<Value>,
        /// 失败错误。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<WireError>,
    },
    /// 推送给订阅方的实时领域事件。
    Event {
        /// 事件标识。
        #[serde(rename = "eventId")]
        event_id: EventId,
        /// 订阅主题。
        topic: String,
        /// 主题内的递增序号，用于检测丢失事件。
        sequence: u64,
        /// 事件发生时间。
        #[serde(rename = "occurredAt")]
        occurred_at: String,
        /// 事件负载。
        data: Value,
    },
}
