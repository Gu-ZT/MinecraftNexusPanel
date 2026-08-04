//! 基岩版监听地址来源。

use serde::Deserialize;
use serde::Serialize;

/// 说明监听地址是从配置读取还是由画像提供。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BedrockBindAddressSource {
    /// 配置文件提供了有效的 IP 字面量。
    Configured,
    /// 配置缺失或无效，使用画像默认地址。
    Default,
}
