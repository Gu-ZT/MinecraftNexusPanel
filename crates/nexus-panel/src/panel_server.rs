use std::net::SocketAddr;

use axum::Json;
use axum::Router;
use axum::extract::Request;
use axum::http::HeaderName;
use axum::http::HeaderValue;
use axum::middleware;
use axum::middleware::Next;
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

use crate::PanelError;

pub struct PanelServer {
    listen_address: SocketAddr,
    listener: TcpListener,
    store: SqliteStore,
}

impl PanelServer {
    pub async fn bind(config: &PanelConfig) -> Result<Self, PanelError> {
        let store = SqliteStore::open(config.data_directory())?;
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
            store,
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

        axum::serve(self.listener, router(self.store))
            .await
            .map_err(PanelError::Serve)
    }
}

fn router(store: SqliteStore) -> Router {
    Router::new()
        .route("/api/v1/health/live", get(health))
        .route("/api/v1/health/ready", get(health))
        .with_state(store)
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

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}
