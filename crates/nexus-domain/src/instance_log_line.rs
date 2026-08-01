use serde::Deserialize;
use serde::Serialize;

use crate::InstanceLogStream;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceLogLine {
    cursor: String,
    occurred_at: String,
    stream: InstanceLogStream,
    line: String,
}

impl InstanceLogLine {
    #[must_use]
    pub const fn new(
        cursor: String,
        occurred_at: String,
        stream: InstanceLogStream,
        line: String,
    ) -> Self {
        Self {
            cursor,
            occurred_at,
            stream,
            line,
        }
    }

    #[must_use]
    pub fn cursor(&self) -> &str {
        &self.cursor
    }

    #[must_use]
    pub fn occurred_at(&self) -> &str {
        &self.occurred_at
    }

    #[must_use]
    pub const fn stream(&self) -> InstanceLogStream {
        self.stream
    }

    #[must_use]
    pub fn line(&self) -> &str {
        &self.line
    }
}
