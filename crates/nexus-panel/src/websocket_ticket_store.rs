use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use time::Duration;
use time::OffsetDateTime;

use crate::AuthError;
use crate::RequestCredential;
use crate::WebSocketTicket;

const TICKET_BYTES: usize = 32;
const TICKET_LIFETIME_SECONDS: i64 = 30;

#[derive(Clone, Default)]
pub(crate) struct WebSocketTicketStore {
    tickets: Arc<Mutex<HashMap<String, WebSocketTicket>>>,
}

impl WebSocketTicketStore {
    pub fn issue(
        &self,
        credential: RequestCredential,
    ) -> Result<(String, OffsetDateTime), AuthError> {
        let mut bytes = [0_u8; TICKET_BYTES];
        getrandom::fill(&mut bytes)?;
        let ticket = URL_SAFE_NO_PAD.encode(bytes);
        let expires_at = OffsetDateTime::now_utc() + Duration::seconds(TICKET_LIFETIME_SECONDS);
        self.tickets
            .lock()
            .map_err(|_| AuthError::RateLimitLock)?
            .insert(ticket.clone(), WebSocketTicket::new(credential, expires_at));

        Ok((ticket, expires_at))
    }

    pub fn consume(&self, ticket: &str) -> Result<Option<WebSocketTicket>, AuthError> {
        let now = OffsetDateTime::now_utc();
        let mut tickets = self.tickets.lock().map_err(|_| AuthError::RateLimitLock)?;
        tickets.retain(|_, ticket| ticket.expires_at() > now);

        Ok(tickets
            .remove(ticket)
            .filter(|ticket| ticket.expires_at() > now))
    }
}
