//! Core 实例创建、配置更新、生命周期、日志和指标 HTTP 路由。
//!
//! 配置更新使用 ETag/修订号并限制在非运行状态，启停和命令操作都通过幂等键
//! 交给 Core 的进程管理器执行。

use std::collections::HashMap;

use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use nexus_domain::CoreId;
use nexus_domain::InstanceCreate;
use nexus_domain::InstanceId;
use nexus_domain::InstanceState;
use nexus_domain::InstanceUpdate;
use nexus_domain::RequestId;

use crate::InstanceCommandRequest;
use crate::InstanceKillRequest;
use crate::InstanceStopRequest;
use crate::PanelState;
use crate::auth_routes::error_response;
use crate::auth_routes::header_text;
use crate::core_routes::authorize;
use crate::core_routes::invalid_core_id_response;
use crate::core_routes::parse_core_id;
use crate::core_routes::registry_error_response;
use crate::core_routes::resource_response;

type InstanceListParameters = (Option<InstanceId>, Option<usize>, Option<InstanceState>);

pub(crate) fn instance_routes() -> Router<PanelState> {
    Router::new()
        .route(
            "/api/v1/cores/{core_id}/instances",
            get(list_instances).post(create_instance),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}",
            get(get_instance).patch(update_instance),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/actions/start",
            post(start_instance),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/actions/stop",
            post(stop_instance),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/actions/kill",
            post(kill_instance),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/commands",
            post(send_instance_command),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/logs",
            get(get_instance_logs),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/audit",
            get(get_instance_audit),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/metrics",
            get(get_instance_metrics),
        )
}

