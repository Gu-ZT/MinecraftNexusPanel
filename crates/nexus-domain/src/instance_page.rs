use serde::Deserialize;
use serde::Serialize;

use crate::Instance;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstancePage {
    items: Vec<Instance>,
    next_cursor: Option<String>,
}

impl InstancePage {
    #[must_use]
    pub fn new(items: Vec<Instance>, next_cursor: Option<String>) -> Self {
        Self { items, next_cursor }
    }

    #[must_use]
    pub fn items(&self) -> &[Instance] {
        &self.items
    }

    #[must_use]
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}
