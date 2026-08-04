use nexus_protocol::PresharedKey;
use nexus_storage::StoredCore;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::sync::RwLock;

use crate::CoreConnection;
use crate::CoreRuntime;

/// Panel 注册的 Core 及其连接、运行时和重连状态。
///
/// 预共享密钥由 Panel 内存持有，连接和运行时状态分别使用异步锁保护；注册记录
/// 本身来自存储层，连接断开时仍可用于展示最近状态。
pub struct ManagedCore {
    pub(crate) registration: StoredCore,
    pub(crate) pre_shared_key: PresharedKey,
    pub(crate) connection: Mutex<Option<CoreConnection>>,
    pub(crate) runtime: RwLock<CoreRuntime>,
    pub(crate) reconnect: Notify,
}

impl ManagedCore {
    /// 创建已注册 Core 的内存运行时状态。
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
