use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;

use crate::ConfigError;
use crate::InitialAdminConfig;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelConfig {
    listen_address: SocketAddr,
    data_directory: PathBuf,
    initial_admin: Option<InitialAdminConfig>,
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
            initial_admin: None,
        })
    }

    #[must_use]
    pub fn with_initial_admin(mut self, initial_admin: InitialAdminConfig) -> Self {
        self.initial_admin = Some(initial_admin);
        self
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
    pub const fn initial_admin(&self) -> Option<&InitialAdminConfig> {
        self.initial_admin.as_ref()
    }
}
