//! 基岩版 UDP 端口绑定状态。

use serde::Deserialize;
use serde::Serialize;

/// Core 绑定探测的结果。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BedrockPortCheckState {
    /// 端口当前可以被 Core 绑定。
    Available,
    /// 端口已被其他套接字占用。
    InUse,
    /// 因权限、地址或系统错误无法绑定。
    Unavailable,
}
