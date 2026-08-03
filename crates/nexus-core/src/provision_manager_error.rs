use std::io;
use std::path::PathBuf;

use nexus_domain::InstanceId;
use thiserror::Error;

use crate::DownloadError;
use crate::InstanceRepositoryError;
use crate::RuntimeManagerError;
use nexus_domain::InstanceCreateError;

#[derive(Debug, Error)]
pub enum ProvisionManagerError {
    #[error("failed to extract provision archive {path}: {message}")]
    Archive { path: PathBuf, message: String },
    #[error("instance {instance_id} already exists")]
    AlreadyExists { instance_id: InstanceId },
    #[error("provision plan field is invalid: {field}")]
    InvalidPlan { field: &'static str },
    #[error("provision plan hash does not match the resolved plan")]
    PlanHashMismatch,
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error(transparent)]
    Instance(#[from] InstanceCreateError),
    #[error(transparent)]
    Repository(#[from] InstanceRepositoryError),
    #[error(transparent)]
    Runtime(#[from] RuntimeManagerError),
    #[error("failed to {operation} provision path {path}")]
    Storage {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("provision task store is unavailable")]
    TaskStorePoisoned,
    #[error("failed to serialize the provision plan")]
    Serialization(#[source] serde_json::Error),
}
