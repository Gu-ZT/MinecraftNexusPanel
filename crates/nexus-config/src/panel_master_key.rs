use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::ConfigError;

const PANEL_MASTER_KEY_BYTES: usize = 32;

/// Panel 用于加密持久化秘密的 32 字节主密钥。
///
/// 该类型只暴露给加密实现使用；`Debug` 输出会隐藏所有密钥字节。
#[derive(Clone, Eq, PartialEq)]
pub struct PanelMasterKey([u8; PANEL_MASTER_KEY_BYTES]);

impl PanelMasterKey {
    /// 从无填充 Base64URL 文本解析主密钥。
    pub fn from_base64url(value: &str) -> Result<Self, ConfigError> {
        let decoded = URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| ConfigError::InvalidPanelMasterKey)?;
        let bytes = decoded
            .try_into()
            .map_err(|_| ConfigError::InvalidPanelMasterKey)?;

        Ok(Self(bytes))
    }

    /// 从恰好 32 字节的数组创建主密钥。
    #[must_use]
    pub const fn from_bytes(bytes: [u8; PANEL_MASTER_KEY_BYTES]) -> Self {
        Self(bytes)
    }

    /// 返回原始主密钥字节供加密实现使用。
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; PANEL_MASTER_KEY_BYTES] {
        &self.0
    }
}

impl fmt::Debug for PanelMasterKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PanelMasterKey(REDACTED)")
    }
}

#[cfg(test)]
mod tests {
    use super::PanelMasterKey;
    use crate::ConfigError;

    #[test]
    fn accepts_exactly_32_base64url_encoded_bytes() {
        let key = PanelMasterKey::from_base64url("MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY")
            .expect("test Panel master key is valid");

        assert_eq!(key.as_bytes().len(), 32);
        assert_eq!(format!("{key:?}"), "PanelMasterKey(REDACTED)");
    }

    #[test]
    fn rejects_keys_with_the_wrong_length() {
        assert_eq!(
            PanelMasterKey::from_base64url("dG9vLXNob3J0"),
            Err(ConfigError::InvalidPanelMasterKey)
        );
    }
}
