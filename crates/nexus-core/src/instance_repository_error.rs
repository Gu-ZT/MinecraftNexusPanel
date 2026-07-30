use nexus_domain::InstanceCreateError;
use nexus_domain::InstanceId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstanceRepositoryError {
    #[error("instance {instance_id} already exists")]
    AlreadyExists { instance_id: InstanceId },
    #[error(transparent)]
    InvalidInstance(#[from] InstanceCreateError),
    #[error("instance repository lock is poisoned")]
    LockPoisoned,
}
