//! 异步任务标识符。

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use uuid::Error as UuidError;
use uuid::Uuid;

/// 标识 Core 或 Panel 中一次可查询的异步操作。
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct TaskId(Uuid);

impl TaskId {
    /// 生成新的 UUIDv7 任务 ID。
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for TaskId {
    type Err = UuidError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::TaskId;

    #[test]
    fn round_trips_through_text() {
        let task_id = TaskId::new();

        assert_eq!(task_id.to_string().parse(), Ok(task_id));
    }
}
