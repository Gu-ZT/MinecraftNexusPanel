use serde::Deserialize;
use serde::Serialize;

use crate::RuntimeKind;
use crate::RuntimeSource;
use crate::RuntimeValidation;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedRuntime {
    runtime_id: Option<String>,
    kind: RuntimeKind,
    source: RuntimeSource,
    distribution: Option<String>,
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
            runtime_id: None,
            kind,
            source,
            distribution: None,
            executable,
            version,
            validation,
        }
    }

    #[must_use]
    pub fn managed(
        runtime_id: String,
        kind: RuntimeKind,
        distribution: String,
        executable: String,
        version: Option<String>,
        validation: RuntimeValidation,
    ) -> Self {
        Self {
            runtime_id: Some(runtime_id),
            kind,
            source: RuntimeSource::Managed,
            distribution: Some(distribution),
            executable,
            version,
            validation,
        }
    }

    #[must_use]
    pub fn runtime_id(&self) -> Option<&str> {
        self.runtime_id.as_deref()
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
    pub fn distribution(&self) -> Option<&str> {
        self.distribution.as_deref()
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
