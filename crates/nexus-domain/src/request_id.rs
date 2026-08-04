//! 请求、幂等键和会话关联使用的 UUIDv7 标识符。

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

/// 用于追踪一次请求并传播到 Core、Panel 日志和错误响应的标识符。
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct RequestId(Uuid);

impl RequestId {
    /// 生成一个新的 UUIDv7 请求 ID。
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for RequestId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::RequestId;

    #[test]
    fn round_trips_through_text() {
        let request_id = RequestId::new();

        assert_eq!(request_id.to_string().parse(), Ok(request_id));
    }
}
