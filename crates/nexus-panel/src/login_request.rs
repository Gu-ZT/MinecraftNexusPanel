use serde::Deserialize;
use serde_json::Value;

use crate::ClientType;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    username: String,
    password: String,
    client_type: ClientType,
    #[serde(rename = "device")]
    _device: Option<Value>,
    #[serde(rename = "mfaCode")]
    _mfa_code: Option<String>,
}

impl LoginRequest {
    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    #[must_use]
    pub fn password(&self) -> &str {
        &self.password
    }

    #[must_use]
    pub const fn client_type(&self) -> ClientType {
        self.client_type
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.username.is_empty()
            && self.username.chars().count() <= 64
            && !self.password.is_empty()
            && self.password.len() <= 1024
    }
}
