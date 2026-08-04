//! 运行时来源分类。

use serde::Deserialize;
use serde::Serialize;

/// 运行时来自系统 PATH 还是 MCNP 受管目录。
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeSource {
    /// 安装在 MCNP 管理目录中。
    Managed,
    /// 从系统环境发现，不由 MCNP 负责删除。
    System,
}
