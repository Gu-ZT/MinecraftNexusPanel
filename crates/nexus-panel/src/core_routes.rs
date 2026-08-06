//! Core 注册、连接测试和重连 HTTP 路由。
//!
//! 路由将领域错误映射为稳定 HTTP 错误码，并为带修订号的资源返回 ETag；所有
//! 管理写操作都要求管理员认证和浏览器 CSRF 校验。

use std::str::FromStr;

use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::Path;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::http::header::ETAG;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::delete;
use axum::routing::get;
use axum::routing::post;
use nexus_domain::CoreId;
use nexus_domain::CpuPolicy;
use nexus_domain::RequestId;
use nexus_domain::TaskId;
use nexus_storage::StoredSession;
use serde_json::Value;
use tracing::error;

use crate::CoreConnectionError;
use crate::CoreCreate;
use crate::CoreRegistryError;
use crate::CpuReservationRequest;
use crate::PanelState;
use crate::auth_routes::authorize_session;
use crate::auth_routes::error_response;
use crate::auth_routes::header_text;

pub(crate) fn core_routes() -> Router<PanelState> {
    Router::new()
        .route("/api/v1/cores", get(list_cores).post(create_core))
        .route("/api/v1/cores/{core_id}", get(get_core))
        .route(
            "/api/v1/cores/{core_id}/actions/test",
            post(test_core_connection),
        )
        .route(
            "/api/v1/cores/{core_id}/actions/reconnect",
            post(reconnect_core),
        )
        .route(
            "/api/v1/cores/{core_id}/cpu-topology",
            get(get_cpu_topology),
        )
        .route(
            "/api/v1/cores/{core_id}/cpu-policies:resolve",
            post(resolve_cpu_policy),
        )
        .route(
            "/api/v1/cores/{core_id}/cpu-reservations",
            get(list_cpu_reservations).post(reserve_cpu),
        )
        .route(
            "/api/v1/cores/{core_id}/cpu-reservations/{reservation_id}",
            delete(release_cpu),
        )
}

