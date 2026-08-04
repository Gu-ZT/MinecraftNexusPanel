use std::net::SocketAddr;

use axum::Json;
use axum::Router;
use axum::extract::Request;
use axum::extract::State;
use axum::http::HeaderName;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::middleware;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use nexus_config::PanelConfig;
use nexus_domain::RequestId;
use nexus_storage::SqliteStore;
use serde_json::Value;
use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::net::TcpListener;
use uuid::Uuid;

use crate::AuthService;
use crate::CoreRegistry;
use crate::PanelError;
use crate::PanelState;
use crate::SecretCipher;
use crate::VersionMetadataClient;
use crate::auth_routes::auth_routes;
use crate::bedrock_routes::bedrock_routes;
use crate::config_routes::config_routes;
use crate::core_routes::core_routes;
use crate::environment_routes::environment_routes;
use crate::extension_routes::extension_routes;
use crate::file_routes::file_routes;
use crate::install_template_routes::install_template_routes;
use crate::instance_routes::instance_routes;
use crate::provision_routes::provision_routes;
use crate::proxy_routes::proxy_routes;
use crate::websocket_routes::websocket_routes;

pub struct PanelServer {
    listen_address: SocketAddr,
    listener: TcpListener,
    state: PanelState,
}

impl PanelServer {
    pub async fn bind(config: &PanelConfig) -> Result<Self, PanelError> {
        let master_key = config
            .master_key()
            .ok_or(PanelError::MissingPanelMasterKey)?;
        let store = SqliteStore::open(config.data_directory())?;
        let panel_id = store.get_or_create_panel_id(&Uuid::now_v7().to_string())?;
        let auth = AuthService::new(store.clone());
        if let Some(initial_admin) = config.initial_admin() {
            if auth.initialize_admin(initial_admin)? {
                tracing::info!(
                    username = initial_admin.username(),
                    "Initial administrator created"
                );
            }
        }
        if !auth.has_users()? {
            tracing::warn!(
                "Panel has no users; configure MCNP_INITIAL_ADMIN_USERNAME and MCNP_INITIAL_ADMIN_PASSWORD"
            );
        }
        let cores = CoreRegistry::new(store, SecretCipher::new(master_key), panel_id)?;
        let version_metadata = VersionMetadataClient::new()?;
        if let Some(local_core) = config.local_core() {
            cores.ensure_local_core(local_core).await?;
        }
        let listener = TcpListener::bind(config.listen_address())
            .await
            .map_err(|source| PanelError::Bind {
                address: config.listen_address(),
                source,
            })?;
        let listen_address = listener.local_addr().map_err(|source| PanelError::Bind {
            address: config.listen_address(),
            source,
        })?;

        Ok(Self {
            listen_address,
            listener,
            state: PanelState::new(auth, cores, version_metadata),
        })
    }

    #[must_use]
    pub const fn listen_address(&self) -> SocketAddr {
        self.listen_address
    }

    pub async fn serve(self) -> Result<(), PanelError> {
        tracing::info!(
            listen_address = %self.listen_address,
            "Panel HTTP listener is ready"
        );

        axum::serve(
            self.listener,
            router(self.state).into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .map_err(PanelError::Serve)
    }
}

fn router(state: PanelState) -> Router {
    Router::new()
        .route("/api/v1/health/live", get(health))
        .route("/api/v1/health/ready", get(readiness))
        .merge(auth_routes())
        .merge(core_routes())
        .merge(bedrock_routes())
        .merge(config_routes())
        .merge(environment_routes())
        .merge(extension_routes())
        .merge(file_routes())
        .merge(instance_routes())
        .merge(proxy_routes())
        .merge(provision_routes())
        .merge(install_template_routes())
        .merge(websocket_routes())
        .with_state(state)
        .layer(middleware::from_fn(assign_request_id))
}

async fn assign_request_id(mut request: Request, next: Next) -> Response {
    let request_id: RequestId = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .unwrap_or_default();
    request.extensions_mut().insert(request_id);

    let mut response = next.run(request).await;
    if let Ok(header_value) = HeaderValue::from_str(&request_id.to_string()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-request-id"), header_value);
    }

    response
}

async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "time": current_timestamp(),
    }))
}

async fn readiness(State(state): State<PanelState>) -> Response {
    match state.auth().has_users() {
        Ok(true) => health().await.into_response(),
        Ok(false) | Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "time": current_timestamp(),
            })),
        )
            .into_response(),
    }
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}
