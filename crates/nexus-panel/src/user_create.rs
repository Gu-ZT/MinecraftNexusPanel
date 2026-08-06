use serde::Deserialize;

use crate::permissions::is_assignable;

/// 管理员创建非管理员用户的请求输入。
///
/// 密码只在 Argon2 哈希前短暂保存在内存中，不进入响应、审计事件或日志。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserCreate {
    username: String,
    display_name: String,
    password: String,
    #[serde(default)]
    permissions: Vec<String>,
}

impl UserCreate {
    /// 返回规范化用户名。
    #[must_use]
    pub(crate) fn username(&self) -> &str {
        self.username.trim()
    }

    /// 返回规范化显示名称。
    #[must_use]
    pub(crate) fn display_name(&self) -> &str {
        self.display_name.trim()
    }

    /// 返回待哈希密码；调用方不得记录该值。
    #[must_use]
    pub(crate) fn password(&self) -> &str {
        &self.password
    }

    /// 返回首个不满足安全或长度约束的字段名。
    #[must_use]
    pub(crate) fn invalid_field(&self) -> Option<&'static str> {
        if self.username().is_empty()
            || self.username().chars().count() > 64
            || self.username.contains('\0')
        {
            return Some("username");
        }
        if self.display_name().is_empty()
            || self.display_name().chars().count() > 128
            || self.display_name.contains('\0')
        {
            return Some("displayName");
        }
        if !(12..=1024).contains(&self.password.len()) {
            return Some("password");
        }
        if self.permissions.len() > 16
            || self
                .permissions
                .iter()
                .any(|permission| !is_assignable(permission))
        {
            return Some("permissions");
        }

        None
    }

    /// 返回排序并去重后的可持久化权限集合。
    #[must_use]
    pub(crate) fn normalized_permissions(&self) -> Vec<String> {
        let mut permissions = self.permissions.clone();
        permissions.sort();
        permissions.dedup();
        permissions
    }
}
