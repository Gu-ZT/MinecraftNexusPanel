use std::io;
use std::path::PathBuf;

use nexus_domain::DownloadArchitecture;
use nexus_domain::DownloadPlatform;
use thiserror::Error;

/// 下载任务、目标校验或缓存操作失败。
#[derive(Debug, Error)]
pub enum DownloadError {
    /// 调用方取消了下载任务。
    #[error("download task was cancelled")]
    Cancelled,
    /// HTTP 客户端初始化失败。
    #[error("failed to create the download HTTP client")]
    Client(#[source] reqwest::Error),
    /// HTTP 响应声明的大小与下载清单不一致。
    #[error(
        "download response size does not match the manifest: expected {expected_bytes} bytes, got {actual_bytes} bytes"
    )]
    ContentLengthMismatch {
        expected_bytes: u64,
        actual_bytes: u64,
    },
    /// 清单目标 CPU 架构不适用于当前 Core。
    #[error("download manifest architecture {architecture:?} is not supported by this Core")]
    UnsupportedArchitecture { architecture: DownloadArchitecture },
    /// 清单目标操作系统平台不适用于当前 Core。
    #[error("download manifest platform {platform:?} is not supported by this Core")]
    UnsupportedPlatform { platform: DownloadPlatform },
    /// 生产下载地址不是 HTTPS。
    #[error("download URL must use HTTPS")]
    InsecureUrl,
    /// 下载地址包含用户名或密码。
    #[error("download URL must not include credentials")]
    UrlContainsCredentials,
    /// 下载地址无法解析或没有主机。
    #[error("download URL is invalid: {url}")]
    InvalidUrl { url: String },
    /// HTTP 请求或状态码处理失败。
    #[error("download request failed")]
    Request(#[source] reqwest::Error),
    /// 下载缓存文件读写失败。
    #[error("failed to {operation} download cache file {path}")]
    Storage {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 实际下载字节数与清单不一致。
    #[error(
        "downloaded size does not match the manifest: expected {expected_bytes} bytes, got {actual_bytes} bytes"
    )]
    SizeMismatch {
        expected_bytes: u64,
        actual_bytes: u64,
    },
    /// 实际 SHA-256 与清单摘要不一致。
    #[error("downloaded SHA-256 does not match the manifest")]
    Sha256Mismatch { expected: String, actual: String },
}
