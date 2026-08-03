use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::body::Bytes;
use axum::extract::Extension;
use axum::extract::Path;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::http::header::ETAG;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use nexus_domain::CoreId;
use nexus_domain::InstanceId;
use nexus_domain::RequestId;
use serde_json::Value;

use crate::CoreRegistryError;
use crate::PanelState;
use crate::auth_routes::error_response;
use crate::auth_routes::header_text;
use crate::core_routes::authorize;
use crate::core_routes::parse_core_id;
use crate::core_routes::registry_error_response;

const MAXIMUM_CONFIG_BYTES: usize = 1024 * 1024;
const CONFIG_READ_CHUNK_BYTES: usize = 32 * 1024;

pub(crate) fn config_routes() -> Router<PanelState> {
    Router::new()
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/config-documents",
            get(list_config_documents),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/config-documents:scan",
            post(scan_config_documents),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/config-documents/{document_id}",
            get(get_config_document),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/config-documents/{document_id}/values",
            patch(patch_config_document),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/config-documents/{document_id}/raw",
            get(read_raw_config).put(write_raw_config),
        )
}

async fn list_config_documents(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    scan_config_documents_inner(state, request_id, core_id, instance_id, headers).await
}

async fn scan_config_documents(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    scan_config_documents_inner(state, request_id, core_id, instance_id, headers).await
}

