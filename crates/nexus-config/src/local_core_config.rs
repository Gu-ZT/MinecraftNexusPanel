use std::fmt;
use std::net::SocketAddr;

use nexus_domain::CoreId;

#[derive(Clone, Eq, PartialEq)]
pub struct LocalCoreConfig {
    core_id: CoreId,
    listen_address: SocketAddr,
    encoded_pre_shared_key: String,
}

impl fmt::Debug for LocalCoreConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalCoreConfig")
            .field("core_id", &self.core_id)
            .field("listen_address", &self.listen_address)
            .field("encoded_pre_shared_key", &"REDACTED")
            .finish()
    }
}

impl LocalCoreConfig {
    #[must_use]
    pub fn new(
        core_id: CoreId,
        listen_address: SocketAddr,
        encoded_pre_shared_key: String,
    ) -> Self {
        Self {
            core_id,
            listen_address,
            encoded_pre_shared_key,
        }
    }

    #[must_use]
    pub const fn core_id(&self) -> CoreId {
        self.core_id
    }

    #[must_use]
    pub const fn listen_address(&self) -> SocketAddr {
        self.listen_address
    }

    #[must_use]
    pub fn encoded_pre_shared_key(&self) -> &str {
        &self.encoded_pre_shared_key
    }
}