async fn list_cores(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }

    match state.cores().list().await {
        Ok(page) => Json(page).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn create_core(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    payload: Result<Json<CoreCreate>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let request = match payload {
        Ok(Json(request)) => request,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "VALIDATION_FAILED",
                "Request validation failed",
                request_id,
            );
        }
    };

    match state.cores().register(&request).await {
        Ok(core) => resource_response(StatusCode::CREATED, core),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn get_core(
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

    match state.cores().get(core_id).await {
        Ok(core) => resource_response(StatusCode::OK, core),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn test_core_connection(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(core_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };

    match state.cores().test_connection(core_id).await {
        Ok(result) => Json(result).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn reconnect_core(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(core_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };

    match state.cores().reconnect(core_id).await {
        Ok(core) => resource_response(StatusCode::ACCEPTED, core),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn get_cpu_topology(
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

    match state.cores().cpu_topology(core_id).await {
        Ok(topology) => Json(topology).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn resolve_cpu_policy(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(core_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<CpuPolicy>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Ok(Json(policy)) = payload else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
            "CPU policy validation failed",
            request_id,
        );
    };

    match state.cores().resolve_cpu_policy(core_id, &policy).await {
        Ok(result) => Json(result).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn list_cpu_reservations(
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

    match state.cores().list_cpu_reservations(core_id).await {
        Ok(reservations) => Json(reservations).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn reserve_cpu(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(core_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<CpuReservationRequest>, JsonRejection>,
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
        .reserve_cpu(core_id, &request, idempotency_key)
        .await
    {
        Ok(reservation) => (StatusCode::CREATED, Json(reservation)).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn release_cpu(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, reservation_id)): Path<(String, String)>,
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
    let Ok(reservation_id) = reservation_id.parse::<TaskId>() else {
        return validation_error(request_id);
    };

    match state
        .cores()
        .release_cpu(core_id, &reservation_id, idempotency_key)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

pub(crate) async fn authorize(
    state: &PanelState,
    headers: &HeaderMap,
    write: bool,
    request_id: RequestId,
) -> Result<StoredSession, Response> {
    let session = authorize_session(state, headers, write, request_id).await?;
    if !session.user().is_admin() {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "FORBIDDEN",
            "The requested operation is not permitted",
            request_id,
        ));
    }

    Ok(session)
}

pub(crate) fn parse_core_id(value: &str) -> Option<CoreId> {
    CoreId::from_str(value).ok()
}

pub(crate) fn invalid_core_id_response(request_id: RequestId) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "VALIDATION_FAILED",
        "Core ID is invalid",
        request_id,
    )
}

pub(crate) fn resource_response(status: StatusCode, value: Value) -> Response {
    let revision = value
        .get("revision")
        .and_then(Value::as_u64)
        .and_then(|revision| HeaderValue::from_str(&format!("\"{revision}\"")).ok());
    let mut response = (status, Json(value)).into_response();
    if let Some(revision) = revision {
        response.headers_mut().insert(ETAG, revision);
    }
    response
}

pub(crate) fn registry_error_response(error: CoreRegistryError, request_id: RequestId) -> Response {
    match error {
        CoreRegistryError::InvalidRequest { .. }
        | CoreRegistryError::InvalidSecret(_)
        | CoreRegistryError::Connection(CoreConnectionError::Endpoint(_)) => error_response(
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
            "Core registration is invalid",
            request_id,
        ),
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "INSTANCE_NOT_FOUND" =>
        {
            error_response(
                StatusCode::NOT_FOUND,
                "INSTANCE_NOT_FOUND",
                "Instance does not exist",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "CONFIG_DOCUMENT_NOT_FOUND" =>
        {
            error_response(
                StatusCode::NOT_FOUND,
                "CONFIG_DOCUMENT_NOT_FOUND",
                "Configuration document does not exist",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "CONFIG_REVISION_MISMATCH" =>
        {
            error_response(
                StatusCode::PRECONDITION_FAILED,
                "CONFIG_REVISION_MISMATCH",
                "Configuration document changed",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "CONFIG_PATCH_INVALID" =>
        {
            error_response(
                StatusCode::BAD_REQUEST,
                "CONFIG_PATCH_INVALID",
                "Configuration patch is invalid",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "CONFIG_PARSE_FAILED" =>
        {
            error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "CONFIG_PARSE_FAILED",
                "Configuration document could not be parsed",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "CONFIG_SCAN_TOO_LARGE" =>
        {
            error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "CONFIG_SCAN_TOO_LARGE",
                "Too many configuration documents",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "FILE_NOT_FOUND" =>
        {
            error_response(
                StatusCode::NOT_FOUND,
                "FILE_NOT_FOUND",
                "File does not exist",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if matches!(code.as_str(), "FILE_NOT_DIRECTORY" | "FILE_NOT_REGULAR") =>
        {
            error_response(
                StatusCode::CONFLICT,
                code.as_str(),
                "File path type does not allow this operation",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "FILE_PATH_FORBIDDEN" =>
        {
            error_response(
                StatusCode::FORBIDDEN,
                "FILE_PATH_FORBIDDEN",
                "File path is not allowed",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "FILE_REVISION_MISMATCH" =>
        {
            error_response(
                StatusCode::PRECONDITION_FAILED,
                "FILE_REVISION_MISMATCH",
                "File hash does not match",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if matches!(
                code.as_str(),
                "FILE_ALREADY_EXISTS" | "FILE_DIRECTORY_NOT_EMPTY"
            ) =>
        {
            error_response(
                StatusCode::CONFLICT,
                code.as_str(),
                "File target cannot be replaced",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "PAYLOAD_TOO_LARGE" =>
        {
            error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "PAYLOAD_TOO_LARGE",
                "File content exceeds the maximum size",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "FILE_OPERATION_FAILED" =>
        {
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "FILE_OPERATION_FAILED",
                "File operation failed",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "FILE_TASK_NOT_FOUND" =>
        {
            error_response(
                StatusCode::NOT_FOUND,
                "FILE_TASK_NOT_FOUND",
                "File task does not exist",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "FILE_TRANSFER_NOT_FOUND" =>
        {
            error_response(
                StatusCode::NOT_FOUND,
                "FILE_TRANSFER_NOT_FOUND",
                "File transfer does not exist",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if matches!(
                code.as_str(),
                "FILE_TRANSFER_OFFSET_MISMATCH"
                    | "FILE_TRANSFER_SIZE_MISMATCH"
                    | "FILE_TRANSFER_HASH_MISMATCH"
            ) =>
        {
            error_response(
                StatusCode::CONFLICT,
                code.as_str(),
                "File transfer state does not allow this operation",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "FILE_TRANSFER_LIMIT_REACHED" =>
        {
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "FILE_TRANSFER_LIMIT_REACHED",
                "The Core has reached its active file transfer limit",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "RUNTIME_NOT_FOUND" =>
        {
            error_response(
                StatusCode::NOT_FOUND,
                "RUNTIME_NOT_FOUND",
                "Runtime does not exist",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "TASK_NOT_FOUND" =>
        {
            error_response(
                StatusCode::NOT_FOUND,
                "TASK_NOT_FOUND",
                "Runtime task does not exist",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "PROVISION_TASK_NOT_FOUND" =>
        {
            error_response(
                StatusCode::NOT_FOUND,
                "PROVISION_TASK_NOT_FOUND",
                "Provision task does not exist",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if matches!(code.as_str(), "RUNTIME_ALREADY_EXISTS" | "RUNTIME_IN_USE") =>
        {
            error_response(
                StatusCode::CONFLICT,
                code.as_str(),
                "Runtime operation conflicts with the current state",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "INSTANCE_ALREADY_EXISTS" =>
        {
            error_response(
                StatusCode::CONFLICT,
                "INSTANCE_ALREADY_EXISTS",
                "Instance already exists",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "PROVISION_PLAN_EXPIRED" =>
        {
            error_response(
                StatusCode::CONFLICT,
                "PROVISION_PLAN_EXPIRED",
                "Provision plan must be resolved again",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "RUNTIME_INVALID" =>
        {
            error_response(
                StatusCode::BAD_REQUEST,
                "RUNTIME_INVALID",
                "Selected runtime is invalid",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "PROVISION_FAILED" =>
        {
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "PROVISION_FAILED",
                "Provision operation failed",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "INSTANCE_STATE_CONFLICT" =>
        {
            error_response(
                StatusCode::CONFLICT,
                "INSTANCE_STATE_CONFLICT",
                "Instance state does not allow this operation",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if matches!(
                code.as_str(),
                "BEDROCK_PROFILE_UNSUPPORTED"
                    | "PROXY_TOPOLOGY_UNSUPPORTED"
                    | "PROXY_SUBSERVER_LIMIT_REACHED"
            ) =>
        {
            error_response(
                StatusCode::CONFLICT,
                code.as_str(),
                "Proxy topology does not allow this subserver operation",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "PROXY_TARGET_INVALID" =>
        {
            error_response(
                StatusCode::BAD_REQUEST,
                code.as_str(),
                "Proxy subserver target is invalid",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "PROXY_TARGET_NOT_FOUND" =>
        {
            error_response(
                StatusCode::NOT_FOUND,
                code.as_str(),
                "Proxy subserver target does not exist",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "PROXY_SUBSERVER_NOT_FOUND" =>
        {
            error_response(
                StatusCode::NOT_FOUND,
                "PROXY_SUBSERVER_NOT_FOUND",
                "Proxy subserver does not exist",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "REVISION_MISMATCH" =>
        {
            error_response(
                StatusCode::PRECONDITION_FAILED,
                "REVISION_MISMATCH",
                "Instance revision does not match",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "INSTANCE_REVISION_CONFLICT" =>
        {
            error_response(
                StatusCode::PRECONDITION_FAILED,
                "INSTANCE_REVISION_CONFLICT",
                "Instance revision does not match",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if matches!(
                code.as_str(),
                "CPU_POLICY_INVALID" | "CPU_RESERVATION_REQUIRES_EXCLUSIVE"
            ) =>
        {
            error_response(
                StatusCode::BAD_REQUEST,
                code.as_str(),
                "CPU reservation request is invalid",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if matches!(
                code.as_str(),
                "CPU_CAPACITY_UNAVAILABLE" | "CPU_RESERVATION_CONFLICT"
            ) =>
        {
            error_response(
                StatusCode::CONFLICT,
                code.as_str(),
                "CPU reservation conflicts with available capacity",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "CPU_RESERVATION_NOT_FOUND" =>
        {
            error_response(
                StatusCode::NOT_FOUND,
                "CPU_RESERVATION_NOT_FOUND",
                "CPU reservation does not exist",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "CPU_RESERVATION_FAILED" =>
        {
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CPU_RESERVATION_FAILED",
                "CPU reservation operation failed",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "BAD_REQUEST" =>
        {
            error_response(
                StatusCode::BAD_REQUEST,
                "VALIDATION_FAILED",
                "Request validation failed",
                request_id,
            )
        }
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "PRECONDITION_REQUIRED" =>
        {
            error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "PRECONDITION_REQUIRED",
                "Idempotency-Key is required",
                request_id,
            )
        }
        CoreRegistryError::AlreadyExists { .. } => error_response(
            StatusCode::CONFLICT,
            "CORE_ALREADY_EXISTS",
            "Core is already registered",
            request_id,
        ),
        CoreRegistryError::NotFound { .. } => error_response(
            StatusCode::NOT_FOUND,
            "CORE_NOT_FOUND",
            "Core registration does not exist",
            request_id,
        ),
        CoreRegistryError::Connection(_)
        | CoreRegistryError::ConnectionTimeout
        | CoreRegistryError::ConnectionUnavailable => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "CORE_UNAVAILABLE",
            "Core connection could not be established",
            request_id,
        ),
        internal_error => {
            error!(
                error = %internal_error,
                %request_id,
                "Core management request failed"
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "The request could not be completed",
                request_id,
            )
        }
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
