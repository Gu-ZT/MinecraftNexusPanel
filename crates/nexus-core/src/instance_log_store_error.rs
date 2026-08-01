use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstanceLogStoreError {
    #[error("instance log store lock is poisoned")]
    LockPoisoned,
}
