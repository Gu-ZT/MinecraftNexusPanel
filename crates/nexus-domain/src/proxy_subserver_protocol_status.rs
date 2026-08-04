//! 代理后端 Minecraft Status 协议状态。

use serde::Deserialize;
use serde::Serialize;

/// 代理后端在 TCP 连接之上的 Minecraft Status 响应状态。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProxySubserverProtocolStatus {
    /// 关系被禁用，没有执行协议探测。
    Disabled,
    /// TCP 可连接，但没有可用的协议结果。
    Unavailable,
    /// 收到数据但无法解析为合法 Status 响应。
    InvalidResponse,
    /// 收到并校验了 Minecraft Status JSON 响应。
    Responded,
}
