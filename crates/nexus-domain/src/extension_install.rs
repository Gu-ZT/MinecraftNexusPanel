use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionKind;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionInstall {
    id: String,
    kind: ExtensionKind,
    path: String,
    sha256: String,
    source: String,
    project_id: Option<String>,
    version: Option<String>,
    installed_at: String,
}

impl ExtensionInstall {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        kind: ExtensionKind,
        path: String,
        sha256: String,
        source: String,
        project_id: Option<String>,
        version: Option<String>,
        installed_at: String,
    ) -> Self {
        Self {
            id,
            kind,
            path,
            sha256,
            source,
            project_id,
            version,
            installed_at,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub const fn kind(&self) -> ExtensionKind {
        self.kind
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn project_id(&self) -> Option<&str> {
        self.project_id.as_deref()
    }

    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    #[must_use]
    pub fn installed_at(&self) -> &str {
        &self.installed_at
    }
}
