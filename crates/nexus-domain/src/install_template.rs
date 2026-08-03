use serde::Deserialize;
use serde::Serialize;

use crate::InstanceKind;
use crate::RuntimeKind;
use crate::VersionMetadataProvider;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallTemplate {
    id: String,
    name: String,
    instance_kind: InstanceKind,
    required_runtime: RuntimeKind,
    metadata_providers: Vec<VersionMetadataProvider>,
}

impl InstallTemplate {
    #[must_use]
    pub fn new(
        id: String,
        name: String,
        instance_kind: InstanceKind,
        required_runtime: RuntimeKind,
        metadata_providers: Vec<VersionMetadataProvider>,
    ) -> Self {
        Self {
            id,
            name,
            instance_kind,
            required_runtime,
            metadata_providers,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn instance_kind(&self) -> InstanceKind {
        self.instance_kind
    }

    #[must_use]
    pub const fn required_runtime(&self) -> RuntimeKind {
        self.required_runtime
    }

    #[must_use]
    pub fn metadata_providers(&self) -> &[VersionMetadataProvider] {
        &self.metadata_providers
    }
}
