//! 一键搭建模板的运行家族。

use serde::Deserialize;
use serde::Serialize;

/// 模板对应的服务端运行家族。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstallTemplateFamily {
    /// Java 服务端，包括原版、模组、插件和混合端。
    JavaServer,
    /// Java 代理服务端。
    JavaProxy,
    /// 基岩版独立服务端。
    BedrockServer,
    /// 面向基岩客户端的代理端，例如 Geyser。
    BedrockProxy,
}
