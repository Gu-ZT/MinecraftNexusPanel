use crate::StoredUser;

/// 从数据库读取的有效会话摘要。
///
/// 查询结果只包含用于鉴权和响应的用户信息，不暴露访问令牌或刷新令牌原文。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredSession {
    id: String,
    client_type: String,
    csrf_token_hash: Option<String>,
    user: StoredUser,
}

impl StoredSession {
    pub(crate) fn new(
        id: String,
        client_type: String,
        csrf_token_hash: Option<String>,
        user: StoredUser,
    ) -> Self {
        Self {
            id,
            client_type,
            csrf_token_hash,
            user,
        }
    }

    /// 返回会话标识。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回客户端类型。
    #[must_use]
    pub fn client_type(&self) -> &str {
        &self.client_type
    }

    /// 返回 CSRF 令牌哈希。
    #[must_use]
    pub fn csrf_token_hash(&self) -> Option<&str> {
        self.csrf_token_hash.as_deref()
    }

    /// 返回关联用户。
    #[must_use]
    pub const fn user(&self) -> &StoredUser {
        &self.user
    }
}
