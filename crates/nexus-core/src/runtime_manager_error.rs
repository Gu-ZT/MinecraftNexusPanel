use std::io;
use std::path::PathBuf;

use thiserror::Error;

use crate::DownloadError;
use crate::InstanceRepositoryError;

#[derive(Debug, Error)]
pub enum RuntimeManagerError {
    #[error("runtime archive is invalid: {message}")]
    Archive {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
    #[error("failed to {operation} runtime archive path {path}")]
    ArchiveIo {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("runtime {runtime_id} is already installed")]
    AlreadyExists { runtime_id: String },
    #[error("runtime manifest field is invalid: {field}")]
    InvalidManifest { field: &'static str },
    #[error("runtime ID is invalid")]
    InvalidRuntimeId,
    #[error("runtime {runtime_id} is referenced by an instance")]
    InUse { runtime_id: String },
    #[error("runtime {runtime_id} is not installed")]
    NotFound { runtime_id: String },
    #[error("runtime executable is not valid: {path}")]
    InvalidExecutable { path: PathBuf },
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error(transparent)]
    Repository(#[from] InstanceRepositoryError),
    #[error("failed to {operation} runtime path {path}")]
    Storage {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("runtime archive entry is unsafe: {path}")]
    UnsafeArchiveEntry { path: PathBuf },
    #[error("runtime task store is unavailable")]
    TaskStorePoisoned,
}
