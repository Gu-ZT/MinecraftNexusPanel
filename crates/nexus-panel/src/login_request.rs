use serde::Deserialize;
use serde_json::Value;

use crate::ClientType;

/// 登录请求输入。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    username: String,
    password: String,
    client_type: ClientType,
    #[serde(rename = "device")]
    _device: Option<Value>,
    #[serde(rename = "mfaCode")]
    _mfa_code: Option<String>,
}

impl LoginRequest {
    /// 返回登录用户名。
    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    /// 返回密码；调用方不得记录该值。
    #[must_use]
    pub fn password(&self) -> &str {
        &self.password
    }

    /// 返回客户端类型。
    #[must_use]
    pub const fn client_type(&self) -> ClientType {
        self.client_type
    }

    /// 判断用户名和密码是否满足基础输入长度约束。
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.username.is_empty()
            && self.username.chars().count() <= 64
            && !self.password.is_empty()
            && self.password.len() <= 1024
    }
}
