//! 代理后端网络和 Minecraft Status 健康结果。

use serde::Deserialize;
use serde::Serialize;

use crate::InstanceId;
use crate::ProxySubserverHealthStatus;
use crate::ProxySubserverProtocolStatus;

/// 从登记 Core 发起的代理后端健康检查结果。
///
/// 网络可达和 Minecraft Status 协议可用是两个独立维度；TCP 已连接但
/// 返回非法协议数据时，`status` 仍为 `Reachable`，而 `protocol_status`
/// 为 `InvalidResponse`。
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
    /// 创建一个代理后端健康结果。
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

    /// 返回关系 ID。
    #[must_use]
    pub fn subserver_id(&self) -> &str {
        &self.subserver_id
    }

    /// 返回后端实例 ID。
    #[must_use]
    pub fn target_instance_id(&self) -> &InstanceId {
        &self.target_instance_id
    }

    /// 返回被探测的主机。
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// 返回被探测的端口。
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// 返回关系是否启用。
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// 返回 TCP 网络层状态。
    #[must_use]
    pub const fn status(&self) -> ProxySubserverHealthStatus {
        self.status
    }

    /// 返回 Minecraft Status 协议状态。
    #[must_use]
    pub const fn protocol_status(&self) -> ProxySubserverProtocolStatus {
        self.protocol_status
    }

    /// 返回 TCP 是否可达；禁用关系时为 `None`。
    #[must_use]
    pub const fn reachable(&self) -> Option<bool> {
        self.reachable
    }

    /// 返回探测延迟，单位为毫秒。
    #[must_use]
    pub const fn latency_ms(&self) -> Option<u64> {
        self.latency_ms
    }

    /// 返回检查完成时间。
    #[must_use]
    pub fn checked_at(&self) -> &str {
        &self.checked_at
    }

    /// 返回受限的网络或协议错误分类。
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
