//! 基岩版 UDP 监听端口检查结果。

use serde::Deserialize;
use serde::Serialize;

use crate::BedrockBindAddressSource;
use crate::BedrockManagementKind;
use crate::BedrockPortCheckState;
use crate::BedrockPortSource;
use crate::BedrockTransport;
use crate::InstanceId;

/// Core 尝试绑定基岩端监听地址和端口后返回的结果。
///
/// 该结果只说明端口是否可以被当前 Core 绑定，不说明已有服务端是否
/// 能够理解 RakNet 协议；协议级存活检查使用 [`crate::BedrockHealth`]。
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
    /// 创建一次 UDP 端口绑定检查结果。
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

    /// 返回被检查的实例 ID。
    #[must_use]
    pub fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    /// 返回基岩管理类型。
    #[must_use]
    pub const fn management_kind(&self) -> BedrockManagementKind {
        self.management_kind
    }

    /// 返回传输类型。
    #[must_use]
    pub const fn transport(&self) -> BedrockTransport {
        self.transport
    }

    /// 返回尝试绑定的地址。
    #[must_use]
    pub fn bind_address(&self) -> &str {
        &self.bind_address
    }

    /// 返回地址来源。
    #[must_use]
    pub const fn bind_address_source(&self) -> BedrockBindAddressSource {
        self.bind_address_source
    }

    /// 返回尝试绑定的端口。
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// 返回端口来源。
    #[must_use]
    pub const fn port_source(&self) -> BedrockPortSource {
        self.port_source
    }

    /// 返回绑定状态。
    #[must_use]
    pub const fn state(&self) -> BedrockPortCheckState {
        self.state
    }

    /// 表示该地址和端口是否可绑定。
    #[must_use]
    pub const fn available(&self) -> bool {
        self.available
    }

    /// 返回检查完成时间。
    #[must_use]
    pub fn checked_at(&self) -> &str {
        &self.checked_at
    }

    /// 返回配置或系统绑定错误分类。
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
