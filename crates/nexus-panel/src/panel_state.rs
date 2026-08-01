use crate::AuthService;
use crate::CoreRegistry;

#[derive(Clone)]
pub struct PanelState {
    auth: AuthService,
    cores: CoreRegistry,
}

impl PanelState {
    #[must_use]
    pub const fn new(auth: AuthService, cores: CoreRegistry) -> Self {
        Self { auth, cores }
    }

    #[must_use]
    pub const fn auth(&self) -> &AuthService {
        &self.auth
    }

    #[must_use]
    pub const fn cores(&self) -> &CoreRegistry {
        &self.cores
    }
}
