//! Core 节点的稳定 UUIDv7 标识符。

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

/// Core 节点身份标识；首次创建后应持久化并跨重启保持不变。
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CoreId(Uuid);

impl CoreId {
    /// 生成新的 UUIDv7 Core 标识符。
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for CoreId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CoreId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for CoreId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::CoreId;

    #[test]
    fn round_trips_through_text() {
        let core_id = CoreId::new();

        assert_eq!(core_id.to_string().parse(), Ok(core_id));
    }
}
