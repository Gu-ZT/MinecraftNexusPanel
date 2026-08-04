//! 规范化的 SHA-256 十六进制摘要值对象。

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde::de::Error as DeserializeError;

use crate::Sha256DigestError;

/// 始终保存为 64 位小写 ASCII 十六进制的 SHA-256 摘要。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    /// 从十六进制文本创建并规范化摘要。
    pub fn from_hex(value: &str) -> Result<Self, Sha256DigestError> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Sha256DigestError::InvalidFormat);
        }

        Ok(Self(value.to_ascii_lowercase()))
    }

    /// 以字符串形式返回规范化摘要。
    /// 返回规范化后的小写十六进制摘要。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Sha256Digest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Self::from_hex(&value).map_err(DeserializeError::custom)
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for Sha256Digest {
    type Err = Sha256DigestError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_hex(value)
    }
}

impl Serialize for Sha256Digest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Sha256Digest;

    #[test]
    fn normalizes_hex_digests() {
        let digest = Sha256Digest::from_hex(
            "A09F0C219438BCD328A56E656FEC64F84D75C95BB09D97235BF38FC1B6C046AA",
        )
        .expect("digest is valid");

        assert_eq!(
            digest.as_str(),
            "a09f0c219438bcd328a56e656fec64f84d75c95bb09d97235bf38fc1b6c046aa"
        );
    }

    #[test]
    fn rejects_non_sha256_digests() {
        assert!(Sha256Digest::from_hex("not-a-digest").is_err());
    }
}
