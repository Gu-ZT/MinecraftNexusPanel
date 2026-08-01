use std::io;

use rustls::Error;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum TlsError {
    #[error("failed to configure TLS")]
    Configuration(#[source] Error),
    #[error("TLS handshake failed")]
    Handshake(#[source] io::Error),
    #[error("invalid TLS server name: {server_name}")]
    InvalidServerName { server_name: String },
    #[error("TLS peer did not provide a certificate")]
    MissingPeerCertificate,
}
