use serde::Deserialize;
use serde::Serialize;

use crate::RuntimeKind;
use crate::RuntimeSource;
use crate::RuntimeValidation;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedRuntime {
    kind: RuntimeKind,
    source: RuntimeSource,
    executable: String,
    version: Option<String>,
    validation: RuntimeValidation,
}

impl ManagedRuntime {
    #[must_use]
    pub fn new(
        kind: RuntimeKind,
        source: RuntimeSource,
        executable: String,
        version: Option<String>,
        validation: RuntimeValidation,
    ) -> Self {
        Self {
            kind,
            source,
            executable,
            version,
            validation,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> RuntimeKind {
        self.kind
    }

    #[must_use]
    pub const fn source(&self) -> RuntimeSource {
        self.source
    }

    #[must_use]
    pub fn executable(&self) -> &str {
        &self.executable
    }

    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    #[must_use]
    pub const fn validation(&self) -> RuntimeValidation {
        self.validation
    }
}
