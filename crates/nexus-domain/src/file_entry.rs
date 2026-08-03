use serde::Deserialize;
use serde::Serialize;

use crate::FileKind;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    name: String,
    path: String,
    kind: FileKind,
    size: u64,
    modified_at: String,
    sha256: Option<String>,
}

impl FileEntry {
    #[must_use]
    pub fn new(
        name: String,
        path: String,
        kind: FileKind,
        size: u64,
        modified_at: String,
        sha256: Option<String>,
    ) -> Self {
        Self {
            name,
            path,
            kind,
            size,
            modified_at,
            sha256,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub const fn kind(&self) -> FileKind {
        self.kind
    }

    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    #[must_use]
    pub fn modified_at(&self) -> &str {
        &self.modified_at
    }

    #[must_use]
    pub fn sha256(&self) -> Option<&str> {
        self.sha256.as_deref()
    }
}
