use nexus_protocol::PresharedKeyError;
use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ConfigError {
    #[error("mode can only be specified once")]
    DuplicateMode,
    #[error("help requested")]
    HelpRequested,
    #[error("invalid mode: {value}")]
    InvalidMode { value: String },
    #[error("invalid socket address for {option}: {value}")]
    InvalidSocketAddress { option: &'static str, value: String },
    #[error("invalid Core pre-shared key")]
    InvalidCorePreSharedKey(#[source] PresharedKeyError),
    #[error("Core TLS certificate and private key must be configured together")]
    IncompleteCoreTlsIdentity,
    #[error("logging filter cannot be empty")]
    EmptyLogFilter,
    #[error("missing value for {option}")]
    MissingValue { option: &'static str },
    #[error("unsupported option: {option}")]
    UnsupportedOption { option: String },
    #[error("version requested")]
    VersionRequested,
}
