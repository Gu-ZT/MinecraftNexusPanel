use std::io;

use rustls::Error;
use thiserror::Error as ThisError;

/// TLS 客户端配置、握手或对端证书错误。
#[derive(Debug, ThisError)]
pub enum TlsError {
    /// 本地 TLS 配置无法构建。
    #[error("failed to configure TLS")]
    Configuration(#[source] Error),
    /// TLS 握手过程失败。
    #[error("TLS handshake failed")]
    Handshake(#[source] io::Error),
    /// 服务器名称不是合法的 TLS ServerName。
    #[error("invalid TLS server name: {server_name}")]
    InvalidServerName { server_name: String },
    /// 握手完成但对端没有提供证书。
    #[error("TLS peer did not provide a certificate")]
    MissingPeerCertificate,
}
