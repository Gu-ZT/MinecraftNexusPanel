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

    /// 返回来源项目标识。
    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// 返回来源标识。
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// 返回插件或模组种类。
    #[must_use]
    pub const fn kind(&self) -> ExtensionKind {
        self.kind
    }

    /// 返回项目显示名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回项目摘要。
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// 返回项目详情 URL。
    #[must_use]
    pub fn project_url(&self) -> &str {
        &self.project_url
    }

    /// 返回项目图标 URL。
    #[must_use]
    pub fn icon_url(&self) -> Option<&str> {
        self.icon_url.as_deref()
    }

    /// 返回来源报告的下载量。
    #[must_use]
    pub const fn downloads(&self) -> u64 {
        self.downloads
    }

    /// 返回来源声明支持的 Minecraft 版本。
    #[must_use]
    pub fn supported_minecraft_versions(&self) -> &[String] {
        &self.supported_minecraft_versions
    }

    /// 返回来源声明支持的加载器。
    #[must_use]
    pub fn supported_loaders(&self) -> &[String] {
        &self.supported_loaders
    }

    /// 返回当前筛选条件下的兼容性结论。
    #[must_use]
    pub const fn compatibility(&self) -> ExtensionCompatibility {
        self.compatibility
    }
}
