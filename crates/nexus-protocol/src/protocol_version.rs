use serde::Deserialize;
use serde::Serialize;

use crate::ProtocolVersionError;

/// MCNP 协议的主版本和次版本。
///
/// 主版本不同表示线协议不兼容；主版本相同则协商双方较小的次版本，
/// 以便新端点向后兼容旧端点支持的能力集合。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolVersion {
    /// 不兼容变更计数。
    pub major: u16,
    /// 向后兼容能力扩展计数。
    pub minor: u16,
}

impl ProtocolVersion {
    /// 创建协议版本值。
    #[must_use]
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// 与远端版本协商共同使用的版本。
    ///
    /// 主版本必须相同；次版本取双方较小值。协商结果只表示线协议版本，
    /// 具体业务方法仍需由双方按版本能力处理。
    pub fn negotiate(self, remote: Self) -> Result<Self, ProtocolVersionError> {
        if self.major != remote.major {
            return Err(ProtocolVersionError::MajorMismatch {
                local: self.major,
                remote: remote.major,
            });
        }

        Ok(Self::new(self.major, self.minor.min(remote.minor)))
    }
}

#[cfg(test)]
mod tests {
    use super::ProtocolVersion;
    use crate::ProtocolVersionError;

    #[test]
    fn selects_the_lower_compatible_minor_version() {
        let local = ProtocolVersion::new(1, 4);
        let remote = ProtocolVersion::new(1, 2);

        assert_eq!(local.negotiate(remote), Ok(ProtocolVersion::new(1, 2)));
    }

    #[test]
    fn rejects_an_incompatible_major_version() {
        let local = ProtocolVersion::new(1, 0);
        let remote = ProtocolVersion::new(2, 0);

        assert_eq!(
            local.negotiate(remote),
            Err(ProtocolVersionError::MajorMismatch {
                local: 1,
                remote: 2,
            })
        );
    }
}
