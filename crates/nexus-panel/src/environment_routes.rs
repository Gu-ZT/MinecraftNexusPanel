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
use nexus_domain::RequestId;
use nexus_domain::RuntimeInstallManifest;
use nexus_domain::TaskId;
use serde::Deserialize;

use crate::PanelState;
use crate::auth_routes::error_response;
use crate::auth_routes::header_text;
use crate::core_routes::authorize;
use crate::core_routes::invalid_core_id_response;
use crate::core_routes::parse_core_id;
use crate::core_routes::registry_error_response;

pub(crate) fn environment_routes() -> Router<PanelState> {
    Router::new()
        .route(
            "/api/v1/cores/{core_id}/environments",
            get(list_managed_runtimes),
        )
        .route(
            "/api/v1/cores/{core_id}/runtime-installations",
            post(install_runtime),
        )
        .route(
            "/api/v1/cores/{core_id}/runtime-installations/{task_id}",
            get(get_runtime_installation),
        )
        .route(
            "/api/v1/cores/{core_id}/runtimes/{runtime_id}/actions/verify",
            post(verify_runtime),
        )
        .route(
            "/api/v1/cores/{core_id}/runtimes/{runtime_id}",
            delete(delete_runtime),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeInstallationRequest {
    manifest: RuntimeInstallManifest,
    #[serde(default)]
    set_as_default: bool,
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

async fn install_runtime(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(core_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<RuntimeInstallationRequest>, JsonRejection>,
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
        Ok(Json(request)) => request,
        Err(_) => return validation_error(request_id),
    };

    match state
        .cores()
        .install_runtime(
            core_id,
            &request.manifest,
            request.set_as_default,
            idempotency_key,
        )
        .await
    {
        Ok(result) => (StatusCode::ACCEPTED, Json(result)).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn get_runtime_installation(
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

    match state.cores().get_runtime_task(core_id, &task_id).await {
        Ok(result) => Json(result).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn verify_runtime(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, runtime_id)): Path<(String, String)>,
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
    if runtime_id.is_empty() {
        return validation_error(request_id);
    }

    match state
        .cores()
        .verify_runtime(core_id, &runtime_id, idempotency_key)
        .await
    {
        Ok(result) => (StatusCode::ACCEPTED, Json(result)).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn delete_runtime(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, runtime_id)): Path<(String, String)>,
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
    if runtime_id.is_empty() {
        return validation_error(request_id);
    }

    match state
        .cores()
        .delete_runtime(core_id, &runtime_id, idempotency_key)
        .await
    {
        Ok(result) => (StatusCode::ACCEPTED, Json(result)).into_response(),
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