async fn scan_config_documents_inner(
    state: PanelState,
    request_id: RequestId,
    core_id: String,
    instance_id: String,
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
        .scan_config_documents(core_id, &instance_id)
        .await
    {
        Ok(documents) => Json(documents).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn get_config_document(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id, document_id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };
    if !is_document_id(&document_id) {
        return validation_error(request_id);
    }

    match state
        .cores()
        .get_config_document(core_id, &instance_id, &document_id)
        .await
    {
        Ok(document) => config_document_response(document),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn patch_config_document(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id, document_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return validation_error(request_id),
    };
    let Some(revision) = payload.get("revision").and_then(Value::as_str) else {
        return validation_error(request_id);
    };
    let Some(patch) = payload.get("patch").filter(|value| value.is_object()) else {
        return validation_error(request_id);
    };
    let allow_lossy = match payload.get("allowLossy") {
        None => false,
        Some(Value::Bool(value)) => *value,
        Some(_) => return validation_error(request_id),
    };
    let Some(idempotency_key) = idempotency_key(&headers) else {
        return precondition_required_response(request_id);
    };
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };
    if !is_document_id(&document_id) {
        return validation_error(request_id);
    }

    match state
        .cores()
        .patch_config_document(
            core_id,
            &instance_id,
            &document_id,
            revision,
            patch,
            idempotency_key,
            allow_lossy,
        )
        .await
    {
        Ok(document) => config_document_response(document),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn read_raw_config(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id, document_id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };
    if !is_document_id(&document_id) {
        return validation_error(request_id);
    }
    let document = match state
        .cores()
        .get_config_document(core_id, &instance_id, &document_id)
        .await
    {
        Ok(document) => document,
        Err(error) => return registry_error_response(error, request_id),
    };
    let Some(path) = document.get("path").and_then(Value::as_str) else {
        return invalid_core_response(request_id);
    };
    let (content, sha256) = match read_config_content(&state, core_id, &instance_id, path).await {
        Ok(content) => content,
        Err(RawConfigError::Registry(error)) => return registry_error_response(error, request_id),
        Err(RawConfigError::InvalidResponse) => return invalid_core_response(request_id),
    };
    let mut response = (StatusCode::OK, Body::from(content)).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    if let Ok(value) = HeaderValue::from_str(&format!("\"{sha256}\"")) {
        response.headers_mut().insert(ETAG, value);
    }
    response
}

async fn write_raw_config(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id, document_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    if body.len() > MAXIMUM_CONFIG_BYTES || std::str::from_utf8(&body).is_err() {
        return validation_error(request_id);
    }
    let Some(idempotency_key) = idempotency_key(&headers) else {
        return precondition_required_response(request_id);
    };
    let expected_sha256 = match expected_file_hash(&headers) {
        Ok(value) => value,
        Err(()) => return validation_error(request_id),
    };
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };
    if !is_document_id(&document_id) {
        return validation_error(request_id);
    }
    let document = match state
        .cores()
        .get_config_document(core_id, &instance_id, &document_id)
        .await
    {
        Ok(document) => document,
        Err(error) => return registry_error_response(error, request_id),
    };
    let Some(path) = document.get("path").and_then(Value::as_str) else {
        return invalid_core_response(request_id);
    };

    match state
        .cores()
        .write_instance_file(
            core_id,
            &instance_id,
            path,
            &body,
            expected_sha256.as_deref(),
            idempotency_key,
        )
        .await
    {
        Ok(_) => match state
            .cores()
            .get_config_document(core_id, &instance_id, &document_id)
            .await
        {
            Ok(document) => config_document_response(document),
            Err(error) => registry_error_response(error, request_id),
        },
        Err(error) => registry_error_response(error, request_id),
    }
}

enum RawConfigError {
    Registry(CoreRegistryError),
    InvalidResponse,
}

async fn read_config_content(
    state: &PanelState,
    core_id: CoreId,
    instance_id: &InstanceId,
    path: &str,
) -> Result<(Vec<u8>, String), RawConfigError> {
    let mut content = Vec::new();
    let mut offset = 0;
    let mut file_sha256 = None;
    loop {
        let chunk = state
            .cores()
            .read_instance_file(core_id, instance_id, path, offset, CONFIG_READ_CHUNK_BYTES)
            .await
            .map_err(RawConfigError::Registry)?;
        let data_base64 = chunk.data_base64().to_owned();
        let sha256 = chunk.sha256().to_owned();
        let eof = chunk.eof();
        let bytes = STANDARD
            .decode(data_base64)
            .map_err(|_| RawConfigError::InvalidResponse)?;
        if file_sha256
            .as_ref()
            .is_some_and(|current: &String| current != &sha256)
        {
            return Err(RawConfigError::InvalidResponse);
        }
        file_sha256 = Some(sha256);
        if content.len().saturating_add(bytes.len()) > MAXIMUM_CONFIG_BYTES {
            return Err(RawConfigError::InvalidResponse);
        }
        if bytes.is_empty() && !eof {
            return Err(RawConfigError::InvalidResponse);
        }
        offset = offset.saturating_add(bytes.len() as u64);
        content.extend(bytes);
        if eof {
            return Ok((content, file_sha256.ok_or(RawConfigError::InvalidResponse)?));
        }
    }
}

fn parse_ids(core_id: &str, instance_id: &str) -> Option<(CoreId, InstanceId)> {
    Some((parse_core_id(core_id)?, instance_id.parse().ok()?))
}

fn is_document_id(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn idempotency_key(headers: &HeaderMap) -> Option<&str> {
    header_text(headers, "idempotency-key").filter(|value| value.parse::<RequestId>().is_ok())
}

fn expected_file_hash(headers: &HeaderMap) -> Result<Option<String>, ()> {
    let Some(value) = header_text(headers, "if-match") else {
        return Ok(None);
    };
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(());
    };
    if !is_document_id(value) {
        return Err(());
    }
    Ok(Some(value.to_owned()))
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

fn invalid_core_response(request_id: RequestId) -> Response {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_ERROR",
        "Core returned an invalid configuration response",
        request_id,
    )
}

fn config_document_response(document: Value) -> Response {
    let mut response = Json(document.clone()).into_response();
    if let Some(hash) = document
        .get("contentHash")
        .and_then(Value::as_str)
        .or_else(|| document.get("revision").and_then(Value::as_str))
        && let Ok(value) = HeaderValue::from_str(&format!("\"{hash}\""))
    {
        response.headers_mut().insert(ETAG, value);
    }
    response
}
