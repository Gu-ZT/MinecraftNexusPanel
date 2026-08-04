//! 文件列表分页结果。

use serde::Deserialize;
use serde::Serialize;

use crate::FileEntry;

/// Core 文件列表的一页及下一页游标。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePage {
    items: Vec<FileEntry>,
    next_cursor: Option<String>,
}

impl FilePage {
    /// 创建文件列表分页结果。
    #[must_use]
    pub fn new(items: Vec<FileEntry>, next_cursor: Option<String>) -> Self {
        Self { items, next_cursor }
    }

    /// 返回当前页条目。
    #[must_use]
    pub fn items(&self) -> &[FileEntry] {
        &self.items
    }

    /// 返回下一页游标。
    #[must_use]
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}
