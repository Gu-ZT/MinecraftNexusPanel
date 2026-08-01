use std::io;
use std::path::PathBuf;

use nexus_domain::InstanceId;
use thiserror::Error;

use crate::InstanceLogStoreError;
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
    #[error("instance command must not contain a NUL byte")]
    CommandContainsNul,
    #[error("instance command must not be empty")]
    CommandEmpty,
    #[error("instance command exceeds the maximum size of {maximum_bytes} bytes")]
    CommandTooLong { maximum_bytes: usize },
    #[error(transparent)]
    LogStore(#[from] InstanceLogStoreError),
    #[error("metrics for instance {instance_id} are unavailable")]
    MetricsUnavailable { instance_id: InstanceId },
    #[error("instance {instance_id} process is unavailable")]
    ProcessUnavailable { instance_id: InstanceId },
    #[error("instance {instance_id} process stdin is unavailable")]
    StdinUnavailable { instance_id: InstanceId },
    #[error("instance {instance_id} process stderr is unavailable")]
    StderrUnavailable { instance_id: InstanceId },
    #[error("instance {instance_id} process stdout is unavailable")]
    StdoutUnavailable { instance_id: InstanceId },
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
    #[error("process metrics system lock is poisoned")]
    SystemLockPoisoned,
}
