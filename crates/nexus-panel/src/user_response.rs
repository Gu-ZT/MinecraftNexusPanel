use nexus_storage::StoredUser;
use serde::Serialize;

const ADMIN_PERMISSIONS: [&str; 6] = [
    "core.read",
    "core.manage",
    "instance.read",
    "instance.create",
    "instance.control",
    "instance.console",
];

/// 对外返回的用户身份、权限和资源范围。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    id: String,
    username: String,
    display_name: String,
    permissions: Vec<String>,
    resource_scopes: Vec<String>,
}

impl From<&StoredUser> for UserResponse {
    fn from(user: &StoredUser) -> Self {
        let permissions = if user.is_admin() {
            ADMIN_PERMISSIONS.iter().map(ToString::to_string).collect()
        } else {
            Vec::new()
        };

        Self {
            id: user.id().to_owned(),
            username: user.username().to_owned(),
            display_name: user.display_name().to_owned(),
            permissions,
            resource_scopes: Vec::new(),
        }
    }
}
