//! 扩展版本之间的依赖关系。

use serde::Deserialize;
use serde::Serialize;

/// 描述一个来源扩展声明的依赖项目、版本或归档文件。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionDependency {
    project_id: Option<String>,
    version_id: Option<String>,
    file_name: Option<String>,
    dependency_type: String,
}

impl ExtensionDependency {
    /// 创建一条依赖记录；缺少项目或版本时保留来源的原始信息。
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

    /// 返回依赖项目 ID。
    #[must_use]
    pub fn project_id(&self) -> Option<&str> {
        self.project_id.as_deref()
    }

    /// 返回依赖版本 ID。
    #[must_use]
    pub fn version_id(&self) -> Option<&str> {
        self.version_id.as_deref()
    }

    /// 返回依赖文件名。
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

    /// 返回来源定义的依赖类型，例如 required 或 optional。
    #[must_use]
    pub fn dependency_type(&self) -> &str {
        &self.dependency_type
    }
}
