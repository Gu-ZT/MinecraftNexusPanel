//! 扩展来源项目的具体版本。

use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionArtifact;
use crate::ExtensionCompatibility;
use crate::ExtensionDependency;

/// 描述一个可供计划解析的扩展版本、依赖和归档文件。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionVersion {
    id: String,
    project_id: String,
    name: String,
    version_number: String,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    dependencies: Vec<ExtensionDependency>,
    artifacts: Vec<ExtensionArtifact>,
    downloads: u64,
    compatibility: ExtensionCompatibility,
}

impl ExtensionVersion {
    /// 创建一个扩展版本描述。
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        project_id: String,
        name: String,
        version_number: String,
        game_versions: Vec<String>,
        loaders: Vec<String>,
        dependencies: Vec<ExtensionDependency>,
        artifacts: Vec<ExtensionArtifact>,
        downloads: u64,
        compatibility: ExtensionCompatibility,
    ) -> Self {
        Self {
            id,
            project_id,
            name,
            version_number,
            game_versions,
            loaders,
            dependencies,
            artifacts,
            downloads,
            compatibility,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn version_number(&self) -> &str {
        &self.version_number
    }

    #[must_use]
    pub fn game_versions(&self) -> &[String] {
        &self.game_versions
    }

    #[must_use]
    pub fn loaders(&self) -> &[String] {
        &self.loaders
    }

    #[must_use]
    pub fn dependencies(&self) -> &[ExtensionDependency] {
        &self.dependencies
    }

    #[must_use]
    pub fn artifacts(&self) -> &[ExtensionArtifact] {
        &self.artifacts
    }

    #[must_use]
    pub const fn downloads(&self) -> u64 {
        self.downloads
    }

    #[must_use]
    pub const fn compatibility(&self) -> ExtensionCompatibility {
        self.compatibility
    }
}
