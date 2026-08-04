/// Panel 观察到的 Core 连接状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreStatus {
    /// 认证失败。
    AuthFailed,
    /// 协议版本不兼容。
    Incompatible,
    /// 当前不可连接。
    Offline,
    /// 最近一次探测成功。
    Online,
    /// 尚未有足够信息判断状态。
    Unknown,
}

impl CoreStatus {
    /// 返回 API 使用的稳定大写状态文本。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AuthFailed => "AUTH_FAILED",
            Self::Incompatible => "INCOMPATIBLE",
            Self::Offline => "OFFLINE",
            Self::Online => "ONLINE",
            Self::Unknown => "UNKNOWN",
        }
    }
}
