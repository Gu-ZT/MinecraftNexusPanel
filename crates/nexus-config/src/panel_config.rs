use std::fmt;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;

use crate::ConfigError;
use crate::DesktopSessionConfig;
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
    desktop_session: Option<DesktopSessionConfig>,
    local_core: Option<LocalCoreConfig>,
    master_key: Option<PanelMasterKey>,
    web_root: Option<PathBuf>,
    audit_retention_events: usize,
}

impl fmt::Debug for PanelConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PanelConfig")
            .field("listen_address", &self.listen_address)
            .field("data_directory", &self.data_directory)
            .field("initial_admin", &self.initial_admin)
            .field("desktop_session", &self.desktop_session)
            .field("local_core", &self.local_core.as_ref().map(|_| "REDACTED"))
            .field("master_key", &self.master_key)
            .field("web_root", &self.web_root)
            .field("audit_retention_events", &self.audit_retention_events)
            .finish()
    }
}

impl PanelConfig {
    /// Panel 默认监听地址。
    pub const DEFAULT_LISTEN_ADDRESS: &'static str = "127.0.0.1:8080";
    /// 默认保留的 Panel 请求审计事件数量。
    pub const DEFAULT_AUDIT_RETENTION_EVENTS: usize = 10_000;
    /// 允许配置的最小审计保留数量。
    pub const MIN_AUDIT_RETENTION_EVENTS: usize = 100;
    /// 允许配置的最大审计保留数量。
    pub const MAX_AUDIT_RETENTION_EVENTS: usize = 100_000;

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
            desktop_session: None,
            local_core: None,
            master_key: None,
            web_root: None,
            audit_retention_events: Self::DEFAULT_AUDIT_RETENTION_EVENTS,
        })
    }

    /// 设置首次初始化管理员凭据。
    #[must_use]
    pub fn with_initial_admin(mut self, initial_admin: InitialAdminConfig) -> Self {
        self.initial_admin = Some(initial_admin);
        self
    }

    /// 设置仅供本地 Tauri sidecar 使用的 Desktop 会话引导凭据。
    #[must_use]
    pub fn with_desktop_session(mut self, desktop_session: DesktopSessionConfig) -> Self {
        self.desktop_session = Some(desktop_session);
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

    /// 设置由 Panel 同源托管的 WebUI 静态资源目录。
    #[must_use]
    pub fn with_web_root(mut self, web_root: PathBuf) -> Self {
        self.web_root = Some(web_root);
        self
    }

    /// 设置 Panel 用户级审计事件保留数量。
    pub fn with_audit_retention_events(
        mut self,
        audit_retention_events: usize,
    ) -> Result<Self, ConfigError> {
        if !(Self::MIN_AUDIT_RETENTION_EVENTS..=Self::MAX_AUDIT_RETENTION_EVENTS)
            .contains(&audit_retention_events)
        {
            return Err(ConfigError::InvalidPanelAuditRetention {
                value: audit_retention_events.to_string(),
            });
        }
        self.audit_retention_events = audit_retention_events;
        Ok(self)
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

    /// 返回 Desktop 本地会话引导配置；普通 Panel 部署为 `None`。
    #[must_use]
    pub const fn desktop_session(&self) -> Option<&DesktopSessionConfig> {
        self.desktop_session.as_ref()
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

    /// 返回 WebUI 静态资源目录；未配置时 Panel 只提供 API。
    #[must_use]
    pub fn web_root(&self) -> Option<&Path> {
        self.web_root.as_deref()
    }

    /// 返回 Panel 用户级审计事件保留数量。
    #[must_use]
    pub const fn audit_retention_events(&self) -> usize {
        self.audit_retention_events
    }
}
