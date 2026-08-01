use nexus_protocol::ProtocolVersionError;
use nexus_protocol::SessionError;
use nexus_protocol::TlsError;
use std::io;
use thiserror::Error;

use crate::CoreEndpointError;

#[derive(Debug, Error)]
pub enum CoreConnectionError {
    #[error("failed to connect to Core at {address}")]
    Connect {
        address: String,
        #[source]
        source: io::Error,
    },
    #[error("Core TLS certificate fingerprint did not match the session welcome")]
    CertificateFingerprintMismatch,
    #[error(transparent)]
    Endpoint(#[from] CoreEndpointError),
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
    #[error(transparent)]
    Tls(#[from] TlsError),
}
