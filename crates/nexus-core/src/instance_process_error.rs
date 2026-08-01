use std::io;
use std::path::PathBuf;

use nexus_domain::InstanceId;
use thiserror::Error;

use crate::InstanceRepositoryError;

#[derive(Debug, Error)]
pub enum InstanceProcessError {
    #[error("failed to canonicalize the Core data directory {path}")]
    CanonicalizeDataDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to canonicalize instance working directory {path}")]
    CanonicalizeWorkingDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to create instance working directory {path}")]
    CreateWorkingDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("instance {instance_id} process is unavailable")]
    ProcessUnavailable { instance_id: InstanceId },
    #[error("instance {instance_id} process stdin is unavailable")]
    StdinUnavailable { instance_id: InstanceId },
    #[error(transparent)]
    Repository(#[from] InstanceRepositoryError),
    #[error("failed to start the process for instance {instance_id}")]
    Spawn {
        instance_id: InstanceId,
        #[source]
        source: io::Error,
    },
    #[error("instance {instance_id} process did not expose an identifier")]
    UnknownProcessId { instance_id: InstanceId },
    #[error("instance working directory {path} escapes the Core data directory")]
    WorkingDirectoryOutsideDataDirectory { path: PathBuf },
    #[error("instance process registry lock is poisoned")]
    ProcessRegistryLockPoisoned,
}
