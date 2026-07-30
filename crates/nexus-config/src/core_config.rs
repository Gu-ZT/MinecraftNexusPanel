use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;

use nexus_protocol::PresharedKey;

use crate::ConfigError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreConfig {
    listen_address: SocketAddr,
    data_directory: PathBuf,
    pre_shared_key: Option<PresharedKey>,
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
            .map(|value| PresharedKey::from_base64url(&value))
            .transpose()
            .map_err(ConfigError::InvalidCorePreSharedKey)?;

        Ok(Self {
            listen_address,
            data_directory,
            pre_shared_key,
        })
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
}
