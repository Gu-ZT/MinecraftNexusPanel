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

    /// 返回来源版本标识。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回所属项目标识。
    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// 返回版本显示名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回来源版本号。
    #[must_use]
    pub fn version_number(&self) -> &str {
        &self.version_number
    }

    /// 返回该版本支持的 Minecraft 版本。
    #[must_use]
    pub fn game_versions(&self) -> &[String] {
        &self.game_versions
    }

    /// 返回该版本支持的加载器。
    #[must_use]
    pub fn loaders(&self) -> &[String] {
        &self.loaders
    }

    /// 返回依赖声明。
    #[must_use]
    pub fn dependencies(&self) -> &[ExtensionDependency] {
        &self.dependencies
    }

    /// 返回可下载工件列表。
    #[must_use]
    pub fn artifacts(&self) -> &[ExtensionArtifact] {
        &self.artifacts
    }

    /// 返回来源报告的下载量。
    #[must_use]
    pub const fn downloads(&self) -> u64 {
        self.downloads
    }

    /// 返回当前筛选条件下的兼容性结论。
    #[must_use]
    pub const fn compatibility(&self) -> ExtensionCompatibility {
        self.compatibility
    }
}
