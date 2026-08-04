use std::collections::HashMap;

use axum::Extension;
use axum::Json;
use axum::Router;
use axum::body::Bytes;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use nexus_domain::ExtensionInstall;
use nexus_domain::ExtensionKind;
use nexus_domain::FilePage;
use nexus_domain::Instance;
use nexus_domain::InstanceId;
use nexus_domain::RequestId;
use serde_json::from_value;
use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::CoreConnectionError;
use crate::CoreRegistryError;
use crate::PanelState;
use crate::auth_routes::error_response;
use crate::auth_routes::header_text;
use crate::core_routes::authorize;
use crate::core_routes::invalid_core_id_response;
use crate::core_routes::parse_core_id;
use crate::core_routes::registry_error_response;
use crate::install_template_catalog::install_template;

const EXTENSION_DIRECTORY_LIST_LIMIT: usize = 200;
const MAXIMUM_EXTENSION_WRITE_BYTES: usize = 1024 * 1024;

pub(crate) fn extension_routes() -> Router<PanelState> {
    Router::new().route(
        "/api/v1/cores/{core_id}/instances/{instance_id}/extensions",
        get(list_instance_extensions)
            .put(write_instance_extension)
            .delete(delete_instance_extension),
    )
}

async fn list_instance_extensions(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(instance_id) = instance_id.parse::<InstanceId>().ok() else {
        return validation_error(request_id);
    };
    let Some(template_id) = query.get("templateId").filter(|value| !value.is_empty()) else {
        return validation_error(request_id);
    };
    let Some(kind) = query
        .get("kind")
        .and_then(|value| from_value::<ExtensionKind>(json!(value)).ok())
    else {
        return validation_error(request_id);
    };
    let Some(template) = install_template(template_id) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Install template does not exist",
            request_id,
        );
    };

    let instance = match state.cores().get_instance(core_id, &instance_id).await {
        Ok(value) => match from_value::<Instance>(value) {
            Ok(instance) => instance,
            Err(_) => return invalid_core_response(request_id),
        },
        Err(error) => return registry_error_response(error, request_id),
    };
    if instance.kind() != template.instance_kind() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TEMPLATE_INSTANCE_KIND_MISMATCH",
            "Install template does not match the instance type",
            request_id,
        );
    }

    let directories = template.extension_directories(kind);
    if directories.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_KIND_UNSUPPORTED",
            "The install template does not declare this extension kind",
            request_id,
        );
    }

    let mut directory_pages = Vec::with_capacity(directories.len());
    for directory in directories {
        let page = match state
            .cores()
            .list_instance_files(
                core_id,
                &instance_id,
                directory,
                None,
                Some(EXTENSION_DIRECTORY_LIST_LIMIT),
            )
            .await
        {
            Ok(page) => page,
            Err(error) if is_missing_directory(&error) => FilePage::new(Vec::new(), None),
            Err(error) => return registry_error_response(error, request_id),
        };
        directory_pages.push(json!({
            "path": directory,
            "page": page,
        }));
    }
    let installations = match state
        .cores()
        .list_extension_installs(core_id, &instance_id, kind)
        .await
    {
        Ok(installations) => installations,
        Err(error) => return registry_error_response(error, request_id),
    };

    Json(json!({
        "templateId": template.id(),
        "kind": kind,
        "directories": directory_pages,
        "installations": installations,
    }))
    .into_response()
}

