use std::io;
use std::net::SocketAddr;

use nexus_storage::StorageError;
use thiserror::Error;

use crate::AuthError;
use crate::CoreRegistryError;
use crate::VersionMetadataError;

#[derive(Debug, Error)]
pub enum PanelError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    CoreRegistry(#[from] CoreRegistryError),
    #[error("failed to bind the Panel HTTP listener at {address}")]
    Bind {
        address: SocketAddr,
        #[source]
        source: io::Error,
    },
    #[error("Panel HTTP server failed")]
    Serve(#[source] io::Error),
    #[error("MCNP_PANEL_MASTER_KEY is required to encrypt registered Core secrets")]
    MissingPanelMasterKey,
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    VersionMetadata(#[from] VersionMetadataError),
}
