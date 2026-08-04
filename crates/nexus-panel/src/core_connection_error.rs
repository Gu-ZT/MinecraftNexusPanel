use nexus_protocol::ProtocolVersionError;
use nexus_protocol::SessionError;
use nexus_protocol::TlsError;
use std::io;
use thiserror::Error;

use crate::CoreEndpointError;

/// Panel 到 Core 的连接、握手和响应解码错误。
#[derive(Debug, Error)]
pub enum CoreConnectionError {
    /// TCP 连接失败。
    #[error("failed to connect to Core at {address}")]
    Connect {
        /// 连接失败的 Core 地址文本。
        address: String,
        #[source]
        /// 操作系统返回的连接错误。
        source: io::Error,
    },
    /// TLS 层指纹与会话欢迎消息不一致。
    #[error("Core TLS certificate fingerprint did not match the session welcome")]
    CertificateFingerprintMismatch,
    /// Core 端点地址解析失败。
    #[error(transparent)]
    Endpoint(#[from] CoreEndpointError),
    /// Core 响应缺少或包含错误类型的字段。
    #[error("Core returned a malformed response field: {field}")]
    InvalidResponse {
        /// 缺失或类型不正确的响应字段名称。
        field: &'static str,
    },
    /// Core 明确拒绝了请求。
    #[error("Core rejected the request: {code}")]
    Rejected {
        /// Core 返回的稳定错误代码。
        code: String,
    },
    /// 协议版本无法协商。
    #[error(transparent)]
    ProtocolVersion(#[from] ProtocolVersionError),
    /// 响应 request ID 与发送请求不匹配。
    #[error("Core response request ID did not match the request")]
    RequestIdMismatch,
    /// Noise 会话建立或消息传输失败。
    #[error(transparent)]
    Session(#[from] SessionError),
    /// TLS 配置或握手失败。
    #[error(transparent)]
    Tls(#[from] TlsError),
}
