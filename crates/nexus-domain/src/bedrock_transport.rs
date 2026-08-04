//! 基岩版对外服务传输类型。

use serde::Deserialize;
use serde::Serialize;

/// 基岩版服务使用的网络传输。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BedrockTransport {
    /// RakNet 无连接 UDP 传输。
    RaknetUdp,
}
