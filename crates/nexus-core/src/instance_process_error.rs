use std::io;
use std::path::PathBuf;

use nexus_domain::InstanceId;
use nexus_domain::RuntimeMode;
use thiserror::Error;

use crate::InstanceLogStoreError;
use crate::InstanceRepositoryError;

/// 实例进程启动、控制和观测错误。
#[derive(Debug, Error)]
pub enum InstanceProcessError {
    /// 无法规范化 Core 数据目录。
    #[error("failed to canonicalize the Core data directory {path}")]
    CanonicalizeDataDirectory {
        /// 需要规范化的 Core 数据目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的规范化错误。
        source: io::Error,
    },
    /// 无法规范化实例工作目录。
    #[error("failed to canonicalize instance working directory {path}")]
    CanonicalizeWorkingDirectory {
        /// 需要规范化的实例工作目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的规范化错误。
        source: io::Error,
    },
    /// 无法创建实例工作目录。
    #[error("failed to create instance working directory {path}")]
    CreateWorkingDirectory {
        /// 创建失败的实例工作目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的目录创建错误。
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
    CommandTooLong {
        /// 单条实例命令允许的最大字节数。
        maximum_bytes: usize,
    },
    /// 日志存储不可用。
    #[error(transparent)]
    LogStore(#[from] InstanceLogStoreError),
    /// 进程指标当前不可用。
    #[error("metrics for instance {instance_id} are unavailable")]
    MetricsUnavailable {
        /// 无法读取指标的实例标识。
        instance_id: InstanceId,
    },
    /// 实例没有可控制的进程。
    #[error("instance {instance_id} process is unavailable")]
    ProcessUnavailable {
        /// 没有关联可控制进程的实例标识。
        instance_id: InstanceId,
    },
    /// 进程标准输入不可用。
    #[error("instance {instance_id} process stdin is unavailable")]
    StdinUnavailable {
        /// 标准输入不可用的实例标识。
        instance_id: InstanceId,
    },
    /// 启动配置选择了当前 Core 尚未执行的运行模式。
    #[error("runtime mode {mode:?} for instance {instance_id} is not supported by this Core")]
    UnsupportedRuntimeMode {
        /// 选择的运行模式。
        mode: RuntimeMode,
        /// 受影响的实例标识。
        instance_id: InstanceId,
    },
    /// 监督模式缺少可执行的包装器配置。
    #[error("supervisor configuration for instance {instance_id} is invalid")]
    InvalidSupervisorConfiguration {
        /// 受影响的实例标识。
        instance_id: InstanceId,
    },
    /// 进程标准错误输出不可用。
    #[error("instance {instance_id} process stderr is unavailable")]
    StderrUnavailable {
        /// 标准错误输出不可用的实例标识。
        instance_id: InstanceId,
    },
    /// 进程标准输出不可用。
    #[error("instance {instance_id} process stdout is unavailable")]
    StdoutUnavailable {
        /// 标准输出不可用的实例标识。
        instance_id: InstanceId,
    },
    /// 实例仓库操作失败。
    #[error(transparent)]
    Repository(#[from] InstanceRepositoryError),
    /// 操作系统拒绝启动进程。
    #[error("failed to start the process for instance {instance_id}")]
    Spawn {
        /// 启动失败的实例标识。
        instance_id: InstanceId,
        #[source]
        /// 操作系统返回的进程启动错误。
        source: io::Error,
    },
    /// 子进程没有返回可跟踪的标识。
    #[error("instance {instance_id} process did not expose an identifier")]
    UnknownProcessId {
        /// 没有返回进程 ID 的实例标识。
        instance_id: InstanceId,
    },
    /// 工作目录越出 Core 数据目录。
    #[error("instance working directory {path} escapes the Core data directory")]
    WorkingDirectoryOutsideDataDirectory {
        /// 解析后落在 Core 数据目录之外的工作目录。
        path: PathBuf,
    },
    /// 进程注册表锁不可用。
    #[error("instance process registry lock is poisoned")]
    ProcessRegistryLockPoisoned,
    /// 系统指标锁不可用。
    #[error("process metrics system lock is poisoned")]
    SystemLockPoisoned,
}
