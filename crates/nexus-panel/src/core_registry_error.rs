use nexus_domain::CoreId;
use nexus_protocol::PresharedKeyError;
use nexus_storage::StorageError;
use serde_json::Error as JsonError;
use thiserror::Error;
use tokio::task::JoinError;

use crate::CoreConnectionError;
use crate::SecretCipherError;

#[derive(Debug, Error)]
pub enum CoreRegistryError {
    #[error("Core {core_id} is already registered")]
    AlreadyExists { core_id: CoreId },
    #[error(transparent)]
    Cipher(#[from] SecretCipherError),
    #[error(transparent)]
    Connection(#[from] CoreConnectionError),
    #[error("Core connection timed out")]
    ConnectionTimeout,
    #[error("Core connection is unavailable")]
    ConnectionUnavailable,
    #[error("invalid Core registration field: {field}")]
    InvalidRequest { field: &'static str },
    #[error("stored Core registration is invalid: {core_id}")]
    InvalidStoredCore { core_id: String },
    #[error("stored extension installation is invalid: {path}")]
    InvalidStoredExtension { path: String },
    #[error("Core secret must be valid unpadded Base64URL containing at least 32 bytes")]
    InvalidSecret(#[source] PresharedKeyError),
    #[error("loopback Core returned unexpected identity: expected {expected}, got {actual}")]
    LocalCoreIdMismatch { expected: CoreId, actual: CoreId },
    #[error("Core registration does not exist: {core_id}")]
    NotFound { core_id: CoreId },
    #[error("stored Core tags are invalid")]
    Serialization(#[from] JsonError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("Core registry worker failed")]
    Task(#[from] JoinError),
}
