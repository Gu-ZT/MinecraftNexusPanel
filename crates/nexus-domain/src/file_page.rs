use serde::Deserialize;
use serde::Serialize;

use crate::FileEntry;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePage {
    items: Vec<FileEntry>,
    next_cursor: Option<String>,
}

impl FilePage {
    #[must_use]
    pub fn new(items: Vec<FileEntry>, next_cursor: Option<String>) -> Self {
        Self { items, next_cursor }
    }

    #[must_use]
    pub fn items(&self) -> &[FileEntry] {
        &self.items
    }

    #[must_use]
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}
