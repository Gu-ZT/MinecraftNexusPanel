//! 代理实例到后端实例的独立关系模型。

use serde::Deserialize;
use serde::Serialize;

use crate::InstanceId;
use crate::ProxySubserverError;

/// 一个代理后端关系的可持久化描述。
///
/// `target_instance_id` 用于管理本机同一 Core 上的实例，`host` 和 `port`
/// 用于代理实际连接后端以及执行从 Core 发起的健康检查。关系的启用状态
/// 不会改变拓扑数量限制，只决定该后端是否参与运维动作。
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
    /// 创建并校验一个代理后端关系。
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

    /// 校验标识符、名称、主机和端口，拒绝路径型或含空白的地址。
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

    /// 返回关系标识符。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回面向用户显示的名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回被代理的实例 ID。
    #[must_use]
    pub fn target_instance_id(&self) -> &InstanceId {
        &self.target_instance_id
    }

    /// 返回代理连接后端使用的主机名或 IP 字面量。
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// 返回代理连接后端使用的 TCP 端口。
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// 返回该关系是否参与代理运维动作。
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
