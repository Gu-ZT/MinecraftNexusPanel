//! 基岩版 UDP 端口来源。

use serde::Deserialize;
use serde::Serialize;

/// 说明 UDP 端口是从配置读取还是由画像提供。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BedrockPortSource {
    /// 配置文件提供了有效端口。
    Configured,
    /// 配置缺失或无效，使用画像默认端口。
    Default,
}
