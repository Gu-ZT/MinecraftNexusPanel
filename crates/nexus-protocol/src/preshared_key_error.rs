use base64::DecodeError;
use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum PresharedKeyError {
    #[error("pre-shared key is not valid unpadded Base64URL")]
    InvalidBase64Url(#[source] DecodeError),
    #[error("pre-shared key must contain at least 32 bytes; received {actual}")]
    SecretTooShort { actual: usize },
    #[error("could not derive the pre-shared key")]
    KeyDerivation,
}
