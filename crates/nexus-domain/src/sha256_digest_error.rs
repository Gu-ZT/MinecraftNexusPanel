//! SHA-256 摘要格式错误。

use thiserror::Error;

/// 传入值不是 64 位十六进制 SHA-256 摘要。
#[derive(Debug, Error, Eq, PartialEq)]
pub enum Sha256DigestError {
    #[error("SHA-256 digest must contain exactly 64 hexadecimal characters")]
    InvalidFormat,
}
