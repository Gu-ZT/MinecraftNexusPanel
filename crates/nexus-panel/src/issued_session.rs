use nexus_storage::StoredUser;

/// 登录或刷新后生成的敏感会话凭据集合。
///
/// 原生客户端使用访问/刷新令牌，浏览器客户端使用 Cookie/CSRF 令牌；调用方
/// 只应在构造 HTTP 响应时读取一次，不能写入日志。
pub struct IssuedSession {
    pub(crate) session_id: String,
    pub(crate) user: StoredUser,
    pub(crate) access_token: Option<String>,
    pub(crate) access_expires_at: i64,
    pub(crate) refresh_token: Option<String>,
    pub(crate) refresh_expires_at: Option<i64>,
    pub(crate) csrf_token: Option<String>,
    pub(crate) browser_cookie: Option<String>,
}

impl IssuedSession {
    /// 返回会话标识。
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// 返回关联用户。
    #[must_use]
    pub const fn user(&self) -> &StoredUser {
        &self.user
    }

    /// 返回原生访问令牌。
    #[must_use]
    pub fn access_token(&self) -> Option<&str> {
        self.access_token.as_deref()
    }

    /// 返回访问令牌到期 Unix 时间戳。
    #[must_use]
    pub const fn access_expires_at(&self) -> i64 {
        self.access_expires_at
    }

    /// 返回原生刷新令牌。
    #[must_use]
    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    /// 返回刷新令牌到期 Unix 时间戳。
    #[must_use]
    pub const fn refresh_expires_at(&self) -> Option<i64> {
        self.refresh_expires_at
    }

    /// 返回浏览器 CSRF 令牌。
    #[must_use]
    pub fn csrf_token(&self) -> Option<&str> {
        self.csrf_token.as_deref()
    }

    /// 返回浏览器会话 Cookie 值。
    #[must_use]
    pub fn browser_cookie(&self) -> Option<&str> {
        self.browser_cookie.as_deref()
    }
}
