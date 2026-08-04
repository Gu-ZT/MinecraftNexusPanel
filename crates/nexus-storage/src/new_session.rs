/// 待写入登录会话记录。
///
/// 令牌字段只接受哈希值；存储层不会接触或持久化访问令牌、刷新令牌原文。
#[derive(Debug)]
pub struct NewSession {
    /// 会话标识。
    pub id: String,
    /// 所属用户标识。
    pub user_id: String,
    /// 客户端类型，如浏览器或原生客户端。
    pub client_type: String,
    /// 访问令牌哈希。
    pub access_token_hash: Option<String>,
    /// 访问令牌到期时间戳。
    pub access_expires_at: Option<i64>,
    /// 刷新令牌哈希。
    pub refresh_token_hash: String,
    /// 刷新令牌到期时间戳。
    pub refresh_expires_at: i64,
    /// CSRF 令牌哈希；原生客户端可为空。
    pub csrf_token_hash: Option<String>,
    /// 会话创建时间戳。
    pub created_at: i64,
}
