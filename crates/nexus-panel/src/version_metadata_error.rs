use thiserror::Error;

#[derive(Debug, Error)]
pub enum VersionMetadataError {
    #[error("failed to create the version metadata HTTP client")]
    Client(#[source] reqwest::Error),
    #[error("version metadata from provider {provider_id} is invalid")]
    InvalidResponse { provider_id: String },
    #[error("template {template_id} does not define metadata provider {provider_id}")]
    ProviderMissing {
        template_id: String,
        provider_id: String,
    },
    #[error("version metadata request to provider {provider_id} failed")]
    Request {
        provider_id: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("version metadata from provider {provider_id} exceeds {maximum_bytes} bytes")]
    ResponseTooLarge {
        provider_id: String,
        maximum_bytes: usize,
    },
    #[error("template {template_id} does not support version metadata resolution")]
    UnsupportedTemplate { template_id: String },
}
