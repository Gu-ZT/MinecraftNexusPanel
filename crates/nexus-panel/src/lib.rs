mod auth_error;
mod auth_routes;
mod auth_service;
mod client_type;
mod core_connection;
mod core_connection_error;
mod core_endpoint;
mod core_endpoint_error;
mod issued_session;
mod login_request;
mod login_response;
mod panel_error;
mod panel_server;
mod refresh_request;
mod session_response;
mod user_response;

pub use auth_error::AuthError;
pub(crate) use auth_service::AuthService;
pub(crate) use client_type::ClientType;
pub use core_connection::CoreConnection;
pub use core_connection_error::CoreConnectionError;
pub use core_endpoint::CoreEndpoint;
pub use core_endpoint_error::CoreEndpointError;
pub(crate) use issued_session::IssuedSession;
pub(crate) use login_request::LoginRequest;
pub(crate) use login_response::LoginResponse;
pub use panel_error::PanelError;
pub use panel_server::PanelServer;
pub(crate) use refresh_request::RefreshRequest;
pub(crate) use session_response::SessionResponse;
pub(crate) use user_response::UserResponse;

use nexus_config::PanelConfig;

pub async fn run(config: &PanelConfig) -> Result<(), PanelError> {
    PanelServer::bind(config).await?.serve().await
}
