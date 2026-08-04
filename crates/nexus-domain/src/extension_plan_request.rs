//! 扩展安装计划解析请求。

use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionKind;

/// 请求为一个扩展项目解析版本、依赖和目标目录计划。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionPlanRequest {
    template_id: String,
    kind: ExtensionKind,
    project_id: String,
    version_id: String,
    minecraft_version: String,
    #[serde(default)]
    loader: Option<String>,
}

impl ExtensionPlanRequest {
    /// 返回目标安装模板 ID。
    #[must_use]
    pub fn template_id(&self) -> &str {
        &self.template_id
    }

    /// 返回插件或模组种类。
    #[must_use]
    pub const fn kind(&self) -> ExtensionKind {
        self.kind
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

    /// 返回目标 Minecraft 版本。
    #[must_use]
    pub fn minecraft_version(&self) -> &str {
        &self.minecraft_version
    }

    /// 返回可选的加载器筛选条件。
    #[must_use]
    pub fn loader(&self) -> Option<&str> {
        self.loader.as_deref()
    }
}
