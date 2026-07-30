use std::io;
use std::net::SocketAddr;

use nexus_protocol::ProtocolVersionError;
use nexus_protocol::SessionError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreConnectionError {
    #[error("failed to connect to Core at {address}")]
    Connect {
        address: SocketAddr,
        #[source]
        source: io::Error,
    },
    #[error("Core returned a malformed response field: {field}")]
    InvalidResponse { field: &'static str },
    #[error("Core rejected the request: {code}")]
    Rejected { code: String },
    #[error(transparent)]
    ProtocolVersion(#[from] ProtocolVersionError),
    #[error("Core response request ID did not match the request")]
    RequestIdMismatch,
    #[error(transparent)]
    Session(#[from] SessionError),
}
