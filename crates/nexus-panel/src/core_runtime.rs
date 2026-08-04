use crate::CoreStatus;

/// Panel 维护的 Core 运行时连接快照。
///
/// 连接状态和心跳观测独立于持久化注册配置；证书验证结果与指纹用于诊断当前
/// 连接，不代表注册秘密本身可被读取。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreRuntime {
    pub(crate) status: CoreStatus,
    pub(crate) latency_milliseconds: Option<u64>,
    pub(crate) last_seen_at: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) protocol_version: Option<String>,
    pub(crate) capabilities: Vec<String>,
    pub(crate) certificate_verified: Option<bool>,
    pub(crate) tls_certificate_sha256: Option<String>,
}

impl CoreRuntime {
    /// 创建未知状态的初始快照。
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            status: CoreStatus::Unknown,
            latency_milliseconds: None,
            last_seen_at: None,
            version: None,
            protocol_version: None,
            capabilities: Vec::new(),
            certificate_verified: None,
            tls_certificate_sha256: None,
        }
    }

    /// 记录连接失败并更新状态。
    pub fn mark_failure(&mut self, status: CoreStatus) {
        self.status = status;
        self.latency_milliseconds = None;
    }

    /// 标记正在重连，清除当前延迟观测。
    pub fn mark_reconnecting(&mut self) {
        self.status = CoreStatus::Unknown;
        self.latency_milliseconds = None;
    }

    /// 记录一次成功心跳及其延迟。
    pub fn mark_ping(&mut self, latency_milliseconds: u64, last_seen_at: String) {
        self.status = CoreStatus::Online;
        self.latency_milliseconds = Some(latency_milliseconds);
        self.last_seen_at = Some(last_seen_at);
    }
}
