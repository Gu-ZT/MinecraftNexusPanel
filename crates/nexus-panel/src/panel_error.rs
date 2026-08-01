use std::io;
use std::net::SocketAddr;

use nexus_storage::StorageError;
use thiserror::Error;

use crate::AuthError;

#[derive(Debug, Error)]
pub enum PanelError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error("failed to bind the Panel HTTP listener at {address}")]
    Bind {
        address: SocketAddr,
        #[source]
        source: io::Error,
    },
    #[error("Panel HTTP server failed")]
    Serve(#[source] io::Error),
    #[error(transparent)]
    Storage(#[from] StorageError),
}
