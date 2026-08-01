use serde::Serialize;

use crate::IssuedSession;
use crate::SessionResponse;
use crate::UserResponse;

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