async fn list_instances(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(core_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let parameters = match list_parameters(&query) {
        Ok(parameters) => parameters,
        Err(()) => return validation_error(request_id),
    };

    match state
        .cores()
        .list_instances(core_id, parameters.0.as_ref(), parameters.1, parameters.2)
        .await
    {
        Ok(page) => Json(page).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn create_instance(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(core_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<InstanceCreate>, JsonRejection>,
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
        Ok(Json(request)) if request.validate().is_ok() => request,
        Ok(_) | Err(_) => return validation_error(request_id),
    };

    match state
        .cores()
        .create_instance(core_id, &request, idempotency_key)
        .await
    {
        Ok(instance) => resource_response(StatusCode::CREATED, instance),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn get_instance(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };

    match state.cores().get_instance(core_id, &instance_id).await {
        Ok(instance) => resource_response(StatusCode::OK, instance),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn update_instance(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    headers: HeaderMap,
    payload: Result<Json<InstanceUpdate>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(expected_revision) = expected_revision(&headers) else {
        return missing_if_match_response(request_id);
    };
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };
    let update = match payload {
        Ok(Json(update)) if update.validate().is_ok() => update,
        Ok(_) | Err(_) => return validation_error(request_id),
    };

    match state
        .cores()
        .update_instance(core_id, &instance_id, expected_revision, &update)
        .await
    {
        Ok(instance) => resource_response(StatusCode::OK, instance),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn start_instance(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(idempotency_key) = idempotency_key(&headers) else {
        return precondition_required_response(request_id);
    };
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };

    match state
        .cores()
        .start_instance(core_id, &instance_id, idempotency_key)
        .await
    {
        Ok(task) => (StatusCode::ACCEPTED, Json(task)).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn stop_instance(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    headers: HeaderMap,
    payload: Result<Option<Json<InstanceStopRequest>>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(idempotency_key) = idempotency_key(&headers) else {
        return precondition_required_response(request_id);
    };
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };
    let timeout_seconds = match payload {
        Ok(Some(Json(request))) => request.timeout_seconds(),
        Ok(None) => None,
        Err(_) => return validation_error(request_id),
    };

    match state
        .cores()
        .stop_instance(core_id, &instance_id, timeout_seconds, idempotency_key)
        .await
    {
        Ok(task) => (StatusCode::ACCEPTED, Json(task)).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn kill_instance(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    headers: HeaderMap,
    payload: Result<Json<InstanceKillRequest>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(idempotency_key) = idempotency_key(&headers) else {
        return precondition_required_response(request_id);
    };
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };
    if !matches!(payload, Ok(Json(request)) if request.confirmation() == "KILL") {
        return validation_error(request_id);
    }

    match state
        .cores()
        .kill_instance(core_id, &instance_id, idempotency_key)
        .await
    {
        Ok(task) => (StatusCode::ACCEPTED, Json(task)).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn send_instance_command(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    headers: HeaderMap,
    payload: Result<Json<InstanceCommandRequest>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(idempotency_key) = idempotency_key(&headers) else {
        return precondition_required_response(request_id);
    };
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };
    let request = match payload {
        Ok(Json(request)) if is_valid_command(request.command()) => request,
        Ok(_) | Err(_) => return validation_error(request_id),
    };

    match state
        .cores()
        .send_instance_command(core_id, &instance_id, request.command(), idempotency_key)
        .await
    {
        Ok(accepted) => (StatusCode::ACCEPTED, Json(accepted)).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn get_instance_logs(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };
    let Some(limit) = optional_limit(&query) else {
        return validation_error(request_id);
    };

    match state
        .cores()
        .get_instance_logs(
            core_id,
            &instance_id,
            query.get("after").map(String::as_str),
            query.get("before").map(String::as_str),
            limit,
        )
        .await
    {
        Ok(logs) => Json(logs).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn get_instance_metrics(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };

    match state
        .cores()
        .get_instance_metrics(
            core_id,
            &instance_id,
            query.get("range").map(String::as_str),
            query.get("resolution").map(String::as_str),
        )
        .await
    {
        Ok(metrics) => Json(metrics).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn get_instance_audit(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };
    let Some(limit) = optional_limit(&query) else {
        return validation_error(request_id);
    };

    match state
        .cores()
        .list_instance_audit(core_id, &instance_id, limit)
        .await
    {
        Ok(page) => Json(page).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

fn parse_ids(core_id: &str, instance_id: &str) -> Option<(CoreId, InstanceId)> {
    let core_id = parse_core_id(core_id)?;
    let instance_id = instance_id.parse::<InstanceId>().ok()?;

    Some((core_id, instance_id))
}

fn list_parameters(query: &HashMap<String, String>) -> Result<InstanceListParameters, ()> {
    let cursor = match query.get("cursor") {
        Some(value) => Some(value.parse::<InstanceId>().map_err(|_| ())?),
        None => None,
    };
    let limit = optional_limit(query).ok_or(())?;
    let state = match query.get("state") {
        Some(value) => Some(parse_instance_state(value).ok_or(())?),
        None => None,
    };

    Ok((cursor, limit, state))
}

fn optional_limit(query: &HashMap<String, String>) -> Option<Option<usize>> {
    match query.get("limit") {
        Some(value) => value
            .parse::<usize>()
            .ok()
            .filter(|value| (1..=200).contains(value))
            .map(Some),
        None => Some(None),
    }
}

fn parse_instance_state(value: &str) -> Option<InstanceState> {
    match value {
        "CREATED" => Some(InstanceState::Created),
        "STARTING" => Some(InstanceState::Starting),
        "RUNNING" => Some(InstanceState::Running),
        "STOPPING" => Some(InstanceState::Stopping),
        "STOPPED" => Some(InstanceState::Stopped),
        "FAILED" => Some(InstanceState::Failed),
        "UNKNOWN" => Some(InstanceState::Unknown),
        _ => None,
    }
}

fn idempotency_key(headers: &HeaderMap) -> Option<&str> {
    header_text(headers, "idempotency-key").filter(|value| value.parse::<RequestId>().is_ok())
}

fn expected_revision(headers: &HeaderMap) -> Option<u64> {
    header_text(headers, "if-match")?
        .strip_prefix('"')?
        .strip_suffix('"')?
        .parse()
        .ok()
        .filter(|revision| *revision > 0)
}

fn is_valid_command(command: &str) -> bool {
    !command.trim().is_empty() && command.len() <= 8192 && !command.contains('\0')
}

fn validation_error(request_id: RequestId) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "VALIDATION_FAILED",
        "Request validation failed",
        request_id,
    )
}

fn precondition_required_response(request_id: RequestId) -> Response {
    error_response(
        StatusCode::PRECONDITION_REQUIRED,
        "PRECONDITION_REQUIRED",
        "Idempotency-Key is required",
        request_id,
    )
}

fn missing_if_match_response(request_id: RequestId) -> Response {
    error_response(
        StatusCode::PRECONDITION_REQUIRED,
        "PRECONDITION_REQUIRED",
        "If-Match is required",
        request_id,
    )
}
