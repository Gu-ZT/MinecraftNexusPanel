use serde::Deserialize;
use serde::Serialize;

use crate::DownloadManifest;
use crate::RuntimeArchiveFormat;
use crate::RuntimeKind;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInstallManifest {
    runtime_id: String,
    kind: RuntimeKind,
    distribution: String,
    version: String,
    archive: DownloadManifest,
    archive_format: RuntimeArchiveFormat,
    executable_path: String,
}

impl RuntimeInstallManifest {
    #[must_use]
    pub fn new(
        runtime_id: String,
        kind: RuntimeKind,
        distribution: String,
        version: String,
        archive: DownloadManifest,
        archive_format: RuntimeArchiveFormat,
        executable_path: String,
    ) -> Self {
        Self {
            runtime_id,
            kind,
            distribution,
            version,
            archive,
            archive_format,
            executable_path,
        }
    }

    #[must_use]
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    #[must_use]
    pub const fn kind(&self) -> RuntimeKind {
        self.kind
    }

    #[must_use]
    pub fn distribution(&self) -> &str {
        &self.distribution
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub const fn archive(&self) -> &DownloadManifest {
        &self.archive
    }

    #[must_use]
    pub const fn archive_format(&self) -> RuntimeArchiveFormat {
        self.archive_format
    }

    #[must_use]
    pub fn executable_path(&self) -> &str {
        &self.executable_path
    }
}
