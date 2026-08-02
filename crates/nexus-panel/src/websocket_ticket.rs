use time::OffsetDateTime;

use crate::RequestCredential;

#[derive(Clone)]
pub(crate) struct WebSocketTicket {
    credential: RequestCredential,
    expires_at: OffsetDateTime,
}

impl WebSocketTicket {
    #[must_use]
    pub const fn new(credential: RequestCredential, expires_at: OffsetDateTime) -> Self {
        Self {
            credential,
            expires_at,
        }
    }

    #[must_use]
    pub const fn credential(&self) -> &RequestCredential {
        &self.credential
    }

    #[must_use]
    pub const fn expires_at(&self) -> OffsetDateTime {
        self.expires_at
    }
}
