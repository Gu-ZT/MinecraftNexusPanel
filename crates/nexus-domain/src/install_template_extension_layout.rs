use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionKind;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallTemplateExtensionLayout {
    kind: ExtensionKind,
    directories: Vec<String>,
}

impl InstallTemplateExtensionLayout {
    #[must_use]
    pub fn new(kind: ExtensionKind, directories: Vec<String>) -> Self {
        Self { kind, directories }
    }

    #[must_use]
    pub const fn kind(&self) -> ExtensionKind {
        self.kind
    }

    #[must_use]
    pub fn directories(&self) -> &[String] {
        &self.directories
    }
}
