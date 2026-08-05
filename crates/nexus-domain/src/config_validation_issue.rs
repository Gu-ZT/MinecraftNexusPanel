//! 单条配置校验诊断。

use serde::Deserialize;
use serde::Serialize;

use crate::ConfigValidationSeverity;

/// 描述一个配置文件、字段以及可选关联字段上的校验结果。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigValidationIssue {
    code: String,
    severity: ConfigValidationSeverity,
    path: String,
    field: Option<String>,
    message: String,
    related_path: Option<String>,
    related_field: Option<String>,
}

impl ConfigValidationIssue {
    /// 创建一条带可选关联位置的诊断。
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        code: String,
        severity: ConfigValidationSeverity,
        path: String,
        field: Option<String>,
        message: String,
        related_path: Option<String>,
        related_field: Option<String>,
    ) -> Self {
        Self {
            code,
            severity,
            path,
            field,
            message,
            related_path,
            related_field,
        }
    }

    /// 返回稳定的机器可读诊断代码。
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// 返回诊断严重级别。
    #[must_use]
    pub const fn severity(&self) -> ConfigValidationSeverity {
        self.severity
    }

    /// 返回主配置文件相对路径。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 返回主配置字段路径；文件级诊断为 `None`。
    #[must_use]
    pub fn field(&self) -> Option<&str> {
        self.field.as_deref()
    }

    /// 返回适合用户显示的诊断消息。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 返回关联配置文件路径，例如端口冲突的另一侧。
    #[must_use]
    pub fn related_path(&self) -> Option<&str> {
        self.related_path.as_deref()
    }

    /// 返回关联配置字段路径。
    #[must_use]
    pub fn related_field(&self) -> Option<&str> {
        self.related_field.as_deref()
    }
}
