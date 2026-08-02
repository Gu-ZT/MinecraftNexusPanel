use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceCommandRequest {
    command: String,
}

impl InstanceCommandRequest {
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
}
