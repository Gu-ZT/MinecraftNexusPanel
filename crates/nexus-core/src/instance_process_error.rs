use std::io;
use std::path::PathBuf;

use nexus_domain::InstanceId;
use thiserror::Error;

use crate::InstanceLogStoreError;
use crate::InstanceRepositoryError;

/// 实例进程启动、控制和观测错误。
#[derive(Debug, Error)]
pub enum InstanceProcessError {
    /// 无法规范化 Core 数据目录。
    #[error("failed to canonicalize the Core data directory {path}")]
    CanonicalizeDataDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 无法规范化实例工作目录。
    #[error("failed to canonicalize instance working directory {path}")]
    CanonicalizeWorkingDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 无法创建实例工作目录。
    #[error("failed to create instance working directory {path}")]
    CreateWorkingDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 命令包含 NUL 字节。
    #[error("instance command must not contain a NUL byte")]
    CommandContainsNul,
    /// 命令为空。
    #[error("instance command must not be empty")]
    CommandEmpty,
    /// 命令超过允许的字节数。
    #[error("instance command exceeds the maximum size of {maximum_bytes} bytes")]
    CommandTooLong { maximum_bytes: usize },
    /// 日志存储不可用。
    #[error(transparent)]
    LogStore(#[from] InstanceLogStoreError),
    /// 进程指标当前不可用。
    #[error("metrics for instance {instance_id} are unavailable")]
    MetricsUnavailable { instance_id: InstanceId },
    /// 实例没有可控制的进程。
    #[error("instance {instance_id} process is unavailable")]
    ProcessUnavailable { instance_id: InstanceId },
    /// 进程标准输入不可用。
    #[error("instance {instance_id} process stdin is unavailable")]
    StdinUnavailable { instance_id: InstanceId },
    /// 进程标准错误输出不可用。
    #[error("instance {instance_id} process stderr is unavailable")]
    StderrUnavailable { instance_id: InstanceId },
    /// 进程标准输出不可用。
    #[error("instance {instance_id} process stdout is unavailable")]
    StdoutUnavailable { instance_id: InstanceId },
    /// 实例仓库操作失败。
    #[error(transparent)]
    Repository(#[from] InstanceRepositoryError),
    /// 操作系统拒绝启动进程。
    #[error("failed to start the process for instance {instance_id}")]
    Spawn {
        instance_id: InstanceId,
        #[source]
        source: io::Error,
    },
    /// 子进程没有返回可跟踪的标识。
    #[error("instance {instance_id} process did not expose an identifier")]
    UnknownProcessId { instance_id: InstanceId },
    /// 工作目录越出 Core 数据目录。
    #[error("instance working directory {path} escapes the Core data directory")]
    WorkingDirectoryOutsideDataDirectory { path: PathBuf },
    /// 进程注册表锁不可用。
    #[error("instance process registry lock is poisoned")]
    ProcessRegistryLockPoisoned,
    /// 系统指标锁不可用。
    #[error("process metrics system lock is poisoned")]
    SystemLockPoisoned,
}
