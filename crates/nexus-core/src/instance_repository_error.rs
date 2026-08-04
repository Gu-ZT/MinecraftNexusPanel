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
    /// 当前实例状态不允许执行请求动作。
    #[error("instance {instance_id} is in state {state:?}")]
    StateConflict {
        /// 不允许执行当前动作的实例标识。
        instance_id: InstanceId,
        /// 实例当前的生命周期状态。
        state: InstanceState,
    },
}
