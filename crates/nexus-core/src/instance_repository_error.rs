use std::io;
use std::path::PathBuf;

use nexus_domain::InstanceCreateError;
use nexus_domain::InstanceId;
use nexus_domain::InstanceState;
use nexus_domain::InstanceUpdateError;
use thiserror::Error;

/// 实例仓库的唯一性、校验、并发版本和生命周期冲突错误。
#[derive(Debug, Error)]
pub enum InstanceRepositoryError {
    /// 实例标识已经存在。
    #[error("instance {instance_id} already exists")]
    AlreadyExists {
        /// 冲突的实例标识。
        instance_id: InstanceId,
    },
    /// 创建配置校验失败。
    #[error(transparent)]
    InvalidInstance(#[from] InstanceCreateError),
    /// 更新配置校验失败。
    #[error(transparent)]
    InvalidUpdate(#[from] InstanceUpdateError),
    /// 实例不存在。
    #[error("instance {instance_id} does not exist")]
    NotFound {
        /// 未找到的实例标识。
        instance_id: InstanceId,
    },
    /// 调用方使用的修订号已过期。
    #[error(
        "instance revision does not match: expected {expected_revision}, actual {actual_revision}"
    )]
    RevisionMismatch {
        /// 调用方持有的旧修订号。
        expected_revision: u64,
        /// 仓储当前保存的修订号。
        actual_revision: u64,
    },
    /// 实例仓库锁不可用。
    #[error("instance repository lock is poisoned")]
    LockPoisoned,
    /// 实例数据目录创建失败。
    #[error("failed to create instance data directory {path}")]
    CreateDirectory {
        /// 创建失败的数据目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的目录创建错误。
        source: io::Error,
    },
    /// 读取实例数据文件失败。
    #[error("failed to read instance store {path}")]
    Read {
        /// 读取失败的实例数据文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的读取错误。
        source: io::Error,
    },
    /// 实例数据文件不是合法的 JSON 实例映射。
    #[error("instance store {path} contains invalid JSON")]
    Decode {
        /// 包含非法 JSON 的实例数据文件路径。
        path: PathBuf,
        #[source]
        /// JSON 解码错误。
        source: serde_json::Error,
    },
    /// 创建实例临时数据文件失败。
    #[error("failed to create temporary instance store in {path}")]
    CreateTemporary {
        /// 创建临时文件的目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的创建错误。
        source: io::Error,
    },
    /// 序列化实例数据失败。
    #[error("failed to encode instance store {path}")]
    Encode {
        /// 无法写入的实例数据文件路径。
        path: PathBuf,
        #[source]
        /// JSON 编码错误。
        source: serde_json::Error,
    },
    /// 写入或同步实例临时数据文件失败。
    #[error("failed to write instance store {path}")]
    Write {
        /// 写入失败的实例数据文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的写入错误。
        source: io::Error,
    },
    /// 用临时文件替换正式实例数据文件失败。
    #[error("failed to atomically replace instance store {path}")]
    Replace {
        /// 替换失败的正式实例数据文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的替换错误。
        source: io::Error,
    },
    /// 当前实例状态不允许执行请求动作。
    #[error("instance {instance_id} is in state {state:?}")]
    StateConflict {
        /// 不允许执行当前动作的实例标识。
        instance_id: InstanceId,
        /// 实例当前的生命周期状态。
        state: InstanceState,
    },
}
