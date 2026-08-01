use serde::Deserialize;
use serde::Serialize;

use crate::InstanceLogLine;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceLogPage {
    event_cursor: String,
    items: Vec<InstanceLogLine>,
    next_cursor: Option<String>,
}

impl InstanceLogPage {
    #[must_use]
    pub const fn new(
        items: Vec<InstanceLogLine>,
        next_cursor: Option<String>,
        event_cursor: String,
    ) -> Self {
        Self {
            event_cursor,
            items,
            next_cursor,
        }
    }

    #[must_use]
    pub fn event_cursor(&self) -> &str {
        &self.event_cursor
    }

    #[must_use]
    pub fn items(&self) -> &[InstanceLogLine] {
        &self.items
    }

    #[must_use]
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}
