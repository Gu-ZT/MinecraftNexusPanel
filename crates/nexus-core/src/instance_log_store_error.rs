use thiserror::Error;

/// 实例日志内存存储错误。
#[derive(Debug, Error)]
pub enum InstanceLogStoreError {
    /// 日志存储锁不可用。
    #[error("instance log store lock is poisoned")]
    LockPoisoned,
}
