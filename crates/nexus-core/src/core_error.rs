use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;

use nexus_protocol::SessionError;
use thiserror::Error;

use crate::CoreTlsIdentityError;

/// Core 启动、连接接受或请求编排失败。
#[derive(Debug, Error)]
pub enum CoreError {
    /// TCP 监听器接受连接失败。
    #[error("failed to accept a Core TCP connection")]
    Accept(#[source] io::Error),
    /// TCP 监听地址绑定失败。
    #[error("failed to bind the Core TCP listener at {address}")]
    Bind {
        address: SocketAddr,
        #[source]
        source: io::Error,
    },
    /// Core 数据目录创建失败。
    #[error("failed to create the Core data directory {path}")]
    CreateDataDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 持久化的 Core 标识无法解析。
    #[error("Core identity file {path} contains an invalid identifier")]
    InvalidStoredCoreId { path: PathBuf },
    /// 未配置接受 Panel 连接所需的预共享密钥。
    #[error("Core requires MCNP_CORE_PSK to accept Panel connections")]
    MissingPreSharedKey,
    /// 读取 Core 标识文件失败。
    #[error("failed to read the Core identity file {path}")]
    ReadCoreIdentity {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 安全会话建立或传输失败。
    #[error(transparent)]
    Session(#[from] SessionError),
    /// TLS 身份加载或生成失败。
    #[error(transparent)]
    TlsIdentity(#[from] CoreTlsIdentityError),
    /// 运行时管理器初始化或执行失败。
    #[error(transparent)]
    RuntimeManager(#[from] crate::RuntimeManagerError),
    /// 一键搭建管理器初始化或执行失败。
    #[error(transparent)]
    ProvisionManager(#[from] crate::ProvisionManagerError),
    /// 写入 Core 标识文件失败。
    #[error("failed to write the Core identity file {path}")]
    WriteCoreIdentity {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
