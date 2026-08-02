use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceStopRequest {
    timeout_seconds: Option<u16>,
}

impl InstanceStopRequest {
    #[must_use]
    pub const fn timeout_seconds(&self) -> Option<u16> {
        self.timeout_seconds
    }
}
