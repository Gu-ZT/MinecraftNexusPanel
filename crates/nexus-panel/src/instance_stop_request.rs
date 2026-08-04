use serde::Deserialize;

/// 实例优雅停止请求体。
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceStopRequest {
    timeout_seconds: Option<u16>,
}

impl InstanceStopRequest {
    /// 返回可选的停止超时秒数。
    #[must_use]
    pub const fn timeout_seconds(&self) -> Option<u16> {
        self.timeout_seconds
    }
}
