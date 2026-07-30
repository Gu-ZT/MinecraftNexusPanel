use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;

use nexus_protocol::SessionError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("failed to accept a Core TCP connection")]
    Accept(#[source] io::Error),
    #[error("failed to bind the Core TCP listener at {address}")]
    Bind {
        address: SocketAddr,
        #[source]
        source: io::Error,
    },
    #[error("failed to create the Core data directory {path}")]
    CreateDataDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Core identity file {path} contains an invalid identifier")]
    InvalidStoredCoreId { path: PathBuf },
    #[error("Core requires MCNP_CORE_PSK to accept Panel connections")]
    MissingPreSharedKey,
    #[error("failed to read the Core identity file {path}")]
    ReadCoreIdentity {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error("failed to write the Core identity file {path}")]
    WriteCoreIdentity {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
