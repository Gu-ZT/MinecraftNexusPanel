//! Core 文件沙箱中的条目类型。

use serde::Deserialize;
use serde::Serialize;

/// 文件列表返回的文件系统条目类型。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileKind {
    /// 普通文件。
    File,
    /// 目录。
    Directory,
    /// 符号链接；Core 对其执行额外安全检查。
    Symlink,
    /// Core 无法归类的其他文件系统条目。
    Other,
}
