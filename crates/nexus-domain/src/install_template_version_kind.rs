//! 模板版本目录中版本项的语义。

use serde::Deserialize;
use serde::Serialize;

/// 版本元数据提供方返回的版本层级。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstallTemplateVersionKind {
    /// Minecraft 游戏版本。
    Game,
    /// 模组加载器或其他运行时加载器版本。
    Loader,
    /// 可直接用于服务端安装的版本。
    Server,
}
