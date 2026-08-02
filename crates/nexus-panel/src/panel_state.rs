use crate::AuthService;
use crate::CoreRegistry;
use crate::WebSocketTicketStore;

#[derive(Clone)]
pub struct PanelState {
    auth: AuthService,
    cores: CoreRegistry,
    websocket_tickets: WebSocketTicketStore,
}

impl PanelState {
    #[must_use]
    pub fn new(auth: AuthService, cores: CoreRegistry) -> Self {
        Self {
            auth,
            cores,
            websocket_tickets: WebSocketTicketStore::default(),
        }
    }

    #[must_use]
    pub const fn auth(&self) -> &AuthService {
        &self.auth
    }

    #[must_use]
    pub const fn cores(&self) -> &CoreRegistry {
        &self.cores
    }

    #[must_use]
    pub const fn websocket_tickets(&self) -> &WebSocketTicketStore {
        &self.websocket_tickets
    }
}
