use nexus_storage::StoredUser;

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
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub const fn user(&self) -> &StoredUser {
        &self.user
    }

    #[must_use]
    pub fn access_token(&self) -> Option<&str> {
        self.access_token.as_deref()
    }

    #[must_use]
    pub const fn access_expires_at(&self) -> i64 {
        self.access_expires_at
    }

    #[must_use]
    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    #[must_use]
    pub const fn refresh_expires_at(&self) -> Option<i64> {
        self.refresh_expires_at
    }

    #[must_use]
    pub fn csrf_token(&self) -> Option<&str> {
        self.csrf_token.as_deref()
    }

    #[must_use]
    pub fn browser_cookie(&self) -> Option<&str> {
        self.browser_cookie.as_deref()
    }
}
