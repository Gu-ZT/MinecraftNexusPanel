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
    permissions: Vec<String>,
}

impl StoredUser {
    pub(crate) fn new(
        id: String,
        username: String,
        display_name: String,
        password_hash: String,
        is_admin: bool,
        permissions: Vec<String>,
    ) -> Self {
        Self {
            id,
            username,
            display_name,
            password_hash,
            is_admin,
            permissions,
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

    /// 返回显式授予非管理员的权限名。
    ///
    /// 管理员的隐式全权不写入该数组，调用方应通过 [`Self::has_permission`]
    /// 统一判断，避免遗漏管理员旁路。
    #[must_use]
    pub fn permissions(&self) -> &[String] {
        &self.permissions
    }

    /// 判断用户是否具有指定权限；管理员始终通过权限检查。
    #[must_use]
    pub fn has_permission(&self, permission: &str) -> bool {
        self.is_admin || self.permissions.iter().any(|value| value == permission)
    }
}
