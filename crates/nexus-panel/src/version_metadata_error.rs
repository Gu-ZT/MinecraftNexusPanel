use thiserror::Error;

/// 安装模板版本元数据客户端错误。
#[derive(Debug, Error)]
pub enum VersionMetadataError {
    /// 元数据 HTTP 客户端初始化失败。
    #[error("failed to create the version metadata HTTP client")]
    Client(#[source] reqwest::Error),
    /// provider 返回的数据结构无法解析。
    #[error("version metadata from provider {provider_id} is invalid")]
    InvalidResponse {
        /// 返回非法数据的 provider 标识。
        provider_id: String,
    },
    /// 模板没有声明请求的 provider。
    #[error("template {template_id} does not define metadata provider {provider_id}")]
    ProviderMissing {
        /// 未声明该 provider 的安装模板标识。
        template_id: String,
        /// 模板配置中缺失的 provider 标识。
        provider_id: String,
    },
    /// provider HTTP 请求或状态码处理失败。
    #[error("version metadata request to provider {provider_id} failed")]
    Request {
        /// 请求失败的 provider 标识。
        provider_id: String,
        #[source]
        /// HTTP 客户端返回的请求错误。
        source: reqwest::Error,
    },
    /// provider 响应超过大小限制。
    #[error("version metadata from provider {provider_id} exceeds {maximum_bytes} bytes")]
    ResponseTooLarge {
        /// 返回超限响应的 provider 标识。
        provider_id: String,
        /// 当前 provider 响应允许的最大字节数。
        maximum_bytes: usize,
    },
    /// 当前模板没有实现版本元数据解析。
    #[error("template {template_id} does not support version metadata resolution")]
    UnsupportedTemplate {
        /// 不支持版本元数据解析的安装模板标识。
        template_id: String,
    },
}
