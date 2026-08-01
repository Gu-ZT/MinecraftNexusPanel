use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::IssuedSession;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    id: String,
    access_token: Option<String>,
    access_expires_at: String,
    refresh_token: Option<String>,
    refresh_expires_at: Option<String>,
    csrf_token: Option<String>,
}

impl From<&IssuedSession> for SessionResponse {
    fn from(session: &IssuedSession) -> Self {
        Self {
            id: session.session_id().to_owned(),
            access_token: session.access_token().map(str::to_owned),
            access_expires_at: format_timestamp(session.access_expires_at()),
            refresh_token: session.refresh_token().map(str::to_owned),
            refresh_expires_at: session.refresh_expires_at().map(format_timestamp),
            csrf_token: session.csrf_token().map(str::to_owned),
        }
    }
}

fn format_timestamp(timestamp: i64) -> String {
    OffsetDateTime::from_unix_timestamp(timestamp)
        .ok()
        .and_then(|value| value.format(&Rfc3339).ok())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_owned())
}
