use serde::Serialize;

use crate::IssuedSession;
use crate::SessionResponse;
use crate::UserResponse;

/// 登录成功响应，包含用户信息和已签发会话。
#[derive(Serialize)]
pub struct LoginResponse {
    user: UserResponse,
    session: SessionResponse,
}

impl From<&IssuedSession> for LoginResponse {
    fn from(session: &IssuedSession) -> Self {
        Self {
            user: UserResponse::from(session.user()),
            session: SessionResponse::from(session),
        }
    }
}
