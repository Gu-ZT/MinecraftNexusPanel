use serde::Deserialize;
use serde::Serialize;

use crate::ProtocolVersionError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    #[must_use]
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

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
