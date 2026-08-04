use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionDependency {
    project_id: Option<String>,
    version_id: Option<String>,
    file_name: Option<String>,
    dependency_type: String,
}

impl ExtensionDependency {
    #[must_use]
    pub fn new(
        project_id: Option<String>,
        version_id: Option<String>,
        file_name: Option<String>,
        dependency_type: String,
    ) -> Self {
        Self {
            project_id,
            version_id,
            file_name,
            dependency_type,
        }
    }

    #[must_use]
    pub fn project_id(&self) -> Option<&str> {
        self.project_id.as_deref()
    }

    #[must_use]
    pub fn version_id(&self) -> Option<&str> {
        self.version_id.as_deref()
    }

    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

    #[must_use]
    pub fn dependency_type(&self) -> &str {
        &self.dependency_type
    }
}
