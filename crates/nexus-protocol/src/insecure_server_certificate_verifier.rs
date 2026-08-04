use rustls::DigitallySignedStruct;
use rustls::Error;
use rustls::SignatureScheme;
use rustls::client::danger::HandshakeSignatureValid;
use rustls::client::danger::ServerCertVerified;
use rustls::client::danger::ServerCertVerifier;
use rustls::crypto::WebPkiSupportedAlgorithms;
use rustls::crypto::verify_tls12_signature;
use rustls::crypto::verify_tls13_signature;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::ServerName;
use rustls::pki_types::UnixTime;

/// 仅跳过证书信任链检查、仍验证握手签名的内部验证器。
///
/// 该类型只在调用方明确关闭系统证书校验时使用；证书指纹固定等额外策略
/// 必须由上层在连接建立后完成。
#[derive(Debug)]
pub(crate) struct InsecureServerCertificateVerifier {
    supported_algorithms: WebPkiSupportedAlgorithms,
}

impl InsecureServerCertificateVerifier {
    pub(crate) const fn new(supported_algorithms: WebPkiSupportedAlgorithms) -> Self {
        Self {
            supported_algorithms,
        }
    }
}

impl ServerCertVerifier for InsecureServerCertificateVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(message, certificate, signature, &self.supported_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(message, certificate, signature, &self.supported_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.supported_algorithms.supported_schemes()
    }
}
