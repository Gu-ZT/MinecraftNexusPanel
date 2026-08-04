use argon2::password_hash::Error as PasswordHashError;
use nexus_storage::StorageError;
use thiserror::Error;
use tokio::task::JoinError;

/// 登录、会话、CSRF 和限流错误。
#[derive(Debug, Error)]
pub enum AuthError {
    /// CSRF 令牌与浏览器会话不匹配。
    #[error("CSRF token is invalid")]
    InvalidCsrfToken,
    /// 用户名或密码不正确。
    #[error("credentials are invalid")]
    InvalidCredentials,
    /// 会话不存在、已过期或客户端类型不匹配。
    #[error("session is invalid or expired")]
    InvalidSession,
    /// Argon2 密码哈希处理失败。
    #[error("failed to process a password hash")]
    PasswordHash(#[from] PasswordHashError),
    /// 系统安全随机数生成失败。
    #[error("failed to generate a secure credential")]
    Random(#[from] getrandom::Error),
    /// 登录限流状态锁不可用。
    #[error("login rate limiter lock is poisoned")]
    RateLimitLock,
    /// 登录尝试超过账户或来源 IP 限制。
    #[error("too many login attempts")]
    RateLimited {
        /// 客户端再次尝试登录前应等待的秒数。
        retry_after_seconds: u64,
    },
    /// 已轮换的刷新凭据被再次使用。
    #[error("a rotated refresh credential was reused")]
    RefreshReused,
    /// 认证持久化操作失败。
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// 后台认证任务无法完成。
    #[error("authentication worker failed")]
    Task(#[from] JoinError),
}
