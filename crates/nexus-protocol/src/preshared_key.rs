use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hkdf::Hkdf;
use sha2::Sha256;

use crate::PresharedKeyError;

const DERIVED_KEY_BYTES: usize = 32;
const MINIMUM_SECRET_BYTES: usize = 32;
const PSK_SALT: &[u8] = b"mcnp-core-psk-v1";

/// 由 Core 预共享秘密派生出的固定长度 Noise PSK。
///
/// 原始秘密不会保存在值对象中；`Debug` 实现也会隐藏派生后的密钥内容。
#[derive(Clone, Eq, PartialEq)]
pub struct PresharedKey([u8; DERIVED_KEY_BYTES]);

impl PresharedKey {
    /// 从无填充 Base64URL 编码的秘密派生 PSK。
    ///
    /// 输入至少需要 32 字节解码结果，编码中不接受 `=` 填充。
    pub fn from_base64url(value: &str) -> Result<Self, PresharedKeyError> {
        let secret = URL_SAFE_NO_PAD
            .decode(value)
            .map_err(PresharedKeyError::InvalidBase64Url)?;

        Self::from_secret(&secret)
    }

    /// 从原始秘密使用固定盐和 HKDF-SHA-256 派生 PSK。
    pub fn from_secret(secret: &[u8]) -> Result<Self, PresharedKeyError> {
        if secret.len() < MINIMUM_SECRET_BYTES {
            return Err(PresharedKeyError::SecretTooShort {
                actual: secret.len(),
            });
        }

        let key_derivation = Hkdf::<Sha256>::new(Some(PSK_SALT), secret);
        let mut derived_key = [0_u8; DERIVED_KEY_BYTES];
        key_derivation
            .expand(&[], &mut derived_key)
            .map_err(|_| PresharedKeyError::KeyDerivation)?;

        Ok(Self(derived_key))
    }

    /// 返回供 Noise 使用的 32 字节派生密钥。
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; DERIVED_KEY_BYTES] {
        &self.0
    }
}

impl fmt::Debug for PresharedKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PresharedKey(REDACTED)")
    }
}

#[cfg(test)]
mod tests {
    use super::PresharedKey;
    use crate::PresharedKeyError;

    #[test]
    fn derives_a_key_from_an_unpadded_base64url_secret() {
        let key = PresharedKey::from_base64url("MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY")
            .expect("base64url test secret is valid");

        assert_eq!(key.as_bytes().len(), 32);
    }

    #[test]
    fn rejects_a_short_secret() {
        assert_eq!(
            PresharedKey::from_secret(b"too short"),
            Err(PresharedKeyError::SecretTooShort { actual: 9 })
        );
    }
}
