use std::collections::HashMap;

use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::body::Bytes;
use axum::extract::Extension;
use axum::extract::Path;
use axum::extract::Query;
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
use axum::routing::post;
use nexus_domain::CoreId;
use nexus_domain::FileContent;
use nexus_domain::FileEntry;
use nexus_domain::InstanceId;
use nexus_domain::RequestId;
use nexus_domain::TaskId;
use serde_json::Value;

use crate::PanelState;
use crate::auth_routes::error_response;
use crate::auth_routes::header_text;
use crate::core_routes::authorize;
use crate::core_routes::parse_core_id;
use crate::core_routes::registry_error_response;

const MAXIMUM_FILE_READ_BYTES: usize = 32 * 1024;
const MAXIMUM_FILE_WRITE_BYTES: usize = 1024 * 1024;

pub(crate) fn file_routes() -> Router<PanelState> {
    Router::new()
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/files",
            get(list_instance_files).delete(delete_instance_file),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/directories",
            post(create_instance_directory),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/file-actions/move",
            post(move_instance_file),
        )
        .route(
            "/api/v1/cores/{core_id}/file-tasks/{task_id}",
            get(get_file_task),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/file-content",
            get(read_instance_file).put(write_instance_file),
        )
}

async fn list_instance_files(
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
    let Some(path) = query.get("path") else {
        return validation_error(request_id);
    };
    let Some(limit) = optional_limit(&query) else {
        return validation_error(request_id);
    };

    match state
        .cores()
        .list_instance_files(
            core_id,
            &instance_id,
            path,
            query.get("cursor").map(String::as_str),
            limit,
        )
        .await
    {
        Ok(page) => Json(page).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn delete_instance_file(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(path) = query.get("path").filter(|path| !path.is_empty()) else {
        return validation_error(request_id);
    };
    if query.get("confirmation").map(String::as_str) != Some("DELETE") {
        return validation_error(request_id);
    }
    let recursive = match query_boolean(&query, "recursive") {
        Ok(value) => value.unwrap_or(false),
        Err(()) => return validation_error(request_id),
    };
    let Some(idempotency_key) = idempotency_key(&headers) else {
        return precondition_required_response(request_id);
    };
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };

    match state
        .cores()
        .delete_instance_file(core_id, &instance_id, path, recursive, idempotency_key)
        .await
    {
        Ok(task) => (StatusCode::ACCEPTED, Json(task)).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn get_file_task(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, task_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return validation_error(request_id);
    };
    let Ok(task_id) = task_id.parse::<TaskId>() else {
        return validation_error(request_id);
    };

    match state.cores().get_file_task(core_id, &task_id).await {
        Ok(task) => Json(task).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn read_instance_file(
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
    let Some(path) = query.get("path").filter(|path| !path.is_empty()) else {
        return validation_error(request_id);
    };
    let offset = match query.get("offset") {
        Some(value) => match value.parse::<u64>() {
            Ok(offset) => offset,
            Err(_) => return validation_error(request_id),
        },
        None => 0,
    };
    let length = match query.get("length") {
        Some(value) => match value
            .parse::<usize>()
            .ok()
            .filter(|length| (1..=MAXIMUM_FILE_READ_BYTES).contains(length))
        {
            Some(length) => length,
            None => return validation_error(request_id),
        },
        None => MAXIMUM_FILE_READ_BYTES,
    };

    match state
        .cores()
        .read_instance_file(core_id, &instance_id, path, offset, length)
        .await
    {
        Ok(content) => file_content_response(content, request_id),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn write_instance_file(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    if body.len() > MAXIMUM_FILE_WRITE_BYTES {
        return error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "PAYLOAD_TOO_LARGE",
            "File content exceeds the maximum size",
            request_id,
        );
    }
    let Some(idempotency_key) = idempotency_key(&headers) else {
        return precondition_required_response(request_id);
    };
    let expected_sha256 = match expected_file_hash(&headers) {
        Ok(hash) => hash,
        Err(()) => return validation_error(request_id),
    };
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };
    let Some(path) = query.get("path").filter(|path| !path.is_empty()) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
            "File path is required",
            request_id,
        );
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
        Ok(entry) => file_entry_response(entry),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn create_instance_directory(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
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
    let Some(path) = payload
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    else {
        return validation_error(request_id);
    };
    let recursive = match optional_boolean(&payload, "recursive") {
        Ok(value) => value.unwrap_or(false),
        Err(()) => return validation_error(request_id),
    };
    let Some(idempotency_key) = idempotency_key(&headers) else {
        return precondition_required_response(request_id);
    };
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };

    match state
        .cores()
        .create_instance_directory(core_id, &instance_id, path, recursive, idempotency_key)
        .await
    {
        Ok(entry) => file_entry_response(entry),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn move_instance_file(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
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
    let Some(from) = payload
        .get("from")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    else {
        return validation_error(request_id);
    };
    let Some(to) = payload
        .get("to")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    else {
        return validation_error(request_id);
    };
    let overwrite = match optional_boolean(&payload, "overwrite") {
        Ok(value) => value.unwrap_or(false),
        Err(()) => return validation_error(request_id),
    };
    let Some(idempotency_key) = idempotency_key(&headers) else {
        return precondition_required_response(request_id);
    };
    let Some((core_id, instance_id)) = parse_ids(&core_id, &instance_id) else {
        return validation_error(request_id);
    };

    match state
        .cores()
        .move_instance_file(core_id, &instance_id, from, to, overwrite, idempotency_key)
        .await
    {
        Ok(entry) => file_entry_response(entry),
        Err(error) => registry_error_response(error, request_id),
    }
}

fn file_entry_response(entry: FileEntry) -> Response {
    let mut response = Json(&entry).into_response();
    if let Some(sha256) = entry.sha256()
        && let Ok(etag) = HeaderValue::from_str(&format!("\"{sha256}\""))
    {
        response.headers_mut().insert(ETAG, etag);
    }
    response
}

fn file_content_response(content: FileContent, request_id: RequestId) -> Response {
    let Ok(bytes) = decode_content(&content) else {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "Core returned invalid file content",
            request_id,
        );
    };
    let Ok(etag) = HeaderValue::from_str(&format!("\"{}\"", content.sha256())) else {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "Core returned an invalid file hash",
            request_id,
        );
    };
    let Ok(eof) = HeaderValue::from_str(if content.eof() { "true" } else { "false" }) else {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "File response metadata is invalid",
            request_id,
        );
    };
    let mut response = (StatusCode::OK, Body::from(bytes)).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response.headers_mut().insert(ETAG, etag);
    response.headers_mut().insert("x-mcnp-file-eof", eof);
    response
}

fn decode_content(content: &FileContent) -> Result<Vec<u8>, ()> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    STANDARD.decode(content.data_base64()).map_err(|_| ())
}

fn parse_ids(core_id: &str, instance_id: &str) -> Option<(CoreId, InstanceId)> {
    Some((parse_core_id(core_id)?, instance_id.parse().ok()?))
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

fn optional_boolean(payload: &Value, name: &str) -> Result<Option<bool>, ()> {
    match payload.get(name) {
        None => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(()),
    }
}

fn query_boolean(query: &HashMap<String, String>, name: &str) -> Result<Option<bool>, ()> {
    match query.get(name) {
        None => Ok(None),
        Some(value) => value.parse::<bool>().map(Some).map_err(|_| ()),
    }
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
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
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
