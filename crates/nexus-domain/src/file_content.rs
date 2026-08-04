//! 文件分块读取结果。

use serde::Deserialize;
use serde::Serialize;

/// 一个 Base64 编码的文件内容分块及其摘要。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    data_base64: String,
    sha256: String,
    eof: bool,
}

impl FileContent {
    /// 创建文件读取分块结果。
    #[must_use]
    pub fn new(data_base64: String, sha256: String, eof: bool) -> Self {
        Self {
            data_base64,
            sha256,
            eof,
        }
    }

    /// 返回 Base64 编码的数据。
    #[must_use]
    pub fn data_base64(&self) -> &str {
        &self.data_base64
    }

    /// 返回该分块内容的 SHA-256 摘要。
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// 表示该分块是否已经到达文件末尾。
    #[must_use]
    pub const fn eof(&self) -> bool {
        self.eof
    }
}
