mod core_connection;
mod core_connection_error;
mod core_endpoint;
mod core_endpoint_error;
mod panel_error;
mod panel_server;

pub use core_connection::CoreConnection;
pub use core_connection_error::CoreConnectionError;
pub use core_endpoint::CoreEndpoint;
pub use core_endpoint_error::CoreEndpointError;
pub use panel_error::PanelError;
pub use panel_server::PanelServer;

use nexus_config::PanelConfig;

pub async fn run(config: &PanelConfig) -> Result<(), PanelError> {
    PanelServer::bind(config).await?.serve().await
}
