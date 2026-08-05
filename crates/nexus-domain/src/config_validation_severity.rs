//! 配置校验诊断的严重级别。

use serde::Deserialize;
use serde::Serialize;

/// 表示配置诊断是否会阻止一次可信的配置应用。
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConfigValidationSeverity {
    /// 配置已经违反可证明的运行约束。
    Error,
    /// 配置值得关注，但不能仅凭当前信息断言启动一定失败。
    Warning,
}

impl ConfigValidationSeverity {
    /// 判断该级别是否属于阻断性错误。
    #[must_use]
    pub const fn is_error(self) -> bool {
        matches!(self, Self::Error)
    }
}