async fn write_instance_extension(
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
    if body.len() > MAXIMUM_EXTENSION_WRITE_BYTES {
        return error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "PAYLOAD_TOO_LARGE",
            "Extension content exceeds the maximum size",
            request_id,
        );
    }
    let Some(idempotency_key) =
        header_text(&headers, "idempotency-key").filter(|value| value.parse::<RequestId>().is_ok())
    else {
        return error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "PRECONDITION_REQUIRED",
            "Idempotency-Key is required",
            request_id,
        );
    };
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(instance_id) = instance_id.parse::<InstanceId>().ok() else {
        return validation_error(request_id);
    };
    let Some(template_id) = query.get("templateId").filter(|value| !value.is_empty()) else {
        return validation_error(request_id);
    };
    let Some(kind) = query
        .get("kind")
        .and_then(|value| from_value::<ExtensionKind>(json!(value)).ok())
    else {
        return validation_error(request_id);
    };
    let Some(path) = query.get("path").filter(|value| !value.is_empty()) else {
        return validation_error(request_id);
    };
    let expected_sha256 = match expected_file_hash(&headers) {
        Ok(hash) => hash,
        Err(()) => return validation_error(request_id),
    };
    let Some(template) = install_template(template_id) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Install template does not exist",
            request_id,
        );
    };

    let instance = match state.cores().get_instance(core_id, &instance_id).await {
        Ok(value) => match from_value::<Instance>(value) {
            Ok(instance) => instance,
            Err(_) => return invalid_core_response(request_id),
        },
        Err(error) => return registry_error_response(error, request_id),
    };
    if instance.kind() != template.instance_kind() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TEMPLATE_INSTANCE_KIND_MISMATCH",
            "Install template does not match the instance type",
            request_id,
        );
    }

    let directories = template.extension_directories(kind);
    if directories.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_KIND_UNSUPPORTED",
            "The install template does not declare this extension kind",
            request_id,
        );
    }
    if !directories
        .iter()
        .any(|directory| is_extension_path(path, directory))
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_PATH_OUTSIDE_LAYOUT",
            "Extension path is outside the declared template directories",
            request_id,
        );
    }

    let entry = match state
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
        Ok(entry) => entry,
        Err(error) => return registry_error_response(error, request_id),
    };
    let Some(sha256) = entry.sha256() else {
        return error_response(
            StatusCode::BAD_GATEWAY,
            "INTERNAL_ERROR",
            "Core did not return an extension file hash",
            request_id,
        );
    };
    let install = ExtensionInstall::new(
        RequestId::new().to_string(),
        kind,
        path.to_owned(),
        sha256.to_owned(),
        "LOCAL".to_owned(),
        None,
        None,
        current_timestamp(),
    );
    match state
        .cores()
        .upsert_extension_install(core_id, &instance_id, &install)
        .await
    {
        Ok(_) => Json(entry).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn delete_instance_extension(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(instance_id) = instance_id.parse::<InstanceId>().ok() else {
        return validation_error(request_id);
    };
    let Some(template_id) = query.get("templateId").filter(|value| !value.is_empty()) else {
        return validation_error(request_id);
    };
    let Some(kind) = query
        .get("kind")
        .and_then(|value| from_value::<ExtensionKind>(json!(value)).ok())
    else {
        return validation_error(request_id);
    };
    let Some(path) = query.get("path").filter(|value| !value.is_empty()) else {
        return validation_error(request_id);
    };
    if query.get("confirmation").map(String::as_str) != Some("DELETE") {
        return validation_error(request_id);
    }
    let Some(idempotency_key) =
        header_text(&headers, "idempotency-key").filter(|value| value.parse::<RequestId>().is_ok())
    else {
        return error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "PRECONDITION_REQUIRED",
            "Idempotency-Key is required",
            request_id,
        );
    };
    let Some(template) = install_template(template_id) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Install template does not exist",
            request_id,
        );
    };

    let instance = match state.cores().get_instance(core_id, &instance_id).await {
        Ok(value) => match from_value::<Instance>(value) {
            Ok(instance) => instance,
            Err(_) => return invalid_core_response(request_id),
        },
        Err(error) => return registry_error_response(error, request_id),
    };
    if instance.kind() != template.instance_kind() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TEMPLATE_INSTANCE_KIND_MISMATCH",
            "Install template does not match the instance type",
            request_id,
        );
    }

    let directories = template.extension_directories(kind);
    if directories.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_KIND_UNSUPPORTED",
            "The install template does not declare this extension kind",
            request_id,
        );
    }
    if !directories
        .iter()
        .any(|directory| is_extension_path(path, directory))
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_PATH_OUTSIDE_LAYOUT",
            "Extension path is outside the declared template directories",
            request_id,
        );
    }

    match state
        .cores()
        .delete_instance_file(core_id, &instance_id, path, false, idempotency_key)
        .await
    {
        Ok(task) => {
            if let Err(error) = state
                .cores()
                .delete_extension_install(core_id, &instance_id, path)
                .await
            {
                tracing::error!(%error, %path, "Failed to remove extension installation record");
            }
            (StatusCode::ACCEPTED, Json(task)).into_response()
        }
        Err(error) => registry_error_response(error, request_id),
    }
}

fn is_extension_path(path: &str, directory: &str) -> bool {
    path.strip_prefix(directory)
        .is_some_and(|suffix| suffix.starts_with('/') && suffix.len() > 1)
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

fn is_missing_directory(error: &CoreRegistryError) -> bool {
    matches!(
        error,
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "FILE_NOT_FOUND"
    )
}

fn invalid_core_response(request_id: RequestId) -> Response {
    error_response(
        StatusCode::BAD_GATEWAY,
        "INTERNAL_ERROR",
        "Core returned an invalid instance response",
        request_id,
    )
}

fn validation_error(request_id: RequestId) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "VALIDATION_FAILED",
        "Extension scan parameters are invalid",
        request_id,
    )
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}
