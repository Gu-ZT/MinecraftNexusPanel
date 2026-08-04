use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

/// 响应中的稳定错误对象。
///
/// `code` 用于机器判断，`message` 面向日志和界面，`details` 只承载可选结构化
/// 诊断信息；调用方不能仅凭 `message` 分支处理错误。
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireError {
    /// 稳定错误码。
    pub code: String,
    /// 面向操作者的错误说明。
    pub message: String,
    /// 是否适合在相同请求参数下重试。
    pub retryable: bool,
    /// 可选的结构化错误详情。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}
