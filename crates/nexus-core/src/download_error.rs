use std::io;
use std::path::PathBuf;

use nexus_domain::DownloadArchitecture;
use nexus_domain::DownloadPlatform;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("download task was cancelled")]
    Cancelled,
    #[error("failed to create the download HTTP client")]
    Client(#[source] reqwest::Error),
    #[error(
        "download response size does not match the manifest: expected {expected_bytes} bytes, got {actual_bytes} bytes"
    )]
    ContentLengthMismatch {
        expected_bytes: u64,
        actual_bytes: u64,
    },
    #[error("download manifest architecture {architecture:?} is not supported by this Core")]
    UnsupportedArchitecture { architecture: DownloadArchitecture },
    #[error("download manifest platform {platform:?} is not supported by this Core")]
    UnsupportedPlatform { platform: DownloadPlatform },
    #[error("download URL must use HTTPS")]
    InsecureUrl,
    #[error("download URL must not include credentials")]
    UrlContainsCredentials,
    #[error("download URL is invalid: {url}")]
    InvalidUrl { url: String },
    #[error("download request failed")]
    Request(#[source] reqwest::Error),
    #[error("failed to {operation} download cache file {path}")]
    Storage {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(
        "downloaded size does not match the manifest: expected {expected_bytes} bytes, got {actual_bytes} bytes"
    )]
    SizeMismatch {
        expected_bytes: u64,
        actual_bytes: u64,
    },
    #[error("downloaded SHA-256 does not match the manifest")]
    Sha256Mismatch { expected: String, actual: String },
}
