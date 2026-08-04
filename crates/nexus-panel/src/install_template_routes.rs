//! 安装模板和版本元数据 HTTP 路由。

use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::Path;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use nexus_domain::RequestId;
use serde_json::json;

use crate::PanelState;
use crate::VersionMetadataError;
use crate::auth_routes::error_response;
use crate::core_routes::authorize;
use crate::install_template_catalog::install_template;
use crate::install_template_catalog::install_templates;

pub(crate) fn install_template_routes() -> Router<PanelState> {
    Router::new()
        .route("/api/v1/install-templates", get(list_install_templates))
        .route(
            "/api/v1/install-templates/{template_id}/versions",
            get(list_install_template_versions),
        )
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

async fn list_install_template_versions(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(template_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some(template) = install_template(&template_id) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Install template does not exist",
            request_id,
        );
    };

    match state.version_metadata().list_versions(&template).await {
        Ok(versions) => Json(json!({ "items": versions })).into_response(),
        Err(error) => version_metadata_error_response(error, request_id),
    }
}

fn version_metadata_error_response(error: VersionMetadataError, request_id: RequestId) -> Response {
    tracing::warn!(%error, %request_id, "Version metadata lookup failed");

    error_response(
        StatusCode::BAD_GATEWAY,
        "VERSION_METADATA_UNAVAILABLE",
        "Version metadata is unavailable",
        request_id,
    )
}
