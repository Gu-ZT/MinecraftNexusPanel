use crate::AuthService;
use crate::CoreRegistry;
use crate::VersionMetadataClient;
use crate::WebSocketTicketStore;
use crate::extension_source_client::ExtensionSourceClient;
use crate::extension_task_store::ExtensionTaskStore;

#[derive(Clone)]
pub struct PanelState {
    auth: AuthService,
    cores: CoreRegistry,
    extension_sources: ExtensionSourceClient,
    extension_tasks: ExtensionTaskStore,
    version_metadata: VersionMetadataClient,
    websocket_tickets: WebSocketTicketStore,
}

impl PanelState {
    #[must_use]
    pub fn new(
        auth: AuthService,
        cores: CoreRegistry,
        extension_sources: ExtensionSourceClient,
        version_metadata: VersionMetadataClient,
    ) -> Self {
        Self {
            auth,
            cores,
            extension_sources,
            extension_tasks: ExtensionTaskStore::default(),
            version_metadata,
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
    pub const fn extension_sources(&self) -> &ExtensionSourceClient {
        &self.extension_sources
    }

    #[must_use]
    pub const fn extension_tasks(&self) -> &ExtensionTaskStore {
        &self.extension_tasks
    }

    #[must_use]
    pub const fn version_metadata(&self) -> &VersionMetadataClient {
        &self.version_metadata
    }

    #[must_use]
    pub const fn websocket_tickets(&self) -> &WebSocketTicketStore {
        &self.websocket_tickets
    }
}
