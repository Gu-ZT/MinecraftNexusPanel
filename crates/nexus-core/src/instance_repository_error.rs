use nexus_domain::InstanceCreateError;
use nexus_domain::InstanceId;
use nexus_domain::InstanceState;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstanceRepositoryError {
    #[error("instance {instance_id} already exists")]
    AlreadyExists { instance_id: InstanceId },
    #[error(transparent)]
    InvalidInstance(#[from] InstanceCreateError),
    #[error("instance {instance_id} does not exist")]
    NotFound { instance_id: InstanceId },
    #[error("instance repository lock is poisoned")]
    LockPoisoned,
    #[error("instance {instance_id} is in state {state:?}")]
    StateConflict {
        instance_id: InstanceId,
        state: InstanceState,
    },
}
