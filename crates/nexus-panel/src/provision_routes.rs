//! 一键搭建计划解析、执行和任务查询 HTTP 路由。
//!
//! 执行请求必须携带解析阶段返回的计划哈希；计划与执行均要求管理员和幂等键。

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
use axum::routing::get;
use axum::routing::post;
use nexus_domain::ProvisionPlan;
use nexus_domain::RequestId;
use nexus_domain::TaskId;

use crate::PanelState;
use crate::ProvisionExecuteRequest;
use crate::auth_routes::error_response;
use crate::auth_routes::header_text;
use crate::core_routes::authorize;
use crate::core_routes::invalid_core_id_response;
use crate::core_routes::parse_core_id;
use crate::core_routes::registry_error_response;

pub(crate) fn provision_routes() -> Router<PanelState> {
    Router::new()
        .route(
            "/api/v1/cores/{core_id}/provision-plans:resolve",
            post(resolve_provision),
        )
        .route(
            "/api/v1/cores/{core_id}/instance-provisions",
            post(execute_provision),
        )
        .route(
            "/api/v1/cores/{core_id}/instance-provisions/{task_id}",
            get(get_provision_task),
        )
}

async fn resolve_provision(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(core_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<ProvisionPlan>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let plan = match payload {
        Ok(Json(plan)) => plan,
        Err(_) => return validation_error(request_id),
    };

    match state.cores().resolve_provision(core_id, &plan).await {
        Ok(result) => Json(result).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn execute_provision(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(core_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<ProvisionExecuteRequest>, JsonRejection>,
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
    let request = match payload {
        Ok(Json(request)) if !request.plan_hash().is_empty() => request,
        Ok(_) | Err(_) => return validation_error(request_id),
    };

    match state
        .cores()
        .execute_provision(
            core_id,
            request.resolved_plan(),
            request.plan_hash(),
            idempotency_key,
        )
        .await
    {
        Ok(result) => (StatusCode::ACCEPTED, Json(result)).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn get_provision_task(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, task_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Ok(task_id) = task_id.parse::<TaskId>() else {
        return validation_error(request_id);
    };

    match state.cores().get_provision_task(core_id, &task_id).await {
        Ok(result) => Json(result).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
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
