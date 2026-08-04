//! 文件列表中的安全条目描述。

use serde::Deserialize;
use serde::Serialize;

use crate::FileKind;

/// 以实例工作目录为根的相对文件条目。
///
/// 路径由 Core 规范化并校验，Panel 不应把它当作宿主机绝对路径处理。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    name: String,
    path: String,
    kind: FileKind,
    size: u64,
    modified_at: String,
    sha256: Option<String>,
}

impl FileEntry {
    /// 创建文件条目描述。
    #[must_use]
    pub fn new(
        name: String,
        path: String,
        kind: FileKind,
        size: u64,
        modified_at: String,
        sha256: Option<String>,
    ) -> Self {
        Self {
            name,
            path,
            kind,
            size,
            modified_at,
            sha256,
        }
    }

    /// 返回条目名称，不包含父目录路径。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回以实例目录为根的规范化相对路径。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 返回文件系统条目类型。
    #[must_use]
    pub const fn kind(&self) -> FileKind {
        self.kind
    }

    /// 返回文件大小，目录大小由 Core 的文件系统实现定义。
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// 返回 Core 提供的最后修改时间。
    #[must_use]
    pub fn modified_at(&self) -> &str {
        &self.modified_at
    }

    /// 返回文件摘要；目录、链接或未计算摘要时为 `None`。
    #[must_use]
    pub fn sha256(&self) -> Option<&str> {
        self.sha256.as_deref()
    }
}
