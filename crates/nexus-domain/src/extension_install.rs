//! 本地产物的扩展安装记录。

use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionKind;

/// 记录已写入实例目录的插件或模组文件及其来源摘要。
///
/// `kind` 和 `path` 必须保持独立，混合端即使暂时共用物理目录，也不能
/// 因路径相同而合并插件和模组的管理记录。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionInstall {
    id: String,
    kind: ExtensionKind,
    path: String,
    sha256: String,
    source: String,
    project_id: Option<String>,
    version: Option<String>,
    installed_at: String,
}

impl ExtensionInstall {
    /// 创建一条本地扩展安装记录。
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        kind: ExtensionKind,
        path: String,
        sha256: String,
        source: String,
        project_id: Option<String>,
        version: Option<String>,
        installed_at: String,
    ) -> Self {
        Self {
            id,
            kind,
            path,
            sha256,
            source,
            project_id,
            version,
            installed_at,
        }
    }

    /// 返回安装记录 ID。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回插件或模组种类。
    #[must_use]
    pub const fn kind(&self) -> ExtensionKind {
        self.kind
    }

    /// 返回实例工作目录内的相对路径。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 返回本地产物的 SHA-256 摘要。
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// 返回来源标识。
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// 返回来源项目标识；本地安装或来源未知时为 `None`。
    #[must_use]
    pub fn project_id(&self) -> Option<&str> {
        self.project_id.as_deref()
    }

    /// 返回来源版本标识；本地安装或来源未知时为 `None`。
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// 返回写入实例目录的时间文本。
    #[must_use]
    pub fn installed_at(&self) -> &str {
        &self.installed_at
    }
}
