//! 实时事件标识符。

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

/// 标识一次广播到订阅者的领域事件。
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct EventId(Uuid);

impl EventId {
    /// 生成新的 UUIDv7 事件 ID。
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for EventId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::EventId;

    #[test]
    fn round_trips_through_text() {
        let event_id = EventId::new();

        assert_eq!(event_id.to_string().parse(), Ok(event_id));
    }
}
