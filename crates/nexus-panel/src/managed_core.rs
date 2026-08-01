use nexus_protocol::PresharedKey;
use nexus_storage::StoredCore;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::sync::RwLock;

use crate::CoreConnection;
use crate::CoreRuntime;

pub struct ManagedCore {
    pub(crate) registration: StoredCore,
    pub(crate) pre_shared_key: PresharedKey,
    pub(crate) connection: Mutex<Option<CoreConnection>>,
    pub(crate) runtime: RwLock<CoreRuntime>,
    pub(crate) reconnect: Notify,
}

impl ManagedCore {
    #[must_use]
    pub fn new(
        registration: StoredCore,
        pre_shared_key: PresharedKey,
        connection: Option<CoreConnection>,
        runtime: CoreRuntime,
    ) -> Self {
        Self {
            registration,
            pre_shared_key,
            connection: Mutex::new(connection),
            runtime: RwLock::new(runtime),
            reconnect: Notify::new(),
        }
    }
}
