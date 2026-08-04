//! 扩展版本列表结果。

use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionVersion;

/// 一个来源项目的版本详情列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionVersionResult {
    source: String,
    project_id: String,
    items: Vec<ExtensionVersion>,
}

impl ExtensionVersionResult {
    /// 创建扩展版本列表结果。
    #[must_use]
    pub fn new(source: String, project_id: String, items: Vec<ExtensionVersion>) -> Self {
        Self {
            source,
            project_id,
            items,
        }
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    #[must_use]
    pub fn items(&self) -> &[ExtensionVersion] {
        &self.items
    }
}
