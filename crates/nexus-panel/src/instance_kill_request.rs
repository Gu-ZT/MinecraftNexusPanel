use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceKillRequest {
    confirmation: String,
}

impl InstanceKillRequest {
    #[must_use]
    pub fn confirmation(&self) -> &str {
        &self.confirmation
    }
}
