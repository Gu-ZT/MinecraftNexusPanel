//! 基岩版 RakNet 健康探测的结果状态。

use serde::Deserialize;
use serde::Serialize;

/// 基岩版 Unconnected Ping/Pong 探测的状态。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BedrockHealthStatus {
    /// 收到并通过魔数、长度和 UTF-8 校验的 Pong。
    Responded,
    /// 在限定时间内没有收到 UDP 响应。
    Unreachable,
    /// 收到数据，但数据不是合法的 RakNet Unconnected Pong。
    InvalidResponse,
    /// Core 无法创建或发送探测套接字。
    Unavailable,
}
