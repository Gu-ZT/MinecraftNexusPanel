/// 待写入 Panel 用户级 HTTP 审计事件。
///
/// 事件只保存请求追踪和授权结果等运维元数据，不保存请求体、Cookie、访问令牌
/// 或查询参数，避免把凭据和实例文件内容写入审计库。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewAuditEvent {
    /// 审计事件标识。
    pub id: String,
    /// 事件发生时间，使用 RFC 3339 文本保存。
    pub occurred_at: String,
    /// 通过有效会话解析出的用户标识；未认证请求为空。
    pub user_id: Option<String>,
    /// Panel 请求 ID。
    pub request_id: String,
    /// TCP 对端 IP；测试或非 TCP 调用没有该信息时为空。
    pub source_ip: Option<String>,
    /// HTTP 方法，例如 `GET` 或 `POST`。
    pub method: String,
    /// 不包含查询参数的请求路径。
    pub path: String,
    /// HTTP 响应状态码。
    pub status_code: u16,
    /// 该请求的授权结果。
    pub permission_result: String,
}
