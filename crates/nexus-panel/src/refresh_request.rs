use serde::Deserialize;

/// 刷新会话请求输入。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    refresh_token: String,
}

impl RefreshRequest {
    /// 返回刷新凭据；调用方不得记录该值。
    #[must_use]
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }
}
