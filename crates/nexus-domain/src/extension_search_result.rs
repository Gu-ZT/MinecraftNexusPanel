//! 扩展项目分页搜索结果。

use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionProject;

/// 一个来源的一页扩展项目及分页游标信息。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionSearchResult {
    source: String,
    items: Vec<ExtensionProject>,
    total: u64,
    limit: usize,
    offset: usize,
}

impl ExtensionSearchResult {
    /// 创建扩展项目分页结果。
    #[must_use]
    pub fn new(
        source: String,
        items: Vec<ExtensionProject>,
        total: u64,
        limit: usize,
        offset: usize,
    ) -> Self {
        Self {
            source,
            items,
            total,
            limit,
            offset,
        }
    }

    /// 返回产生本页结果的来源标识。
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// 返回当前页项目。
    #[must_use]
    pub fn items(&self) -> &[ExtensionProject] {
        &self.items
    }

    /// 返回来源报告的总项目数。
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.total
    }

    /// 返回本页请求的数量上限。
    #[must_use]
    pub const fn limit(&self) -> usize {
        self.limit
    }

    /// 返回本页在来源结果中的偏移量。
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }
}
