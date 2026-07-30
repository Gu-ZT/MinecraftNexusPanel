use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;

use crate::ConfigError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelConfig {
    listen_address: SocketAddr,
    data_directory: PathBuf,
}

impl PanelConfig {
    pub const DEFAULT_LISTEN_ADDRESS: &'static str = "127.0.0.1:8080";

    pub fn new(listen_address: String, data_directory: PathBuf) -> Result<Self, ConfigError> {
        let listen_address =
            listen_address
                .parse()
                .map_err(|_| ConfigError::InvalidSocketAddress {
                    option: "--panel-listen",
                    value: listen_address,
                })?;

        Ok(Self {
            listen_address,
            data_directory,
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
}
