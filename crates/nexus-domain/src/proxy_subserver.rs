use serde::Deserialize;
use serde::Serialize;

use crate::InstanceId;
use crate::ProxySubserverError;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxySubserver {
    id: String,
    name: String,
    target_instance_id: InstanceId,
    host: String,
    port: u16,
    enabled: bool,
}

impl ProxySubserver {
    pub fn new(
        id: String,
        name: String,
        target_instance_id: InstanceId,
        host: String,
        port: u16,
        enabled: bool,
    ) -> Result<Self, ProxySubserverError> {
        let subserver = Self {
            id,
            name,
            target_instance_id,
            host,
            port,
            enabled,
        };
        subserver.validate()?;

        Ok(subserver)
    }

    pub fn validate(&self) -> Result<(), ProxySubserverError> {
        if !is_valid_identifier(&self.id) {
            return Err(ProxySubserverError::InvalidId);
        }
        if !is_valid_name(&self.name) {
            return Err(ProxySubserverError::InvalidName);
        }
        if !is_valid_host(&self.host) {
            return Err(ProxySubserverError::InvalidHost);
        }
        if self.port == 0 {
            return Err(ProxySubserverError::InvalidPort);
        }

        Ok(())
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn target_instance_id(&self) -> &InstanceId {
        &self.target_instance_id
    }

    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

fn is_valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn is_valid_name(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value == value.trim() && !value.contains('\0')
}

fn is_valid_host(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value == value.trim()
        && !value.chars().any(|character| {
            character.is_ascii_whitespace() || matches!(character, '\0' | '/' | '\\')
        })
}

#[cfg(test)]
mod tests {
    use super::ProxySubserver;
    use crate::InstanceId;

    #[test]
    fn validates_a_managed_proxy_target() {
        let target = InstanceId::new("survival".to_owned()).expect("target ID is valid");
        let subserver = ProxySubserver::new(
            "survival".to_owned(),
            "Survival".to_owned(),
            target.clone(),
            "127.0.0.1".to_owned(),
            25565,
            true,
        )
        .expect("proxy subserver is valid");

        assert_eq!(subserver.target_instance_id(), &target);
        assert_eq!(subserver.port(), 25565);
    }

    #[test]
    fn rejects_unsafe_proxy_addresses() {
        let target = InstanceId::new("survival".to_owned()).expect("target ID is valid");
        let result = ProxySubserver::new(
            "survival".to_owned(),
            "Survival".to_owned(),
            target,
            "127.0.0.1/../other".to_owned(),
            25565,
            true,
        );

        assert!(result.is_err());
    }
}
