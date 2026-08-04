use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionProject;

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

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn items(&self) -> &[ExtensionProject] {
        &self.items
    }

    #[must_use]
    pub const fn total(&self) -> u64 {
        self.total
    }

    #[must_use]
    pub const fn limit(&self) -> usize {
        self.limit
    }

    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }
}
