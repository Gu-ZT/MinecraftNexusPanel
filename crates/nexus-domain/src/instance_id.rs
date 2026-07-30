use std::fmt;
use std::str::FromStr;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde::de::Error;

use crate::InstanceIdError;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InstanceId(String);

impl InstanceId {
    pub fn new(value: String) -> Result<Self, InstanceIdError> {
        value.parse()
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InstanceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for InstanceId {
    type Err = InstanceIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let bytes = value.as_bytes();
        let is_valid = (1..=64).contains(&bytes.len())
            && bytes[0].is_ascii_alphanumeric()
            && bytes
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));

        if is_valid {
            Ok(Self(value.to_owned()))
        } else {
            Err(InstanceIdError::InvalidFormat)
        }
    }
}

impl Serialize for InstanceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for InstanceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        value.parse().map_err(Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::InstanceId;

    #[test]
    fn accepts_a_portable_instance_identifier() {
        assert!("survival-1.20_4".parse::<InstanceId>().is_ok());
    }

    #[test]
    fn rejects_an_unsafe_instance_identifier() {
        assert!("../survival".parse::<InstanceId>().is_err());
        assert!("survival/backup".parse::<InstanceId>().is_err());
        assert!("survival ".parse::<InstanceId>().is_err());
    }
}
