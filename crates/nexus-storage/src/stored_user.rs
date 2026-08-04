/// 从数据库读取的用户身份摘要。
///
/// `password_hash` 是单向哈希，不是用户密码原文；调用方不得把它作为登录凭据返回客户端。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredUser {
    id: String,
    username: String,
    display_name: String,
    password_hash: String,
    is_admin: bool,
}

impl StoredUser {
    pub(crate) fn new(
        id: String,
        username: String,
        display_name: String,
        password_hash: String,
        is_admin: bool,
    ) -> Self {
        Self {
            id,
            username,
            display_name,
            password_hash,
            is_admin,
        }
    }

    /// 返回用户标识。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回登录用户名。
    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    /// 返回显示名称。
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// 返回密码哈希供鉴权服务校验。
    #[must_use]
    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }

    /// 返回管理员标记。
    #[must_use]
    pub const fn is_admin(&self) -> bool {
        self.is_admin
    }
}
