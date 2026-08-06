use serde::Deserialize;

use crate::permissions::is_assignable;

/// 管理员更新非管理员用户资料和权限的 Merge Patch 输入。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UserUpdate {
    display_name: Option<String>,
    permissions: Option<Vec<String>>,
}

impl UserUpdate {
    /// 返回规范化后的可选显示名称。
    #[must_use]
    pub(crate) fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref().map(str::trim)
    }

    /// 返回首个不满足安全或权限约束的字段名。
    #[must_use]
    pub(crate) fn invalid_field(&self) -> Option<&'static str> {
        if self.display_name.is_none() && self.permissions.is_none() {
            return Some("body");
        }
        if self.display_name().is_some_and(|display_name| {
            display_name.is_empty()
                || display_name.chars().count() > 128
                || display_name.contains('\0')
        }) {
            return Some("displayName");
        }
        if self.permissions.as_ref().is_some_and(|permissions| {
            permissions.len() > 16
                || permissions
                    .iter()
                    .any(|permission| !is_assignable(permission))
        }) {
            return Some("permissions");
        }

        None
    }

    /// 返回排序并去重后的可选权限集合。
    #[must_use]
    pub(crate) fn normalized_permissions(&self) -> Option<Vec<String>> {
        let mut permissions = self.permissions.clone()?;
        permissions.sort();
        permissions.dedup();
        Some(permissions)
    }
}
