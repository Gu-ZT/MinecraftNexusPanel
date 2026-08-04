use thiserror::Error;

/// 安装模板版本元数据客户端错误。
#[derive(Debug, Error)]
pub enum VersionMetadataError {
    /// 元数据 HTTP 客户端初始化失败。
    #[error("failed to create the version metadata HTTP client")]
    Client(#[source] reqwest::Error),
    /// provider 返回的数据结构无法解析。
    #[error("version metadata from provider {provider_id} is invalid")]
    InvalidResponse { provider_id: String },
    /// 模板没有声明请求的 provider。
    #[error("template {template_id} does not define metadata provider {provider_id}")]
    ProviderMissing {
        template_id: String,
        provider_id: String,
    },
    /// provider HTTP 请求或状态码处理失败。
    #[error("version metadata request to provider {provider_id} failed")]
    Request {
        provider_id: String,
        #[source]
        source: reqwest::Error,
    },
    /// provider 响应超过大小限制。
    #[error("version metadata from provider {provider_id} exceeds {maximum_bytes} bytes")]
    ResponseTooLarge {
        provider_id: String,
        maximum_bytes: usize,
    },
    /// 当前模板没有实现版本元数据解析。
    #[error("template {template_id} does not support version metadata resolution")]
    UnsupportedTemplate { template_id: String },
}
