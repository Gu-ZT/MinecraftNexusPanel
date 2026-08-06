//! Panel 已执行的权限名与可授予范围。
//!
//! PLAN 中的完整权限目录是目标模型；这里只暴露已在服务端路由真正执行的权限，
//! 防止持久化尚不起作用的授权。

/// 查看 Panel 用户级审计记录。
pub(crate) const AUDIT_READ: &str = "audit.read";

/// 当前管理员会话向客户端公开的权限集合。
///
/// 管理员在服务端仍由 `is_admin` 获得全权；该列表只用于前端能力展示，必须
/// 随实际接入的权限检查增量扩展，不能提前宣称未执行的 RBAC 能力。
pub(crate) const ADMIN_PERMISSIONS: [&str; 9] = [
    "core.read",
    "core.manage",
    "instance.read",
    "instance.create",
    "instance.control",
    "instance.console",
    "user.read",
    "user.manage",
    AUDIT_READ,
];

/// 判断权限是否已支持授予非管理员。
///
/// 当前只有审计读取完成了服务端权限检查。其他 PLAN 权限在对应业务路由完成
/// RBAC 前不能进入可授予集合，否则客户端会看到无法兑现的授权。
pub(crate) fn is_assignable(permission: &str) -> bool {
    permission == AUDIT_READ
}
