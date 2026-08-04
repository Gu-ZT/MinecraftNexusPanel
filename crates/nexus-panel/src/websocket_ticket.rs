use time::OffsetDateTime;

use crate::RequestCredential;

/// 一次性 WebSocket 连接票据及其到期时间。
///
/// 票据只在内存中短期保存，消费后立即删除；关联凭据仍需在升级连接时重新鉴权。
#[derive(Clone)]
pub(crate) struct WebSocketTicket {
    credential: RequestCredential,
    expires_at: OffsetDateTime,
}

impl WebSocketTicket {
    /// 创建 WebSocket 票据记录。
    #[must_use]
    pub const fn new(credential: RequestCredential, expires_at: OffsetDateTime) -> Self {
        Self {
            credential,
            expires_at,
        }
    }

    /// 返回票据关联的认证凭据。
    #[must_use]
    pub const fn credential(&self) -> &RequestCredential {
        &self.credential
    }

    /// 返回票据到期时间。
    #[must_use]
    pub const fn expires_at(&self) -> OffsetDateTime {
        self.expires_at
    }
}
