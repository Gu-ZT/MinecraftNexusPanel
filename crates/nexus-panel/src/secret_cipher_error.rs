use getrandom::Error as RandomError;
use thiserror::Error;

/// Core 秘密信封加密或解密错误。
#[derive(Debug, Error)]
pub enum SecretCipherError {
    /// 密文认证失败或关联数据不匹配。
    #[error("Core secret envelope could not be authenticated")]
    Authentication,
    /// 信封版本、长度或布局不受支持。
    #[error("Core secret envelope has an unsupported or malformed format")]
    InvalidEnvelope,
    /// 随机 nonce 生成失败。
    #[error("failed to generate a Core secret nonce")]
    Random(#[from] RandomError),
}
