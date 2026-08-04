use serde::Deserialize;
use serde::Serialize;

use crate::BedrockBindAddressSource;
use crate::BedrockManagementKind;
use crate::BedrockPortCheckState;
use crate::BedrockPortSource;
use crate::BedrockTransport;
use crate::InstanceId;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockPortCheck {
    instance_id: InstanceId,
    management_kind: BedrockManagementKind,
    transport: BedrockTransport,
    bind_address: String,
    bind_address_source: BedrockBindAddressSource,
    port: u16,
    port_source: BedrockPortSource,
    state: BedrockPortCheckState,
    available: bool,
    checked_at: String,
    error: Option<String>,
}

impl BedrockPortCheck {
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
        state: BedrockPortCheckState,
        available: bool,
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
            state,
            available,
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
    pub const fn state(&self) -> BedrockPortCheckState {
        self.state
    }

    #[must_use]
    pub const fn available(&self) -> bool {
        self.available
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
