use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    refresh_token: String,
}

impl RefreshRequest {
    #[must_use]
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }
}
