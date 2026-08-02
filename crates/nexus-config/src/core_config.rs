use std::fmt;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;

use nexus_protocol::PresharedKey;

use crate::ConfigError;

#[derive(Clone, Eq, PartialEq)]
pub struct CoreConfig {
    listen_address: SocketAddr,
    data_directory: PathBuf,
    encoded_pre_shared_key: Option<String>,
    pre_shared_key: Option<PresharedKey>,
    tls_certificate_path: Option<PathBuf>,
    tls_private_key_path: Option<PathBuf>,
}

impl fmt::Debug for CoreConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CoreConfig")
            .field("listen_address", &self.listen_address)
            .field("data_directory", &self.data_directory)
            .field(
                "encoded_pre_shared_key",
                &self.encoded_pre_shared_key.as_ref().map(|_| "REDACTED"),
            )
            .field("pre_shared_key", &self.pre_shared_key)
            .field("tls_certificate_path", &self.tls_certificate_path)
            .field("tls_private_key_path", &self.tls_private_key_path)
            .finish()
    }
}

impl CoreConfig {
    pub const DEFAULT_LISTEN_ADDRESS: &'static str = "0.0.0.0:25580";

    pub fn new(
        listen_address: String,
        data_directory: PathBuf,
        encoded_pre_shared_key: Option<String>,
    ) -> Result<Self, ConfigError> {
        let listen_address =
            listen_address
                .parse()
                .map_err(|_| ConfigError::InvalidSocketAddress {
                    option: "--core-listen",
                    value: listen_address,
                })?;
        let pre_shared_key = encoded_pre_shared_key
            .as_deref()
            .map(PresharedKey::from_base64url)
            .transpose()
            .map_err(ConfigError::InvalidCorePreSharedKey)?;

        Ok(Self {
            listen_address,
            data_directory,
            encoded_pre_shared_key,
            pre_shared_key,
            tls_certificate_path: None,
            tls_private_key_path: None,
        })
    }

    pub fn with_tls_identity_paths(
        mut self,
        certificate_path: Option<PathBuf>,
        private_key_path: Option<PathBuf>,
    ) -> Result<Self, ConfigError> {
        match (&certificate_path, &private_key_path) {
            (Some(_), Some(_)) | (None, None) => {
                self.tls_certificate_path = certificate_path;
                self.tls_private_key_path = private_key_path;
                Ok(self)
            }
            (Some(_), None) | (None, Some(_)) => Err(ConfigError::IncompleteCoreTlsIdentity),
        }
    }

    #[must_use]
    pub const fn listen_address(&self) -> SocketAddr {
        self.listen_address
    }

    #[must_use]
    pub fn data_directory(&self) -> &Path {
        &self.data_directory
    }

    #[must_use]
    pub const fn pre_shared_key(&self) -> Option<&PresharedKey> {
        self.pre_shared_key.as_ref()
    }

    #[must_use]
    pub fn encoded_pre_shared_key(&self) -> Option<&str> {
        self.encoded_pre_shared_key.as_deref()
    }

    #[must_use]
    pub fn tls_certificate_path(&self) -> Option<&Path> {
        self.tls_certificate_path.as_deref()
    }

    #[must_use]
    pub fn tls_private_key_path(&self) -> Option<&Path> {
        self.tls_private_key_path.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::CoreConfig;
    use crate::ConfigError;

    #[test]
    fn requires_tls_certificate_and_private_key_together() {
        let config = CoreConfig::new("127.0.0.1:25580".to_owned(), PathBuf::from("data"), None)
            .expect("base Core configuration is valid");

        assert_eq!(
            config.with_tls_identity_paths(Some(PathBuf::from("cert.pem")), None),
            Err(ConfigError::IncompleteCoreTlsIdentity)
        );
    }

    #[test]
    fn redacts_the_pre_shared_key_from_debug_output() {
        let secret = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY";
        let config = CoreConfig::new(
            "127.0.0.1:25580".to_owned(),
            PathBuf::from("data"),
            Some(secret.to_owned()),
        )
        .expect("base Core configuration is valid");

        let debug_output = format!("{config:?}");

        assert!(!debug_output.contains(secret));
        assert!(debug_output.contains("REDACTED"));
    }
}
