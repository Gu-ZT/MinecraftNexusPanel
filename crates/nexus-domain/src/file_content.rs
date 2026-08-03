use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    data_base64: String,
    sha256: String,
    eof: bool,
}

impl FileContent {
    #[must_use]
    pub fn new(data_base64: String, sha256: String, eof: bool) -> Self {
        Self {
            data_base64,
            sha256,
            eof,
        }
    }

    #[must_use]
    pub fn data_base64(&self) -> &str {
        &self.data_base64
    }

    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    #[must_use]
    pub const fn eof(&self) -> bool {
        self.eof
    }
}
