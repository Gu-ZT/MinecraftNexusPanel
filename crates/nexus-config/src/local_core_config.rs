use std::fmt;
use std::net::SocketAddr;

use nexus_domain::CoreId;

/// Panel 连接内置本地 Core 时使用的连接配置。
///
/// 预共享密钥只以编码文本形式保存，`Debug` 输出固定脱敏；Core 标识用于
/// 将本地连接与持久化注册信息关联。
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
    /// 创建本地 Core 连接配置。
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

    /// 返回本地 Core 标识。
    #[must_use]
    pub const fn core_id(&self) -> CoreId {
        self.core_id
    }

    /// 返回本地 Core 监听地址。
    #[must_use]
    pub const fn listen_address(&self) -> SocketAddr {
        self.listen_address
    }

    /// 返回 Base64URL 编码的预共享密钥。
    #[must_use]
    pub fn encoded_pre_shared_key(&self) -> &str {
        &self.encoded_pre_shared_key
    }
}
