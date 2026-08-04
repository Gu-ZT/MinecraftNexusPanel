//! 基岩版服务端实现和 Geyser 的管理分类。

use serde::Deserialize;
use serde::Serialize;

/// 基岩端的配置和扩展管理策略分类。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BedrockManagementKind {
    /// Mojang Bedrock Dedicated Server，不提供插件目录。
    DedicatedServer,
    /// PocketMine-MP，使用 PHP 运行时并支持插件。
    PocketMine,
    /// Nukkit 或 Cloudburst Nukkit，使用 Java 运行时并支持插件。
    Nukkit,
    /// Geyser 基岩代理，管理一个 Java 后端而不是插件目录。
    Geyser,
}
