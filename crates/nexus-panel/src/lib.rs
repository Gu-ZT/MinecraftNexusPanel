//! MCNP Panel HTTP API、Core 注册和身份认证服务。
//!
//! Panel 负责面向用户的鉴权、持久化和 HTTP 路由，并通过 `CoreConnection` 调用
//! Core；宿主机文件、进程和服务端实际操作仍由 Core 执行。

mod audit_routes;
mod auth_error;
mod auth_routes;
mod auth_service;
mod bedrock_extension_validator;
mod bedrock_routes;
mod client_type;
mod config_routes;
mod core_connection;
mod core_connection_error;
mod core_create;
mod core_endpoint;
mod core_endpoint_error;
mod core_registry;
mod core_registry_error;
mod core_routes;
mod core_runtime;
mod core_status;
mod cpu_reservation_request;
mod environment_routes;
mod extension_routes;
mod extension_source_client;
mod extension_source_error;
mod extension_task_store;
mod file_routes;
mod install_template_catalog;
mod install_template_routes;
mod instance_command_request;
mod instance_kill_request;
mod instance_reset_request;
mod instance_routes;
mod instance_stop_request;
mod issued_session;
mod login_request;
mod login_response;
mod managed_core;
mod panel_error;
mod panel_server;
mod panel_state;
mod permissions;
mod provision_execute_request;
mod provision_routes;
mod proxy_orchestration_request;
mod proxy_routes;
mod refresh_request;
mod secret_cipher;
mod secret_cipher_error;
mod session_response;
mod user_create;
mod user_response;
mod user_routes;
mod user_update;
mod version_metadata_client;
mod version_metadata_error;
mod websocket_routes;
mod websocket_ticket;
mod websocket_ticket_store;

pub use auth_error::AuthError;
pub(crate) use auth_routes::RequestCredential;
pub(crate) use auth_service::AuthService;
pub(crate) use client_type::ClientType;
pub use core_connection::CoreConnection;
pub use core_connection_error::CoreConnectionError;
pub(crate) use core_create::CoreCreate;
pub use core_endpoint::CoreEndpoint;
pub use core_endpoint_error::CoreEndpointError;
pub(crate) use core_registry::CoreRegistry;
pub(crate) use core_registry_error::CoreRegistryError;
pub(crate) use core_runtime::CoreRuntime;
pub(crate) use core_status::CoreStatus;
pub(crate) use cpu_reservation_request::CpuReservationRequest;
pub(crate) use instance_command_request::InstanceCommandRequest;
pub(crate) use instance_kill_request::InstanceKillRequest;
pub(crate) use instance_reset_request::InstanceResetRequest;
pub(crate) use instance_stop_request::InstanceStopRequest;
pub(crate) use issued_session::IssuedSession;
pub(crate) use login_request::LoginRequest;
pub(crate) use login_response::LoginResponse;
pub(crate) use managed_core::ManagedCore;
pub use panel_error::PanelError;
pub use panel_server::PanelServer;
pub(crate) use panel_state::PanelState;
pub(crate) use provision_execute_request::ProvisionExecuteRequest;
pub(crate) use proxy_orchestration_request::ProxyOrchestrationRequest;
pub(crate) use refresh_request::RefreshRequest;
pub(crate) use secret_cipher::SecretCipher;
pub(crate) use secret_cipher_error::SecretCipherError;
pub(crate) use session_response::SessionResponse;
pub(crate) use user_create::UserCreate;
pub(crate) use user_response::UserResponse;
pub(crate) use user_update::UserUpdate;
pub(crate) use version_metadata_client::VersionMetadataClient;
pub use version_metadata_error::VersionMetadataError;
pub(crate) use websocket_ticket::WebSocketTicket;
pub(crate) use websocket_ticket_store::WebSocketTicketStore;

use nexus_config::PanelConfig;

/// 按配置绑定并运行 Panel HTTP 服务。
pub async fn run(config: &PanelConfig) -> Result<(), PanelError> {
    PanelServer::bind(config).await?.serve().await
}
