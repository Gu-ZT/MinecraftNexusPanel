/// 从 Panel SQLite 读取的一条用户级 HTTP 审计事件。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredAuditEvent {
    pub(crate) id: String,
    pub(crate) occurred_at: String,
    pub(crate) user_id: Option<String>,
    pub(crate) request_id: String,
    pub(crate) source_ip: Option<String>,
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) status_code: u16,
    pub(crate) permission_result: String,
}

impl StoredAuditEvent {
    /// 返回审计事件标识。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回事件发生时间。
    #[must_use]
    pub fn occurred_at(&self) -> &str {
        &self.occurred_at
    }

    /// 返回关联用户标识。
    #[must_use]
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// 返回 Panel 请求标识。
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// 返回 TCP 对端 IP。
    #[must_use]
    pub fn source_ip(&self) -> Option<&str> {
        self.source_ip.as_deref()
    }

    /// 返回 HTTP 方法。
    #[must_use]
    pub fn method(&self) -> &str {
        &self.method
    }

    /// 返回不含查询参数的请求路径。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 返回 HTTP 响应状态码。
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status_code
    }

    /// 返回授权结果。
    #[must_use]
    pub fn permission_result(&self) -> &str {
        &self.permission_result
    }
}
