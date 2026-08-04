//! 基岩版 RakNet 健康检查结果。

use serde::Deserialize;
use serde::Serialize;

use crate::BedrockBindAddressSource;
use crate::BedrockHealthStatus;
use crate::BedrockManagementKind;
use crate::BedrockPortSource;
use crate::BedrockTransport;
use crate::InstanceId;

/// Core 对基岩版端点执行 UDP 健康检查后返回的完整结果。
///
/// `bind_address` 是服务端配置声明的本地监听地址，`probe_address` 是 Core
/// 实际发送 UDP 探测的地址。对于 `0.0.0.0` 或 `::`，后者会使用本机回环地址，
/// 因为未指定地址只能用于监听，不能作为有效的远端目的地址。
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
    /// 创建一次基岩健康检查结果。
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

    /// 返回被检查的实例 ID。
    #[must_use]
    pub fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    /// 返回基岩端管理类型。
    #[must_use]
    pub const fn management_kind(&self) -> BedrockManagementKind {
        self.management_kind
    }

    /// 返回传输类型；当前基岩画像统一使用 RakNet UDP。
    #[must_use]
    pub const fn transport(&self) -> BedrockTransport {
        self.transport
    }

    /// 返回配置解析出的服务端绑定地址。
    #[must_use]
    pub fn bind_address(&self) -> &str {
        &self.bind_address
    }

    /// 返回绑定地址来自配置还是画像默认值。
    #[must_use]
    pub const fn bind_address_source(&self) -> BedrockBindAddressSource {
        self.bind_address_source
    }

    /// 返回配置解析出的 UDP 端口。
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// 返回端口来自配置还是画像默认值。
    #[must_use]
    pub const fn port_source(&self) -> BedrockPortSource {
        self.port_source
    }

    /// 返回 Core 实际发送探测包的 IP 地址。
    #[must_use]
    pub fn probe_address(&self) -> &str {
        &self.probe_address
    }

    /// 返回 RakNet 探测状态。
    #[must_use]
    pub const fn status(&self) -> BedrockHealthStatus {
        self.status
    }

    /// 表示 UDP 端点是否返回了可用响应。
    #[must_use]
    pub const fn reachable(&self) -> bool {
        self.reachable
    }

    /// 返回从发送探测到收到结果的耗时，单位为毫秒。
    #[must_use]
    pub const fn latency_ms(&self) -> Option<u64> {
        self.latency_ms
    }

    /// 返回合法 Pong 中的 Bedrock 服务端身份字符串。
    #[must_use]
    pub fn server_identity(&self) -> Option<&str> {
        self.server_identity.as_deref()
    }

    /// 返回检查完成时间。
    #[must_use]
    pub fn checked_at(&self) -> &str {
        &self.checked_at
    }

    /// 返回配置、网络或协议错误分类。
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
