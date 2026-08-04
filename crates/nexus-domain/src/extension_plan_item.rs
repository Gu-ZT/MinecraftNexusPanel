use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionArtifact;
use crate::ExtensionDependency;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionPlanItem {
    source: String,
    project_id: String,
    version_id: String,
    version_number: String,
    artifact: ExtensionArtifact,
    dependencies: Vec<ExtensionDependency>,
}

impl ExtensionPlanItem {
    #[must_use]
    pub fn new(
        source: String,
        project_id: String,
        version_id: String,
        version_number: String,
        artifact: ExtensionArtifact,
        dependencies: Vec<ExtensionDependency>,
    ) -> Self {
        Self {
            source,
            project_id,
            version_id,
            version_number,
            artifact,
            dependencies,
        }
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    #[must_use]
    pub fn version_id(&self) -> &str {
        &self.version_id
    }

    #[must_use]
    pub fn version_number(&self) -> &str {
        &self.version_number
    }

    #[must_use]
    pub const fn artifact(&self) -> &ExtensionArtifact {
        &self.artifact
    }

    #[must_use]
    pub fn dependencies(&self) -> &[ExtensionDependency] {
        &self.dependencies
    }
}
