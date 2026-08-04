use thiserror::Error;

/// Core 地址解析和 URL 约束错误。
#[derive(Debug, Error, Eq, PartialEq)]
pub enum CoreEndpointError {
    /// 地址无法解析为合法主机和端口。
    #[error("Core address is invalid: {address}")]
    InvalidAddress { address: String },
    /// URL 含有凭据、路径、查询或片段。
    #[error("Core URL must not contain credentials, a path, query parameters, or a fragment")]
    UnexpectedUrlComponents,
    /// URL scheme 不属于支持的 Core 协议方案。
    #[error("unsupported Core URL scheme: {scheme}")]
    UnsupportedScheme { scheme: String },
}
