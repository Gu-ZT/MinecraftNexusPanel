use serde::Deserialize;
use serde::Serialize;

use crate::BedrockManagementKind;
use crate::BedrockTransport;
use crate::ExtensionKind;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockManagementProfile {
    management_kind: BedrockManagementKind,
    transport: BedrockTransport,
    default_port: u16,
    configuration_files: Vec<String>,
    extension_kind: Option<ExtensionKind>,
    extension_directories: Vec<String>,
}

impl BedrockManagementProfile {
    #[must_use]
    pub fn new(
        management_kind: BedrockManagementKind,
        transport: BedrockTransport,
        default_port: u16,
        configuration_files: Vec<String>,
        extension_kind: Option<ExtensionKind>,
    ) -> Self {
        Self {
            management_kind,
            transport,
            default_port,
            configuration_files,
            extension_kind,
            extension_directories: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_extension_directories(mut self, extension_directories: Vec<String>) -> Self {
        self.extension_directories = extension_directories;
        self
    }

    #[must_use]
    pub const fn management_kind(&self) -> BedrockManagementKind {
        self.management_kind
    }

    #[must_use]
    pub const fn transport(&self) -> BedrockTransport {
        self.transport
    }

    #[must_use]
    pub const fn default_port(&self) -> u16 {
        self.default_port
    }

    #[must_use]
    pub fn configuration_files(&self) -> &[String] {
        &self.configuration_files
    }

    #[must_use]
    pub const fn extension_kind(&self) -> Option<ExtensionKind> {
        self.extension_kind
    }

    #[must_use]
    pub fn extension_directories(&self) -> &[String] {
        &self.extension_directories
    }
}
