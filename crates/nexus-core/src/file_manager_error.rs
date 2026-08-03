use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileManagerError {
    #[error("failed to canonicalize the Core data directory {path}")]
    CanonicalizeDataDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to canonicalize the instance directory {path}")]
    CanonicalizeInstanceDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to create the instance directory {path}")]
    CreateInstanceDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to access {operation} path {path}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("file path is invalid: {path}")]
    InvalidPath { path: String },
    #[error("file path does not exist: {path}")]
    NotFound { path: PathBuf },
    #[error("file path is not a directory: {path}")]
    NotDirectory { path: PathBuf },
    #[error("file path is not a regular file: {path}")]
    NotFile { path: PathBuf },
    #[error("symbolic link is not allowed for file writes: {path}")]
    SymlinkNotAllowed { path: PathBuf },
    #[error("file path escapes the instance directory: {path}")]
    PathEscapes { path: PathBuf },
    #[error("file content exceeds the maximum size of {maximum_bytes} bytes")]
    ContentTooLarge { maximum_bytes: usize },
    #[error("file hash is invalid: {value}")]
    InvalidHash { value: String },
    #[error("file hash does not match the expected value")]
    HashMismatch { expected: String, actual: String },
    #[error("file path is not valid UTF-8: {path}")]
    NonUtf8Path { path: PathBuf },
    #[error("file path already exists: {path}")]
    AlreadyExists { path: PathBuf },
    #[error("directory is not empty: {path}")]
    DirectoryNotEmpty { path: PathBuf },
}
