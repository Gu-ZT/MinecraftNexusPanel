use std::fmt;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;

use crate::ConfigError;
use crate::InitialAdminConfig;
use crate::LocalCoreConfig;
use crate::PanelMasterKey;

#[derive(Clone, Eq, PartialEq)]
pub struct PanelConfig {
    listen_address: SocketAddr,
    data_directory: PathBuf,
    initial_admin: Option<InitialAdminConfig>,
    local_core: Option<LocalCoreConfig>,
    master_key: Option<PanelMasterKey>,
}

impl fmt::Debug for PanelConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PanelConfig")
            .field("listen_address", &self.listen_address)
            .field("data_directory", &self.data_directory)
            .field("initial_admin", &self.initial_admin)
            .field("local_core", &self.local_core.as_ref().map(|_| "REDACTED"))
            .field("master_key", &self.master_key)
            .finish()
    }
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
            local_core: None,
            master_key: None,
        })
    }

    #[must_use]
    pub fn with_initial_admin(mut self, initial_admin: InitialAdminConfig) -> Self {
        self.initial_admin = Some(initial_admin);
        self
    }

    #[must_use]
    pub fn with_master_key(mut self, master_key: PanelMasterKey) -> Self {
        self.master_key = Some(master_key);
        self
    }

    #[must_use]
    pub fn with_local_core(mut self, local_core: LocalCoreConfig) -> Self {
        self.local_core = Some(local_core);
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

    #[must_use]
    pub const fn local_core(&self) -> Option<&LocalCoreConfig> {
        self.local_core.as_ref()
    }

    #[must_use]
    pub const fn master_key(&self) -> Option<&PanelMasterKey> {
        self.master_key.as_ref()
    }
}
