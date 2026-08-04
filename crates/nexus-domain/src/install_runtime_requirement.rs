//! 安装模板所需的受管运行时。

use serde::Deserialize;
use serde::Serialize;

/// 模板安装和启动所需的受管运行时类别。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstallRuntimeRequirement {
    /// Java 虚拟机，适用于 Java 服务端和代理。
    Java,
    /// Node.js 运行时。
    NodeJs,
    /// Python 运行时。
    Python,
    /// PHP 运行时，主要用于 PocketMine-MP。
    Php,
    /// 不依赖语言运行时的原生可执行文件。
    Native,
}
