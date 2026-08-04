use serde::Deserialize;
use serde::Serialize;

use crate::InstanceId;
use crate::ProxySubserverHealthStatus;
use crate::ProxySubserverProtocolStatus;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxySubserverHealth {
    subserver_id: String,
    target_instance_id: InstanceId,
    host: String,
    port: u16,
    enabled: bool,
    status: ProxySubserverHealthStatus,
    protocol_status: ProxySubserverProtocolStatus,
    reachable: Option<bool>,
    latency_ms: Option<u64>,
    checked_at: String,
    error: Option<String>,
}

impl ProxySubserverHealth {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subserver_id: String,
        target_instance_id: InstanceId,
        host: String,
        port: u16,
        enabled: bool,
        status: ProxySubserverHealthStatus,
        protocol_status: ProxySubserverProtocolStatus,
        reachable: Option<bool>,
        latency_ms: Option<u64>,
        checked_at: String,
        error: Option<String>,
    ) -> Self {
        Self {
            subserver_id,
            target_instance_id,
            host,
            port,
            enabled,
            status,
            protocol_status,
            reachable,
            latency_ms,
            checked_at,
            error,
        }
    }

    #[must_use]
    pub fn subserver_id(&self) -> &str {
        &self.subserver_id
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

    #[must_use]
    pub const fn status(&self) -> ProxySubserverHealthStatus {
        self.status
    }

    #[must_use]
    pub const fn protocol_status(&self) -> ProxySubserverProtocolStatus {
        self.protocol_status
    }

    #[must_use]
    pub const fn reachable(&self) -> Option<bool> {
        self.reachable
    }

    #[must_use]
    pub const fn latency_ms(&self) -> Option<u64> {
        self.latency_ms
    }

    #[must_use]
    pub fn checked_at(&self) -> &str {
        &self.checked_at
    }

    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
