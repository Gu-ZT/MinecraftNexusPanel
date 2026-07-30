mod core_connection;
mod core_connection_error;
mod panel_error;
mod panel_server;

pub use core_connection::CoreConnection;
pub use core_connection_error::CoreConnectionError;
pub use panel_error::PanelError;
pub use panel_server::PanelServer;

use nexus_config::PanelConfig;

pub async fn run(config: &PanelConfig) -> Result<(), PanelError> {
    PanelServer::bind(config).await?.serve().await
}
