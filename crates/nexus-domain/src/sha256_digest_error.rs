use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum Sha256DigestError {
    #[error("SHA-256 digest must contain exactly 64 hexadecimal characters")]
    InvalidFormat,
}
