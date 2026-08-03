use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::Path;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::delete;
use axum::routing::get;
use nexus_domain::InstanceId;
use nexus_domain::ProxySubserver;
use nexus_domain::RequestId;

use crate::PanelState;
use crate::auth_routes::error_response;
use crate::auth_routes::header_text;
use crate::core_routes::authorize;
use crate::core_routes::invalid_core_id_response;
use crate::core_routes::parse_core_id;
use crate::core_routes::registry_error_response;

pub(crate) fn proxy_routes() -> Router<PanelState> {
    Router::new()
        .route(
            "/api/v1/cores/{core_id}/instances/{proxy_instance_id}/proxy-subservers",
            get(list_proxy_subservers).post(upsert_proxy_subserver),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{proxy_instance_id}/proxy-subservers/{subserver_id}",
            delete(delete_proxy_subserver),
        )
}

async fn list_proxy_subservers(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, proxy_instance_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(proxy_instance_id) = parse_instance_id(&proxy_instance_id) else {
        return validation_error(request_id);
    };

    match state
        .cores()
        .list_proxy_subservers(core_id, &proxy_instance_id)
        .await
    {
        Ok(items) => Json(items).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn upsert_proxy_subserver(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, proxy_instance_id)): Path<(String, String)>,
    headers: HeaderMap,
    payload: Result<Json<ProxySubserver>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(idempotency_key) = idempotency_key(&headers) else {
        return precondition_required_response(request_id);
    };
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(proxy_instance_id) = parse_instance_id(&proxy_instance_id) else {
        return validation_error(request_id);
    };
    let request = match payload {
        Ok(Json(request)) if request.validate().is_ok() => request,
        Ok(_) | Err(_) => return validation_error(request_id),
    };

    match state
        .cores()
        .upsert_proxy_subserver(core_id, &proxy_instance_id, &request, idempotency_key)
        .await
    {
        Ok(item) => Json(item).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn delete_proxy_subserver(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, proxy_instance_id, subserver_id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(idempotency_key) = idempotency_key(&headers) else {
        return precondition_required_response(request_id);
    };
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(proxy_instance_id) = parse_instance_id(&proxy_instance_id) else {
        return validation_error(request_id);
    };
    if subserver_id.is_empty() {
        return validation_error(request_id);
    }

    match state
        .cores()
        .delete_proxy_subserver(core_id, &proxy_instance_id, &subserver_id, idempotency_key)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

fn parse_instance_id(value: &str) -> Option<InstanceId> {
    value.parse().ok()
}

fn idempotency_key(headers: &HeaderMap) -> Option<&str> {
    header_text(headers, "idempotency-key").filter(|value| value.parse::<RequestId>().is_ok())
}

fn precondition_required_response(request_id: RequestId) -> Response {
    error_response(
        StatusCode::PRECONDITION_REQUIRED,
        "PRECONDITION_REQUIRED",
        "Idempotency-Key is required",
        request_id,
    )
}

fn validation_error(request_id: RequestId) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "VALIDATION_FAILED",
        "Request validation failed",
        request_id,
    )
}
