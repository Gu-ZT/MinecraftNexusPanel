use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::Path;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use nexus_domain::RequestId;

use crate::PanelState;
use crate::core_routes::authorize;
use crate::core_routes::invalid_core_id_response;
use crate::core_routes::parse_core_id;
use crate::core_routes::registry_error_response;

pub(crate) fn environment_routes() -> Router<PanelState> {
    Router::new().route(
        "/api/v1/cores/{core_id}/environments",
        get(list_managed_runtimes),
    )
}

async fn list_managed_runtimes(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(core_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };

    match state.cores().list_managed_runtimes(core_id).await {
        Ok(runtimes) => Json(runtimes).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}
