//! 受信下载产物的校验清单。

use serde::Deserialize;
use serde::Serialize;

use crate::DownloadArchitecture;
use crate::DownloadPlatform;
use crate::Sha256Digest;

/// 描述一个必须经过平台、架构、大小和 SHA-256 校验的下载产物。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadManifest {
    url: String,
    size_bytes: u64,
    sha256: Sha256Digest,
    platform: DownloadPlatform,
    architecture: DownloadArchitecture,
}

impl DownloadManifest {
    /// 创建下载清单。
    #[must_use]
    pub fn new(
        url: String,
        size_bytes: u64,
        sha256: Sha256Digest,
        platform: DownloadPlatform,
        architecture: DownloadArchitecture,
    ) -> Self {
        Self {
            url,
            size_bytes,
            sha256,
            platform,
            architecture,
        }
    }

    /// 返回下载地址。
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// 返回来源声明的文件大小，单位为字节。
    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    /// 返回归档 SHA-256 摘要。
    #[must_use]
    pub const fn sha256(&self) -> &Sha256Digest {
        &self.sha256
    }

    /// 返回目标平台。
    #[must_use]
    pub const fn platform(&self) -> DownloadPlatform {
        self.platform
    }

    /// 返回目标 CPU 架构。
    #[must_use]
    pub const fn architecture(&self) -> DownloadArchitecture {
        self.architecture
    }

    /// 判断清单是否适用于当前 Core。
    #[must_use]
    pub fn supports_current_target(&self) -> bool {
        self.platform.is_current() && self.architecture.is_current()
    }
}
