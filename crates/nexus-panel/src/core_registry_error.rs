use nexus_domain::CoreId;
use nexus_protocol::PresharedKeyError;
use nexus_storage::StorageError;
use serde_json::Error as JsonError;
use thiserror::Error;
use tokio::task::JoinError;

use crate::CoreConnectionError;
use crate::SecretCipherError;

/// Core 注册、连接、持久化和重连错误。
#[derive(Debug, Error)]
pub enum CoreRegistryError {
    /// Core 标识已经注册。
    #[error("Core {core_id} is already registered")]
    AlreadyExists { core_id: CoreId },
    /// Core 秘密加解密失败。
    #[error(transparent)]
    Cipher(#[from] SecretCipherError),
    /// Core 连接或协议请求失败。
    #[error(transparent)]
    Connection(#[from] CoreConnectionError),
    /// 建立 Core 连接超时。
    #[error("Core connection timed out")]
    ConnectionTimeout,
    /// 当前没有可用的已建立连接。
    #[error("Core connection is unavailable")]
    ConnectionUnavailable,
    /// 注册请求字段无效。
    #[error("invalid Core registration field: {field}")]
    InvalidRequest { field: &'static str },
    /// 数据库中的 Core 注册记录无法解析。
    #[error("stored Core registration is invalid: {core_id}")]
    InvalidStoredCore { core_id: String },
    /// 数据库中的扩展安装记录无法解析。
    #[error("stored extension installation is invalid: {path}")]
    InvalidStoredExtension { path: String },
    /// Core 秘密不是合法的无填充 Base64URL PSK。
    #[error("Core secret must be valid unpadded Base64URL containing at least 32 bytes")]
    InvalidSecret(#[source] PresharedKeyError),
    /// 本地 Core 返回的标识与配置不一致。
    #[error("loopback Core returned unexpected identity: expected {expected}, got {actual}")]
    LocalCoreIdMismatch { expected: CoreId, actual: CoreId },
    /// Core 注册不存在。
    #[error("Core registration does not exist: {core_id}")]
    NotFound { core_id: CoreId },
    /// 标签或扩展存储 JSON 无法解析。
    #[error("stored Core tags are invalid")]
    Serialization(#[from] JsonError),
    /// SQLite 存储失败。
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// 后台连接监视任务失败。
    #[error("Core registry worker failed")]
    Task(#[from] JoinError),
}
