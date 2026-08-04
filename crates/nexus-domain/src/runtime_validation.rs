//! 运行时可执行文件验证结论。

use serde::Deserialize;
use serde::Serialize;

/// Core 对运行时版本和可执行文件的验证结果。
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeValidation {
    /// 可执行文件和版本检查通过。
    Valid,
    /// 检查失败或版本信息无法信任。
    Invalid,
}
