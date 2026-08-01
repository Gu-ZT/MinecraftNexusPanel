use std::io;
use std::path::PathBuf;

use rcgen::Error as RcgenError;
use rustls::Error as RustlsError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreTlsIdentityError {
    #[error("failed to create the Core TLS identity directory {path}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Core TLS certificate chain is empty: {path}")]
    EmptyCertificateChain { path: PathBuf },
    #[error("failed to generate the default Core TLS identity")]
    Generate(#[source] RcgenError),
    #[error("default Core TLS identity is incomplete: {certificate_path}, {private_key_path}")]
    IncompleteDefaultIdentity {
        certificate_path: PathBuf,
        private_key_path: PathBuf,
    },
    #[error("configured Core TLS identity is incomplete")]
    IncompleteConfiguredIdentity,
    #[error("Core TLS certificate or private key is invalid")]
    InvalidIdentity(#[source] RustlsError),
    #[error("Core TLS private key file does not contain a supported key: {path}")]
    MissingPrivateKey { path: PathBuf },
    #[error("failed to read the Core TLS certificate {path}")]
    ReadCertificate {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to read the Core TLS private key {path}")]
    ReadPrivateKey {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to restrict permissions on the Core TLS private key {path}")]
    RestrictPrivateKey {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to write the default Core TLS certificate {path}")]
    WriteCertificate {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to write the default Core TLS private key {path}")]
    WritePrivateKey {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
