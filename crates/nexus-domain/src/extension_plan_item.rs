//! 解析后的扩展安装计划条目。

use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionArtifact;
use crate::ExtensionDependency;

/// 一个根项目或 required 依赖最终选定的可安装文件。
///
/// 计划解析只负责选择和校验元数据；真正安装仍需再次解析、下载和校验
/// 归档，避免使用过期或被篡改的计划。
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
    /// 创建一个扩展安装计划条目。
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

    /// 返回扩展来源标识。
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// 返回来源项目标识。
    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// 返回来源版本标识。
    #[must_use]
    pub fn version_id(&self) -> &str {
        &self.version_id
    }

    /// 返回来源版本号。
    #[must_use]
    pub fn version_number(&self) -> &str {
        &self.version_number
    }

    /// 返回计划选定的下载工件。
    #[must_use]
    pub const fn artifact(&self) -> &ExtensionArtifact {
        &self.artifact
    }

    /// 返回该版本声明的依赖摘要。
    #[must_use]
    pub fn dependencies(&self) -> &[ExtensionDependency] {
        &self.dependencies
    }
}
