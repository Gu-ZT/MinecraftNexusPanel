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
use axum::routing::get;
use axum::routing::post;
use nexus_domain::CoreId;
use nexus_domain::RequestId;
use serde_json::Value;
use tracing::error;

use crate::AuthError;
use crate::CoreConnectionError;
use crate::CoreCreate;
use crate::CoreRegistryError;
use crate::PanelState;
use crate::auth_routes::RequestCredential;
use crate::auth_routes::auth_error_response;
use crate::auth_routes::authenticate;
use crate::auth_routes::error_response;
use crate::auth_routes::header_text;
use crate::auth_routes::request_credential;
use crate::auth_routes::run_blocking;

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

async fn authorize(
    state: &PanelState,
    headers: &HeaderMap,
    write: bool,
    request_id: RequestId,
) -> Result<(), Response> {
    let credential = request_credential(headers)
        .ok_or_else(|| auth_error_response(AuthError::InvalidSession, request_id))?;
    let browser_session = matches!(&credential, RequestCredential::Browser(_));
    let csrf_token = header_text(headers, "x-csrf-token").map(str::to_owned);
    let auth = state.auth().clone();
    let session = run_blocking(move || authenticate(&auth, &credential))
        .await
        .map_err(|error| auth_error_response(error, request_id))?;
    if write && browser_session {
        state
            .auth()
            .verify_csrf(
                &session,
                csrf_token
                    .as_deref()
                    .ok_or_else(|| auth_error_response(AuthError::InvalidCsrfToken, request_id))?,
            )
            .map_err(|error| auth_error_response(error, request_id))?;
    }
    if !session.user().is_admin() {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "FORBIDDEN",
            "The requested operation is not permitted",
            request_id,
        ));
    }

    Ok(())
}

fn parse_core_id(value: &str) -> Option<CoreId> {
    CoreId::from_str(value).ok()
}

fn invalid_core_id_response(request_id: RequestId) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "VALIDATION_FAILED",
        "Core ID is invalid",
        request_id,
    )
}

fn resource_response(status: StatusCode, value: Value) -> Response {
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

fn registry_error_response(error: CoreRegistryError, request_id: RequestId) -> Response {
    match error {
        CoreRegistryError::InvalidRequest { .. }
        | CoreRegistryError::InvalidSecret(_)
        | CoreRegistryError::Connection(CoreConnectionError::Endpoint(_)) => error_response(
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
            "Core registration is invalid",
            request_id,
        ),
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
        CoreRegistryError::Connection(_) | CoreRegistryError::ConnectionTimeout => error_response(
            StatusCode::BAD_GATEWAY,
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
