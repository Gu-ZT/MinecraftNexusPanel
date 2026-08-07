use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;

use nexus_storage::StorageError;
use thiserror::Error;

use crate::AuthError;
use crate::CoreRegistryError;
use crate::VersionMetadataError;
use crate::extension_source_error::ExtensionSourceError;

/// Panel 启动、路由依赖和外部服务错误。
#[derive(Debug, Error)]
pub enum PanelError {
    /// 认证服务失败。
    #[error(transparent)]
    Auth(#[from] AuthError),
    /// Core 注册服务失败。
    #[error(transparent)]
    CoreRegistry(#[from] CoreRegistryError),
    /// HTTP 监听地址绑定失败。
    #[error("failed to bind the Panel HTTP listener at {address}")]
    Bind {
        /// Panel 试图监听的本地 HTTP 地址。
        address: SocketAddr,
        #[source]
        /// 操作系统返回的监听器绑定错误。
        source: io::Error,
    },
    /// HTTP 服务器运行失败。
    #[error("Panel HTTP server failed")]
    Serve(#[source] io::Error),
    /// 未配置用于加密 Core 秘密的 Panel 主密钥。
    #[error("MCNP_PANEL_MASTER_KEY is required to encrypt registered Core secrets")]
    MissingPanelMasterKey,
    /// 配置的 WebUI 目录不包含可用的 SPA 入口。
    #[error("Panel WebUI entry file does not exist: {path}")]
    MissingWebEntry {
        /// Panel 期望读取的 `index.html` 路径。
        path: PathBuf,
    },
    /// 扩展来源服务初始化或请求失败。
    #[error(transparent)]
    ExtensionSource(#[from] ExtensionSourceError),
    /// Panel SQLite 存储失败。
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// 版本元数据服务初始化或请求失败。
    #[error(transparent)]
    VersionMetadata(#[from] VersionMetadataError),
}
