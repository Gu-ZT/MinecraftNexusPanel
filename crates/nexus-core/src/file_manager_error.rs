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
        /// 需要规范化的 Core 数据目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的规范化错误。
        source: io::Error,
    },
    /// 无法规范化实例目录。
    #[error("failed to canonicalize the instance directory {path}")]
    CanonicalizeInstanceDirectory {
        /// 需要规范化的实例目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的规范化错误。
        source: io::Error,
    },
    /// 无法创建实例目录。
    #[error("failed to create the instance directory {path}")]
    CreateInstanceDirectory {
        /// 创建失败的实例目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的目录创建错误。
        source: io::Error,
    },
    /// 文件系统访问失败。
    #[error("failed to access {operation} path {path}")]
    Io {
        /// 失败的文件系统操作名称。
        operation: &'static str,
        /// 发生错误的路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的文件错误。
        source: io::Error,
    },
    /// 相对路径格式不合法。
    #[error("file path is invalid: {path}")]
    InvalidPath {
        /// 调用方提交的原始相对路径文本。
        path: String,
    },
    /// 目标路径不存在。
    #[error("file path does not exist: {path}")]
    NotFound {
        /// 不存在的目标路径。
        path: PathBuf,
    },
    /// 目标路径不是目录。
    #[error("file path is not a directory: {path}")]
    NotDirectory {
        /// 预期为目录但实际不是目录的路径。
        path: PathBuf,
    },
    /// 目标路径不是普通文件。
    #[error("file path is not a regular file: {path}")]
    NotFile {
        /// 预期为普通文件但实际不是文件的路径。
        path: PathBuf,
    },
    /// 写操作遇到符号链接。
    #[error("symbolic link is not allowed for file writes: {path}")]
    SymlinkNotAllowed {
        /// 被拒绝访问的符号链接路径。
        path: PathBuf,
    },
    /// 路径解析后越出实例目录。
    #[error("file path escapes the instance directory: {path}")]
    PathEscapes {
        /// 解析后落在实例根目录之外的路径。
        path: PathBuf,
    },
    /// 文件内容超过单次操作上限。
    #[error("file content exceeds the maximum size of {maximum_bytes} bytes")]
    ContentTooLarge {
        /// 当前文件操作允许的最大字节数。
        maximum_bytes: usize,
    },
    /// 归档超过条目数或源大小上限。
    #[error("file archive exceeds its entry or source size limit")]
    ArchiveTooLarge {
        /// 归档允许包含的最大条目数。
        maximum_entries: usize,
        /// 归档源文件允许的最大字节数。
        maximum_bytes: u64,
    },
    /// 提供的 SHA-256 文本格式不合法。
    #[error("file hash is invalid: {value}")]
    InvalidHash {
        /// 调用方提交的原始摘要文本。
        value: String,
    },
    /// 当前文件摘要与调用方期望值不同。
    #[error("file hash does not match the expected value")]
    HashMismatch {
        /// 调用方声明的文件摘要。
        expected: String,
        /// 实际文件内容计算出的摘要。
        actual: String,
    },
    /// 文件路径不是有效 UTF-8，无法返回协议条目。
    #[error("file path is not valid UTF-8: {path}")]
    NonUtf8Path {
        /// 无法编码为协议文本的文件路径。
        path: PathBuf,
    },
    /// 目标路径已存在且不允许覆盖。
    #[error("file path already exists: {path}")]
    AlreadyExists {
        /// 已存在的目标路径。
        path: PathBuf,
    },
    /// 需要覆盖的目录仍包含条目。
    #[error("directory is not empty: {path}")]
    DirectoryNotEmpty {
        /// 未清空的目标目录路径。
        path: PathBuf,
    },
    /// 文件任务状态锁不可用。
    #[error("file task store is unavailable")]
    TaskStorePoisoned,
    /// 传输标识不存在或已结束。
    #[error("file transfer does not exist: {transfer_id}")]
    TransferNotFound {
        /// 不存在或已结束的传输任务标识。
        transfer_id: TaskId,
    },
    /// 传输块偏移没有接上上一个块。
    #[error("file transfer offset mismatch: expected {expected}, got {actual}")]
    TransferOffsetMismatch {
        /// 服务端根据已接收内容计算出的期望偏移。
        expected: u64,
        /// 当前数据块声明的起始偏移。
        actual: u64,
    },
    /// 提交传输时仍未收到声明的全部字节。
    #[error("file transfer is incomplete: expected {expected}, got {actual}")]
    TransferIncomplete {
        /// 传输声明的完整文件字节数。
        expected: u64,
        /// 截止提交时实际收到的字节数。
        actual: u64,
    },
    /// 传输总大小与声明值不一致。
    #[error("file transfer size mismatch: expected {expected}, got {actual}")]
    TransferSizeMismatch {
        /// 传输请求声明的文件字节数。
        expected: u64,
        /// 根据数据块累计出的文件字节数。
        actual: u64,
    },
    /// 完整文件摘要与声明值不一致。
    #[error("file transfer hash does not match the expected value")]
    TransferHashMismatch {
        /// 传输请求声明的完整文件摘要。
        expected: String,
        /// 已接收完整内容计算出的摘要。
        actual: String,
    },
    /// 当前传输块摘要与声明值不一致。
    #[error("file transfer chunk hash does not match the expected value")]
    TransferChunkHashMismatch {
        /// 数据块声明的摘要。
        expected: String,
        /// 当前数据块内容计算出的摘要。
        actual: String,
    },
    /// 当前传输块超过允许大小。
    #[error("file transfer chunk exceeds the maximum size of {maximum_bytes} bytes")]
    TransferChunkTooLarge {
        /// 单个数据块允许的最大字节数。
        maximum_bytes: usize,
    },
    /// 活跃文件传输数量达到上限。
    #[error("too many active file transfers")]
    TooManyTransfers,
    /// 配置文档标识不存在。
    #[error("configuration document does not exist: {document_id}")]
    ConfigDocumentNotFound {
        /// 不存在的配置文档标识。
        document_id: String,
    },
    /// 配置文件无法解析。
    #[error("configuration document could not be parsed: {path}: {message}")]
    ConfigParse {
        /// 解析失败的配置文件路径。
        path: PathBuf,
        /// 解析器返回的诊断文本。
        message: String,
    },
    /// 配置补丁不符合文档格式或字段约束。
    #[error("configuration patch is invalid: {message}")]
    ConfigPatchInvalid {
        /// 补丁校验失败的诊断文本。
        message: String,
    },
    /// 配置文件修订号与当前文件不一致。
    #[error("configuration revision does not match the current file")]
    ConfigRevisionMismatch {
        /// 调用方基于的配置修订号。
        expected: String,
        /// 当前配置文档的修订号。
        actual: String,
    },
    /// 扫描到的配置文档数量超过上限。
    #[error("too many configuration documents; maximum is {maximum_documents}")]
    ConfigScanTooLarge {
        /// 单次扫描允许发现的最大文档数。
        maximum_documents: usize,
    },
}
