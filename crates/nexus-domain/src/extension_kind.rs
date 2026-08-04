//! 模组和插件的扩展种类。

use serde::Deserialize;
use serde::Serialize;

/// 扩展安装、扫描和审计记录使用的独立种类。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExtensionKind {
    /// 服务端插件，例如 Bukkit/PocketMine 插件。
    Plugin,
    /// 模组，例如 Fabric/Forge 模组。
    Mod,
}
