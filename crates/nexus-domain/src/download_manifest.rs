use serde::Deserialize;
use serde::Serialize;

use crate::DownloadArchitecture;
use crate::DownloadPlatform;
use crate::Sha256Digest;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadManifest {
    url: String,
    size_bytes: u64,
    sha256: Sha256Digest,
    platform: DownloadPlatform,
    architecture: DownloadArchitecture,
}

impl DownloadManifest {
    #[must_use]
    pub fn new(
        url: String,
        size_bytes: u64,
        sha256: Sha256Digest,
        platform: DownloadPlatform,
        architecture: DownloadArchitecture,
    ) -> Self {
        Self {
            url,
            size_bytes,
            sha256,
            platform,
            architecture,
        }
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    #[must_use]
    pub const fn sha256(&self) -> &Sha256Digest {
        &self.sha256
    }

    #[must_use]
    pub const fn platform(&self) -> DownloadPlatform {
        self.platform
    }

    #[must_use]
    pub const fn architecture(&self) -> DownloadArchitecture {
        self.architecture
    }

    #[must_use]
    pub fn supports_current_target(&self) -> bool {
        self.platform.is_current() && self.architecture.is_current()
    }
}
