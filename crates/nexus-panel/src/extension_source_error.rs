use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtensionSourceError {
    #[error("failed to create the extension source HTTP client")]
    Client(#[source] reqwest::Error),
    #[error("extension source response is invalid")]
    InvalidResponse,
    #[error("extension source request failed")]
    Request(#[source] reqwest::Error),
    #[error("extension source response exceeds {maximum_bytes} bytes")]
    ResponseTooLarge { maximum_bytes: usize },
}
