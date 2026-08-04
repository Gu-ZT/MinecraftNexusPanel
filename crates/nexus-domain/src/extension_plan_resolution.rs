//! 扩展安装计划解析结果。

use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionKind;
use crate::ExtensionPlanItem;

/// 包含根项目和 required 依赖的有界安装计划。
///
/// `kind` 始终属于单独的插件或模组空间，混合端不能在此阶段合并两类
/// 记录；调用方还应根据模板布局选择实际目录。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionPlanResolution {
    template_id: String,
    kind: ExtensionKind,
    minecraft_version: String,
    loader: Option<String>,
    items: Vec<ExtensionPlanItem>,
}

impl ExtensionPlanResolution {
    /// 创建一个已完成解析的扩展计划。
    #[must_use]
    pub fn new(
        template_id: String,
        kind: ExtensionKind,
        minecraft_version: String,
        loader: Option<String>,
        items: Vec<ExtensionPlanItem>,
    ) -> Self {
        Self {
            template_id,
            kind,
            minecraft_version,
            loader,
            items,
        }
    }

    #[must_use]
    pub fn template_id(&self) -> &str {
        &self.template_id
    }

    #[must_use]
    pub const fn kind(&self) -> ExtensionKind {
        self.kind
    }

    #[must_use]
    pub fn minecraft_version(&self) -> &str {
        &self.minecraft_version
    }

    #[must_use]
    pub fn loader(&self) -> Option<&str> {
        self.loader.as_deref()
    }

    #[must_use]
    pub fn items(&self) -> &[ExtensionPlanItem] {
        &self.items
    }
}
