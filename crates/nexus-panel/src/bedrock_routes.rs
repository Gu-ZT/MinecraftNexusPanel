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
use axum::routing::post;
use nexus_domain::InstanceId;
use nexus_domain::RequestId;

use crate::PanelState;
use crate::auth_routes::error_response;
use crate::core_routes::authorize;
use crate::core_routes::invalid_core_id_response;
use crate::core_routes::parse_core_id;
use crate::core_routes::registry_error_response;

pub(crate) fn bedrock_routes() -> Router<PanelState> {
    Router::new()
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/bedrock-profile",
            get(get_bedrock_profile),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/bedrock-profile/actions/check-port",
            post(check_bedrock_port),
        )
}

async fn get_bedrock_profile(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(instance_id) = instance_id.parse::<InstanceId>().ok() else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
            "Instance ID is invalid",
            request_id,
        );
    };

    match state
        .cores()
        .get_bedrock_profile(core_id, &instance_id)
        .await
    {
        Ok(profile) => Json(profile).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn check_bedrock_port(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(instance_id) = instance_id.parse::<InstanceId>().ok() else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
            "Instance ID is invalid",
            request_id,
        );
    };

    match state
        .cores()
        .check_bedrock_port(core_id, &instance_id)
        .await
    {
        Ok(check) => Json(check).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}
