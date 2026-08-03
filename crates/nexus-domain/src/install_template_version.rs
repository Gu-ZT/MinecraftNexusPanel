use serde::Deserialize;
use serde::Serialize;

use crate::InstallTemplateVersionKind;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallTemplateVersion {
    id: String,
    provider_id: String,
    kind: InstallTemplateVersionKind,
    stable: bool,
    metadata_url: Option<String>,
}

impl InstallTemplateVersion {
    #[must_use]
    pub fn new(
        id: String,
        provider_id: String,
        kind: InstallTemplateVersionKind,
        stable: bool,
        metadata_url: Option<String>,
    ) -> Self {
        Self {
            id,
            provider_id,
            kind,
            stable,
            metadata_url,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    #[must_use]
    pub const fn kind(&self) -> InstallTemplateVersionKind {
        self.kind
    }

    #[must_use]
    pub const fn stable(&self) -> bool {
        self.stable
    }

    #[must_use]
    pub fn metadata_url(&self) -> Option<&str> {
        self.metadata_url.as_deref()
    }
}
