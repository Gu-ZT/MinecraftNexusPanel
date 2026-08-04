use base64::DecodeError;
use thiserror::Error;

/// PSK 输入或派生失败的原因。
#[derive(Debug, Error, Eq, PartialEq)]
pub enum PresharedKeyError {
    /// 输入不是无填充 Base64URL。
    #[error("pre-shared key is not valid unpadded Base64URL")]
    InvalidBase64Url(#[source] DecodeError),
    /// 解码后的原始秘密短于最低长度。
    #[error("pre-shared key must contain at least 32 bytes; received {actual}")]
    SecretTooShort { actual: usize },
    /// HKDF 无法生成目标长度的派生密钥。
    #[error("could not derive the pre-shared key")]
    KeyDerivation,
}
