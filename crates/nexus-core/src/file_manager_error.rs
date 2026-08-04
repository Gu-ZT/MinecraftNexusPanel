use std::io;
use std::path::PathBuf;

use nexus_domain::TaskId;
use thiserror::Error;

/// 文件、目录、归档、传输和配置文档操作错误。
#[derive(Debug, Error)]
pub enum FileManagerError {
    /// 无法规范化 Core 数据目录。
    #[error("failed to canonicalize the Core data directory {path}")]
    CanonicalizeDataDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 无法规范化实例目录。
    #[error("failed to canonicalize the instance directory {path}")]
    CanonicalizeInstanceDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 无法创建实例目录。
    #[error("failed to create the instance directory {path}")]
    CreateInstanceDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 文件系统访问失败。
    #[error("failed to access {operation} path {path}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 相对路径格式不合法。
    #[error("file path is invalid: {path}")]
    InvalidPath { path: String },
    /// 目标路径不存在。
    #[error("file path does not exist: {path}")]
    NotFound { path: PathBuf },
    /// 目标路径不是目录。
    #[error("file path is not a directory: {path}")]
    NotDirectory { path: PathBuf },
    /// 目标路径不是普通文件。
    #[error("file path is not a regular file: {path}")]
    NotFile { path: PathBuf },
    /// 写操作遇到符号链接。
    #[error("symbolic link is not allowed for file writes: {path}")]
    SymlinkNotAllowed { path: PathBuf },
    /// 路径解析后越出实例目录。
    #[error("file path escapes the instance directory: {path}")]
    PathEscapes { path: PathBuf },
    /// 文件内容超过单次操作上限。
    #[error("file content exceeds the maximum size of {maximum_bytes} bytes")]
    ContentTooLarge { maximum_bytes: usize },
    /// 归档超过条目数或源大小上限。
    #[error("file archive exceeds its entry or source size limit")]
    ArchiveTooLarge {
        maximum_entries: usize,
        maximum_bytes: u64,
    },
    /// 提供的 SHA-256 文本格式不合法。
    #[error("file hash is invalid: {value}")]
    InvalidHash { value: String },
    /// 当前文件摘要与调用方期望值不同。
    #[error("file hash does not match the expected value")]
    HashMismatch { expected: String, actual: String },
    /// 文件路径不是有效 UTF-8，无法返回协议条目。
    #[error("file path is not valid UTF-8: {path}")]
    NonUtf8Path { path: PathBuf },
    /// 目标路径已存在且不允许覆盖。
    #[error("file path already exists: {path}")]
    AlreadyExists { path: PathBuf },
    /// 需要覆盖的目录仍包含条目。
    #[error("directory is not empty: {path}")]
    DirectoryNotEmpty { path: PathBuf },
    /// 文件任务状态锁不可用。
    #[error("file task store is unavailable")]
    TaskStorePoisoned,
    /// 传输标识不存在或已结束。
    #[error("file transfer does not exist: {transfer_id}")]
    TransferNotFound { transfer_id: TaskId },
    /// 传输块偏移没有接上上一个块。
    #[error("file transfer offset mismatch: expected {expected}, got {actual}")]
    TransferOffsetMismatch { expected: u64, actual: u64 },
    /// 提交传输时仍未收到声明的全部字节。
    #[error("file transfer is incomplete: expected {expected}, got {actual}")]
    TransferIncomplete { expected: u64, actual: u64 },
    /// 传输总大小与声明值不一致。
    #[error("file transfer size mismatch: expected {expected}, got {actual}")]
    TransferSizeMismatch { expected: u64, actual: u64 },
    /// 完整文件摘要与声明值不一致。
    #[error("file transfer hash does not match the expected value")]
    TransferHashMismatch { expected: String, actual: String },
    /// 当前传输块摘要与声明值不一致。
    #[error("file transfer chunk hash does not match the expected value")]
    TransferChunkHashMismatch { expected: String, actual: String },
    /// 当前传输块超过允许大小。
    #[error("file transfer chunk exceeds the maximum size of {maximum_bytes} bytes")]
    TransferChunkTooLarge { maximum_bytes: usize },
    /// 活跃文件传输数量达到上限。
    #[error("too many active file transfers")]
    TooManyTransfers,
    /// 配置文档标识不存在。
    #[error("configuration document does not exist: {document_id}")]
    ConfigDocumentNotFound { document_id: String },
    /// 配置文件无法解析。
    #[error("configuration document could not be parsed: {path}: {message}")]
    ConfigParse { path: PathBuf, message: String },
    /// 配置补丁不符合文档格式或字段约束。
    #[error("configuration patch is invalid: {message}")]
    ConfigPatchInvalid { message: String },
    /// 配置文件修订号与当前文件不一致。
    #[error("configuration revision does not match the current file")]
    ConfigRevisionMismatch { expected: String, actual: String },
    /// 扫描到的配置文档数量超过上限。
    #[error("too many configuration documents; maximum is {maximum_documents}")]
    ConfigScanTooLarge { maximum_documents: usize },
}
