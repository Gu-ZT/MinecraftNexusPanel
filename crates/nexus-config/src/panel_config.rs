use std::fmt;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;

use crate::ConfigError;
use crate::InitialAdminConfig;
use crate::LocalCoreConfig;
use crate::PanelMasterKey;

/// Panel HTTP 服务、数据目录和身份初始化配置。
///
/// `local_core` 只在单进程或本机托管场景使用；远程 Core 注册信息由 Panel
/// 其他领域服务管理。初始管理员和主密钥均为可选输入，最终启用方式由启动模式决定。
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
    /// Panel 默认监听地址。
    pub const DEFAULT_LISTEN_ADDRESS: &'static str = "127.0.0.1:8080";

    /// 解析 Panel 监听地址并创建基础配置。
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

    /// 设置首次初始化管理员凭据。
    #[must_use]
    pub fn with_initial_admin(mut self, initial_admin: InitialAdminConfig) -> Self {
        self.initial_admin = Some(initial_admin);
        self
    }

    /// 设置 Panel 用于加密持久化秘密的主密钥。
    #[must_use]
    pub fn with_master_key(mut self, master_key: PanelMasterKey) -> Self {
        self.master_key = Some(master_key);
        self
    }

    /// 设置内置本地 Core 的连接配置。
    #[must_use]
    pub fn with_local_core(mut self, local_core: LocalCoreConfig) -> Self {
        self.local_core = Some(local_core);
        self
    }

    /// 返回 Panel HTTP 监听地址。
    #[must_use]
    pub const fn listen_address(&self) -> SocketAddr {
        self.listen_address
    }

    /// 返回 Panel 数据目录。
    #[must_use]
    pub fn data_directory(&self) -> &Path {
        &self.data_directory
    }

    /// 返回初始管理员配置；未配置时为 `None`。
    #[must_use]
    pub const fn initial_admin(&self) -> Option<&InitialAdminConfig> {
        self.initial_admin.as_ref()
    }

    /// 返回本地 Core 配置；未配置时为 `None`。
    #[must_use]
    pub const fn local_core(&self) -> Option<&LocalCoreConfig> {
        self.local_core.as_ref()
    }

    /// 返回 Panel 主密钥；未配置时为 `None`。
    #[must_use]
    pub const fn master_key(&self) -> Option<&PanelMasterKey> {
        self.master_key.as_ref()
    }
}
