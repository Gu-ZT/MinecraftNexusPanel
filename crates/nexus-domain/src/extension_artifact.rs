//! 可下载的扩展归档元数据。

use serde::Deserialize;
use serde::Serialize;

/// 扩展版本中的一个待下载归档及其摘要。
///
/// 安装器必须使用 HTTPS、大小和强摘要校验；`primary` 只表示来源推荐，
/// 不代表归档已经在本地验证可执行。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionArtifact {
    file_name: String,
    download_url: String,
    size: u64,
    sha1: Option<String>,
    sha512: String,
    primary: bool,
}

impl ExtensionArtifact {
    /// 创建扩展归档描述。
    #[must_use]
    pub fn new(
        file_name: String,
        download_url: String,
        size: u64,
        sha1: Option<String>,
        sha512: String,
        primary: bool,
    ) -> Self {
        Self {
            file_name,
            download_url,
            size,
            sha1,
            sha512,
            primary,
        }
    }

    /// 返回归档文件名。
    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// 返回来源声明的下载地址。
    #[must_use]
    pub fn download_url(&self) -> &str {
        &self.download_url
    }

    /// 返回来源声明的归档大小，单位为字节。
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// 返回可选 SHA-1 摘要；它不能替代 SHA-512 校验。
    #[must_use]
    pub fn sha1(&self) -> Option<&str> {
        self.sha1.as_deref()
    }

    /// 返回用于安装校验的 SHA-512 摘要。
    #[must_use]
    pub fn sha512(&self) -> &str {
        &self.sha512
    }

    /// 表示该归档是否是来源推荐的主文件。
    #[must_use]
    pub const fn primary(&self) -> bool {
        self.primary
    }
}
