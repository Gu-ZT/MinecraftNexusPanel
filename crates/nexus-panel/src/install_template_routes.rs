use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use nexus_domain::RequestId;
use serde_json::json;

use crate::PanelState;
use crate::core_routes::authorize;
use crate::install_template_catalog::install_templates;

pub(crate) fn install_template_routes() -> Router<PanelState> {
    Router::new().route("/api/v1/install-templates", get(list_install_templates))
}

async fn list_install_templates(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }

    Json(json!({ "items": install_templates() })).into_response()
}
