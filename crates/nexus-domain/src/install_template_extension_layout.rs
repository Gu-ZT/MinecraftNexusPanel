//! 安装模板按扩展种类声明的目录布局。

use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionKind;

/// 描述某一种插件或模组可以安装到哪些相对目录。
///
/// 同一 `kind` 可以拥有多个目录，同一个目录也可以被多个种类声明；
/// 因此调用方必须消费模板结果，不能把 `plugins/` 或 `mods/` 写死。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallTemplateExtensionLayout {
    kind: ExtensionKind,
    directories: Vec<String>,
}

impl InstallTemplateExtensionLayout {
    /// 创建一个扩展目录布局声明。
    #[must_use]
    pub fn new(kind: ExtensionKind, directories: Vec<String>) -> Self {
        Self { kind, directories }
    }

    /// 返回该布局对应的扩展种类。
    #[must_use]
    pub const fn kind(&self) -> ExtensionKind {
        self.kind
    }

    /// 返回实例工作目录内的相对目录列表。
    #[must_use]
    pub fn directories(&self) -> &[String] {
        &self.directories
    }
}
