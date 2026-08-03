use nexus_domain::InstanceCreateError;
use nexus_domain::InstanceId;
use nexus_domain::InstanceState;
use nexus_domain::InstanceUpdateError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstanceRepositoryError {
    #[error("instance {instance_id} already exists")]
    AlreadyExists { instance_id: InstanceId },
    #[error(transparent)]
    InvalidInstance(#[from] InstanceCreateError),
    #[error(transparent)]
    InvalidUpdate(#[from] InstanceUpdateError),
    #[error("instance {instance_id} does not exist")]
    NotFound { instance_id: InstanceId },
    #[error(
        "instance revision does not match: expected {expected_revision}, actual {actual_revision}"
    )]
    RevisionMismatch {
        expected_revision: u64,
        actual_revision: u64,
    },
    #[error("instance repository lock is poisoned")]
    LockPoisoned,
    #[error("instance {instance_id} is in state {state:?}")]
    StateConflict {
        instance_id: InstanceId,
        state: InstanceState,
    },
}
