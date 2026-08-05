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
        /// Core 试图监听的本地地址。
        address: SocketAddr,
        #[source]
        /// 操作系统返回的绑定错误。
        source: io::Error,
    },
    /// Core 数据目录创建失败。
    #[error("failed to create the Core data directory {path}")]
    CreateDataDirectory {
        /// 创建失败的数据目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的目录创建错误。
        source: io::Error,
    },
    /// 持久化的 Core 标识无法解析。
    #[error("Core identity file {path} contains an invalid identifier")]
    InvalidStoredCoreId {
        /// 包含非法标识文本的文件路径。
        path: PathBuf,
    },
    /// 未配置接受 Panel 连接所需的预共享密钥。
    #[error("Core requires MCNP_CORE_PSK to accept Panel connections")]
    MissingPreSharedKey,
    /// 读取 Core 标识文件失败。
    #[error("failed to read the Core identity file {path}")]
    ReadCoreIdentity {
        /// 读取失败的 Core 标识文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的读取错误。
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
    /// 实例生命周期审计仓储初始化或持久化失败。
    #[error(transparent)]
    InstanceAuditRepository(#[from] crate::InstanceAuditRepositoryError),
    /// 写入 Core 标识文件失败。
    #[error("failed to write the Core identity file {path}")]
    WriteCoreIdentity {
        /// 写入失败的 Core 标识文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的写入错误。
        source: io::Error,
    },
}
