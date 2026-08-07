use std::io;
use std::path::PathBuf;

use nexus_domain::InstanceId;
use thiserror::Error;

use crate::DownloadError;
use crate::InstanceRepositoryError;
use crate::RuntimeManagerError;
use nexus_domain::InstanceCreateError;

/// 一键搭建计划解析、执行和回滚错误。
#[derive(Debug, Error)]
pub enum ProvisionManagerError {
    /// 搭建归档解压失败。
    #[error("failed to extract provision archive {path}: {message}")]
    Archive {
        /// 解压失败的搭建归档路径。
        path: PathBuf,
        /// 归档处理器返回的诊断文本。
        message: String,
    },
    /// 目标实例已经存在。
    #[error("instance {instance_id} already exists")]
    AlreadyExists {
        /// 已存在的目标实例标识。
        instance_id: InstanceId,
    },
    /// 计划字段不符合领域或路径约束。
    #[error("provision plan field is invalid: {field}")]
    InvalidPlan {
        /// 不符合约束的计划字段名称。
        field: &'static str,
    },
    /// 供应商 installer 进程执行失败或超时。
    #[error("provision installer failed: {message}")]
    Installer {
        /// 经过截断且不包含秘密的进程诊断。
        message: String,
    },
    /// 执行哈希与已解析计划不一致。
    #[error("provision plan hash does not match the resolved plan")]
    PlanHashMismatch,
    /// 下载产物获取失败。
    #[error(transparent)]
    Download(#[from] DownloadError),
    /// 实例创建配置校验失败。
    #[error(transparent)]
    Instance(#[from] InstanceCreateError),
    /// 实例仓库写入或回滚失败。
    #[error(transparent)]
    Repository(#[from] InstanceRepositoryError),
    /// 运行时解析或安装失败。
    #[error(transparent)]
    Runtime(#[from] RuntimeManagerError),
    /// 搭建目录读写失败。
    #[error("failed to {operation} provision path {path}")]
    Storage {
        /// 失败的文件系统操作名称。
        operation: &'static str,
        /// 发生错误的搭建路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的文件错误。
        source: io::Error,
    },
    /// 搭建任务状态锁不可用。
    #[error("provision task store is unavailable")]
    TaskStorePoisoned,
    /// 计划无法序列化以生成稳定哈希。
    #[error("failed to serialize the provision plan")]
    Serialization(#[source] serde_json::Error),
}
