use std::net::IpAddr;
use std::net::SocketAddr;

use url::Url;

use crate::CoreEndpointError;

const DEFAULT_CORE_PORT: u16 = 25_580;

/// 解析后的 Core 网络端点和 TLS 证书校验策略。
///
/// 域名地址默认验证证书；IP、localhost 和显式跳过校验的地址会关闭系统证书验证，
/// 连接层仍会比较握手欢迎消息中的证书指纹。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreEndpoint {
    host: String,
    port: u16,
    verify_certificate: bool,
}

impl CoreEndpoint {
    /// 解析 Socket 地址或 `tls`/`https`/`mcnp` URL。
    pub fn parse(
        address: &str,
        skip_certificate_verification: bool,
    ) -> Result<Self, CoreEndpointError> {
        if let Ok(address) = address.parse::<SocketAddr>() {
            return Ok(Self::from_socket_address(address));
        }

        let normalized = if address.contains("://") {
            address.to_owned()
        } else {
            format!("tls://{address}")
        };
        let url = Url::parse(&normalized).map_err(|_| CoreEndpointError::InvalidAddress {
            address: address.to_owned(),
        })?;
        if !matches!(url.scheme(), "https" | "mcnp" | "tls") {
            return Err(CoreEndpointError::UnsupportedScheme {
                scheme: url.scheme().to_owned(),
            });
        }
        if !url.username().is_empty()
            || url.password().is_some()
            || !matches!(url.path(), "" | "/")
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(CoreEndpointError::UnexpectedUrlComponents);
        }
        let host = url
            .host_str()
            .filter(|host| !host.is_empty())
            .ok_or_else(|| CoreEndpointError::InvalidAddress {
                address: address.to_owned(),
            })?
            .to_owned();
        let port = url.port().unwrap_or_else(|| {
            if url.scheme() == "https" {
                443
            } else {
                DEFAULT_CORE_PORT
            }
        });
        let verify_certificate = !skip_certificate_verification && !is_local_address(&host);

        Ok(Self {
            host,
            port,
            verify_certificate,
        })
    }

    /// 从 Socket 地址创建端点；该形式默认不做域名证书校验。
    #[must_use]
    pub fn from_socket_address(address: SocketAddr) -> Self {
        Self {
            host: address.ip().to_string(),
            port: address.port(),
            verify_certificate: false,
        }
    }

    /// 返回主机名或 IP 文本。
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// 返回 TCP 端口。
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// 返回是否启用系统 TLS 证书校验。
    #[must_use]
    pub const fn verify_certificate(&self) -> bool {
        self.verify_certificate
    }
}

fn is_local_address(host: &str) -> bool {
    let host = host.trim_end_matches('.');

    host.parse::<IpAddr>().is_ok()
        || host.eq_ignore_ascii_case("localhost")
        || host.to_ascii_lowercase().ends_with(".localhost")
}

#[cfg(test)]
mod tests {
    use super::CoreEndpoint;

    #[test]
    fn verifies_domain_urls_by_default() {
        let endpoint = CoreEndpoint::parse("tls://core.example.com:25580", false)
            .expect("domain Core URL is valid");

        assert_eq!(endpoint.host(), "core.example.com");
        assert_eq!(endpoint.port(), 25_580);
        assert!(endpoint.verify_certificate());
    }

    #[test]
    fn skips_verification_for_ip_and_localhost_addresses() {
        let ip = CoreEndpoint::parse("10.0.0.12:25580", false).expect("IP Core address is valid");
        let localhost = CoreEndpoint::parse("tls://localhost:25580", false)
            .expect("localhost Core URL is valid");

        assert!(!ip.verify_certificate());
        assert!(!localhost.verify_certificate());
    }

    #[test]
    fn honors_explicit_certificate_verification_opt_out() {
        let endpoint = CoreEndpoint::parse("tls://core.example.com:25580", true)
            .expect("domain Core URL is valid");

        assert!(!endpoint.verify_certificate());
    }
}
