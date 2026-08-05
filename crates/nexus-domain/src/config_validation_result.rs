//! 实例配置校验的聚合结果。

use serde::Deserialize;
use serde::Serialize;

use crate::ConfigValidationIssue;

/// 汇总一次实例配置校验所读取的文档和诊断。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigValidationResult {
    valid: bool,
    checked_documents: Vec<String>,
    issues: Vec<ConfigValidationIssue>,
}

impl ConfigValidationResult {
    /// 根据已读取文档和诊断创建结果；存在 `ERROR` 时结果无效。
    #[must_use]
    pub fn new(checked_documents: Vec<String>, issues: Vec<ConfigValidationIssue>) -> Self {
        let valid = !issues.iter().any(|issue| issue.severity().is_error());
        Self {
            valid,
            checked_documents,
            issues,
        }
    }

    /// 返回配置是否没有阻断性错误。
    #[must_use]
    pub const fn valid(&self) -> bool {
        self.valid
    }

    /// 返回本次实际读取并参与校验的相对路径。
    #[must_use]
    pub fn checked_documents(&self) -> &[String] {
        &self.checked_documents
    }

    /// 返回按规则产生的诊断列表。
    #[must_use]
    pub fn issues(&self) -> &[ConfigValidationIssue] {
        &self.issues
    }
}
