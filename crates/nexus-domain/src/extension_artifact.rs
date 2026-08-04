use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionArtifact {
    file_name: String,
    download_url: String,
    size: u64,
    sha1: Option<String>,
    sha512: String,
    primary: bool,
}

impl ExtensionArtifact {
    #[must_use]
    pub fn new(
        file_name: String,
        download_url: String,
        size: u64,
        sha1: Option<String>,
        sha512: String,
        primary: bool,
    ) -> Self {
        Self {
            file_name,
            download_url,
            size,
            sha1,
            sha512,
            primary,
        }
    }

    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    #[must_use]
    pub fn download_url(&self) -> &str {
        &self.download_url
    }

    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    #[must_use]
    pub fn sha1(&self) -> Option<&str> {
        self.sha1.as_deref()
    }

    #[must_use]
    pub fn sha512(&self) -> &str {
        &self.sha512
    }

    #[must_use]
    pub const fn primary(&self) -> bool {
        self.primary
    }
}
