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
        /// 失败的归档处理操作名称。
        operation: &'static str,
        /// 发生归档错误的路径。
        path: PathBuf,
        /// 归档校验或解压器返回的诊断文本。
        message: String,
    },
    /// 归档文件系统操作失败。
    #[error("failed to {operation} runtime archive path {path}")]
    ArchiveIo {
        /// 失败的归档文件系统操作名称。
        operation: &'static str,
        /// 发生错误的归档路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的文件错误。
        source: io::Error,
    },
    /// 运行时标识已经存在。
    #[error("runtime {runtime_id} is already installed")]
    AlreadyExists {
        /// 已安装的运行时标识。
        runtime_id: String,
    },
    /// 安装清单字段不符合领域约束。
    #[error("runtime manifest field is invalid: {field}")]
    InvalidManifest {
        /// 不符合约束的安装清单字段名称。
        field: &'static str,
    },
    /// 运行时标识不是安全的相对标识。
    #[error("runtime ID is invalid")]
    InvalidRuntimeId,
    /// 运行时仍被实例启动配置引用。
    #[error("runtime {runtime_id} is referenced by an instance")]
    InUse {
        /// 仍被实例引用的运行时标识。
        runtime_id: String,
    },
    /// 找不到指定受管运行时。
    #[error("runtime {runtime_id} is not installed")]
    NotFound {
        /// 未找到的运行时标识。
        runtime_id: String,
    },
    /// 清单声明的可执行文件不是安全的普通文件。
    #[error("runtime executable is not valid: {path}")]
    InvalidExecutable {
        /// 不符合普通文件或目录边界约束的可执行文件路径。
        path: PathBuf,
    },
    /// 下载产物获取失败。
    #[error(transparent)]
    Download(#[from] DownloadError),
    /// 实例仓库查询失败。
    #[error(transparent)]
    Repository(#[from] InstanceRepositoryError),
    /// 运行时目录读写失败。
    #[error("failed to {operation} runtime path {path}")]
    Storage {
        /// 失败的运行时文件系统操作名称。
        operation: &'static str,
        /// 发生错误的运行时路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的文件错误。
        source: io::Error,
    },
    /// 归档包含越界路径、符号链接或特殊条目。
    #[error("runtime archive entry is unsafe: {path}")]
    UnsafeArchiveEntry {
        /// 被拒绝的归档条目路径。
        path: PathBuf,
    },
    /// 运行时任务状态锁不可用。
    #[error("runtime task store is unavailable")]
    TaskStorePoisoned,
}
