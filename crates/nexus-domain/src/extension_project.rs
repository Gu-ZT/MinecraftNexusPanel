//! 扩展来源项目搜索结果。

use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionCompatibility;
use crate::ExtensionKind;

/// 聚合来源返回的一个插件或模组项目。
///
/// `compatibility` 只表示当前请求筛选条件下的来源元数据结论，
/// 不能替代对具体版本、归档摘要和运行时行为的验证。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionProject {
    project_id: String,
    source: String,
    kind: ExtensionKind,
    name: String,
    summary: String,
    project_url: String,
    icon_url: Option<String>,
    downloads: u64,
    supported_minecraft_versions: Vec<String>,
    supported_loaders: Vec<String>,
    compatibility: ExtensionCompatibility,
}

impl ExtensionProject {
    /// 创建一个来源项目摘要。
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: String,
        source: String,
        kind: ExtensionKind,
        name: String,
        summary: String,
        project_url: String,
        icon_url: Option<String>,
        downloads: u64,
        supported_minecraft_versions: Vec<String>,
        supported_loaders: Vec<String>,
        compatibility: ExtensionCompatibility,
    ) -> Self {
        Self {
            project_id,
            source,
            kind,
            name,
            summary,
            project_url,
            icon_url,
            downloads,
            supported_minecraft_versions,
            supported_loaders,
            compatibility,
        }
    }

    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub const fn kind(&self) -> ExtensionKind {
        self.kind
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    #[must_use]
    pub fn project_url(&self) -> &str {
        &self.project_url
    }

    #[must_use]
    pub fn icon_url(&self) -> Option<&str> {
        self.icon_url.as_deref()
    }

    #[must_use]
    pub const fn downloads(&self) -> u64 {
        self.downloads
    }

    #[must_use]
    pub fn supported_minecraft_versions(&self) -> &[String] {
        &self.supported_minecraft_versions
    }

    #[must_use]
    pub fn supported_loaders(&self) -> &[String] {
        &self.supported_loaders
    }

    #[must_use]
    pub const fn compatibility(&self) -> ExtensionCompatibility {
        self.compatibility
    }
}
