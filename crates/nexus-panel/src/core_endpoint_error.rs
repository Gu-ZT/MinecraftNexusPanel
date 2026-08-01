use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum CoreEndpointError {
    #[error("Core address is invalid: {address}")]
    InvalidAddress { address: String },
    #[error("Core URL must not contain credentials, a path, query parameters, or a fragment")]
    UnexpectedUrlComponents,
    #[error("unsupported Core URL scheme: {scheme}")]
    UnsupportedScheme { scheme: String },
}
