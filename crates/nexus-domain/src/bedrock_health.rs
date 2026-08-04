use serde::Deserialize;
use serde::Serialize;

use crate::BedrockBindAddressSource;
use crate::BedrockHealthStatus;
use crate::BedrockManagementKind;
use crate::BedrockPortSource;
use crate::BedrockTransport;
use crate::InstanceId;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockHealth {
    instance_id: InstanceId,
    management_kind: BedrockManagementKind,
    transport: BedrockTransport,
    bind_address: String,
    bind_address_source: BedrockBindAddressSource,
    port: u16,
    port_source: BedrockPortSource,
    probe_address: String,
    status: BedrockHealthStatus,
    reachable: bool,
    latency_ms: Option<u64>,
    server_identity: Option<String>,
    checked_at: String,
    error: Option<String>,
}

impl BedrockHealth {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instance_id: InstanceId,
        management_kind: BedrockManagementKind,
        transport: BedrockTransport,
        bind_address: String,
        bind_address_source: BedrockBindAddressSource,
        port: u16,
        port_source: BedrockPortSource,
        probe_address: String,
        status: BedrockHealthStatus,
        reachable: bool,
        latency_ms: Option<u64>,
        server_identity: Option<String>,
        checked_at: String,
        error: Option<String>,
    ) -> Self {
        Self {
            instance_id,
            management_kind,
            transport,
            bind_address,
            bind_address_source,
            port,
            port_source,
            probe_address,
            status,
            reachable,
            latency_ms,
            server_identity,
            checked_at,
            error,
        }
    }

    #[must_use]
    pub fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    #[must_use]
    pub const fn management_kind(&self) -> BedrockManagementKind {
        self.management_kind
    }

    #[must_use]
    pub const fn transport(&self) -> BedrockTransport {
        self.transport
    }

    #[must_use]
    pub fn bind_address(&self) -> &str {
        &self.bind_address
    }

    #[must_use]
    pub const fn bind_address_source(&self) -> BedrockBindAddressSource {
        self.bind_address_source
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub const fn port_source(&self) -> BedrockPortSource {
        self.port_source
    }

    #[must_use]
    pub fn probe_address(&self) -> &str {
        &self.probe_address
    }

    #[must_use]
    pub const fn status(&self) -> BedrockHealthStatus {
        self.status
    }

    #[must_use]
    pub const fn reachable(&self) -> bool {
        self.reachable
    }

    #[must_use]
    pub const fn latency_ms(&self) -> Option<u64> {
        self.latency_ms
    }

    #[must_use]
    pub fn server_identity(&self) -> Option<&str> {
        self.server_identity.as_deref()
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
