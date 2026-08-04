use serde::Deserialize;

/// 登录客户端类型，决定令牌和 CSRF 策略。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClientType {
    /// 浏览器客户端，使用 Cookie 和 CSRF 令牌。
    Browser,
    /// 原生客户端，使用访问令牌和刷新令牌。
    Native,
}

impl ClientType {
    /// 返回存储层使用的稳定客户端类型字符串。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Browser => "BROWSER",
            Self::Native => "NATIVE",
        }
    }
}
