//! 运行时安装归档格式。

use serde::Deserialize;
use serde::Serialize;

/// Core 安装运行时时支持的归档格式。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeArchiveFormat {
    /// gzip 压缩的 tar 归档。
    TarGz,
    /// ZIP 归档。
    Zip,
}
