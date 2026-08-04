//! 代理后端关系、健康检查和代理编排 HTTP 路由。
//!
//! 后端数量和一对多/一对一拓扑由 Core 领域校验；编排请求明确控制是否连带
//! 启停后端以及停止超时。

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
use axum::routing::post;
use nexus_domain::InstanceId;
use nexus_domain::ProxySubserver;
use nexus_domain::RequestId;

use crate::PanelState;
use crate::ProxyOrchestrationRequest;
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
        .route(
            "/api/v1/cores/{core_id}/instances/{proxy_instance_id}/proxy-subservers/{subserver_id}/actions/check",
            post(check_proxy_subserver),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{proxy_instance_id}/proxy-subservers/actions/start",
            post(start_proxy),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{proxy_instance_id}/proxy-subservers/actions/stop",
            post(stop_proxy),
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

async fn check_proxy_subserver(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, proxy_instance_id, subserver_id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
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
        .check_proxy_subserver(core_id, &proxy_instance_id, &subserver_id)
        .await
    {
        Ok(health) => Json(health).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn start_proxy(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, proxy_instance_id)): Path<(String, String)>,
    headers: HeaderMap,
    payload: Result<Option<Json<ProxyOrchestrationRequest>>, JsonRejection>,
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
        Ok(Some(Json(request))) if request.validate().is_ok() => request,
        Ok(None) => ProxyOrchestrationRequest::default(),
        Ok(Some(_)) | Err(_) => return validation_error(request_id),
    };

    match state
        .cores()
        .start_proxy(
            core_id,
            &proxy_instance_id,
            request.include_backends(),
            idempotency_key,
        )
        .await
    {
        Ok(result) => Json(result).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn stop_proxy(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, proxy_instance_id)): Path<(String, String)>,
    headers: HeaderMap,
    payload: Result<Option<Json<ProxyOrchestrationRequest>>, JsonRejection>,
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
        Ok(Some(Json(request))) if request.validate().is_ok() => request,
        Ok(None) => ProxyOrchestrationRequest::default(),
        Ok(Some(_)) | Err(_) => return validation_error(request_id),
    };

    match state
        .cores()
        .stop_proxy(
            core_id,
            &proxy_instance_id,
            request.include_backends(),
            request.timeout_seconds(),
            idempotency_key,
        )
        .await
    {
        Ok(result) => Json(result).into_response(),
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
