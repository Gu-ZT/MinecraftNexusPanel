use argon2::password_hash::Error as PasswordHashError;
use nexus_storage::StorageError;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("CSRF token is invalid")]
    InvalidCsrfToken,
    #[error("credentials are invalid")]
    InvalidCredentials,
    #[error("session is invalid or expired")]
    InvalidSession,
    #[error("failed to process a password hash")]
    PasswordHash(#[from] PasswordHashError),
    #[error("failed to generate a secure credential")]
    Random(#[from] getrandom::Error),
    #[error("login rate limiter lock is poisoned")]
    RateLimitLock,
    #[error("too many login attempts")]
    RateLimited { retry_after_seconds: u64 },
    #[error("a rotated refresh credential was reused")]
    RefreshReused,
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("authentication worker failed")]
    Task(#[from] JoinError),
}
