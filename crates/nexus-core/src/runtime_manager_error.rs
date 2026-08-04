use std::io;
use std::path::PathBuf;

use thiserror::Error;

use crate::DownloadError;
use crate::InstanceRepositoryError;

/// 运行时下载、解压、验证、任务和引用检查错误。
#[derive(Debug, Error)]
pub enum RuntimeManagerError {
    /// 归档格式或解压过程返回了结构化错误。
    #[error("runtime archive is invalid: {message}")]
    Archive {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
    /// 归档文件系统操作失败。
    #[error("failed to {operation} runtime archive path {path}")]
    ArchiveIo {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 运行时标识已经存在。
    #[error("runtime {runtime_id} is already installed")]
    AlreadyExists { runtime_id: String },
    /// 安装清单字段不符合领域约束。
    #[error("runtime manifest field is invalid: {field}")]
    InvalidManifest { field: &'static str },
    /// 运行时标识不是安全的相对标识。
    #[error("runtime ID is invalid")]
    InvalidRuntimeId,
    /// 运行时仍被实例启动配置引用。
    #[error("runtime {runtime_id} is referenced by an instance")]
    InUse { runtime_id: String },
    /// 找不到指定受管运行时。
    #[error("runtime {runtime_id} is not installed")]
    NotFound { runtime_id: String },
    /// 清单声明的可执行文件不是安全的普通文件。
    #[error("runtime executable is not valid: {path}")]
    InvalidExecutable { path: PathBuf },
    /// 下载产物获取失败。
    #[error(transparent)]
    Download(#[from] DownloadError),
    /// 实例仓库查询失败。
    #[error(transparent)]
    Repository(#[from] InstanceRepositoryError),
    /// 运行时目录读写失败。
    #[error("failed to {operation} runtime path {path}")]
    Storage {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 归档包含越界路径、符号链接或特殊条目。
    #[error("runtime archive entry is unsafe: {path}")]
    UnsafeArchiveEntry { path: PathBuf },
    /// 运行时任务状态锁不可用。
    #[error("runtime task store is unavailable")]
    TaskStorePoisoned,
}
