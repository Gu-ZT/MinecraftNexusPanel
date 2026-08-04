use std::sync::Arc;

use rustls::ClientConfig;
use rustls::RootCertStore;
use rustls::crypto::ring::default_provider;
use rustls::pki_types::ServerName;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::TlsError;
use crate::certificate_sha256;
use crate::insecure_server_certificate_verifier::InsecureServerCertificateVerifier;

const CORE_ALPN_PROTOCOL: &[u8] = b"mcnp/1";

/// 基于 Tokio 异步流的 TLS 客户端流类型别名。
pub type TlsClientStream<S> = TlsStream<S>;

/// 建立面向 Core 的 TLS 客户端连接并返回对端证书指纹。
///
/// `verify_certificate = true` 使用系统内置 Web PKI 根证书；传入 `false` 会跳过
/// 证书链和主机名信任校验，仅保留握手签名校验，适合由更高层执行指纹固定的场景。
/// 调用方必须明确选择其安全策略，返回的指纹本身不代表证书可信。
pub async fn connect_tls<S>(
    stream: S,
    server_name: String,
    verify_certificate: bool,
) -> Result<(TlsClientStream<S>, String), TlsError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let server_name = ServerName::try_from(server_name.clone())
        .map_err(|_| TlsError::InvalidServerName { server_name })?;
    let connector = TlsConnector::from(Arc::new(client_config(verify_certificate)?));
    let tls_stream = connector
        .connect(server_name, stream)
        .await
        .map_err(TlsError::Handshake)?;
    let certificate = tls_stream
        .get_ref()
        .1
        .peer_certificates()
        .and_then(|certificates| certificates.first())
        .ok_or(TlsError::MissingPeerCertificate)?;
    let fingerprint = certificate_sha256(certificate.as_ref());

    Ok((tls_stream, fingerprint))
}

fn client_config(verify_certificate: bool) -> Result<ClientConfig, TlsError> {
    let provider = default_provider();
    let builder = ClientConfig::builder_with_provider(Arc::new(provider.clone()))
        .with_safe_default_protocol_versions()
        .map_err(TlsError::Configuration)?;
    let mut config = if verify_certificate {
        let roots = RootCertStore::from_iter(TLS_SERVER_ROOTS.iter().cloned());
        builder.with_root_certificates(roots).with_no_client_auth()
    } else {
        let verifier =
            InsecureServerCertificateVerifier::new(provider.signature_verification_algorithms);
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(verifier))
            .with_no_client_auth()
    };
    config.alpn_protocols = vec![CORE_ALPN_PROTOCOL.to_vec()];

    Ok(config)
}
