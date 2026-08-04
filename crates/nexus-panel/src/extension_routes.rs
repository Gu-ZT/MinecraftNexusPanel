use std::collections::HashMap;

use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use nexus_domain::ExtensionKind;
use nexus_domain::FilePage;
use nexus_domain::Instance;
use nexus_domain::InstanceId;
use nexus_domain::RequestId;
use serde_json::from_value;
use serde_json::json;

use crate::CoreConnectionError;
use crate::CoreRegistryError;
use crate::PanelState;
use crate::auth_routes::error_response;
use crate::core_routes::authorize;
use crate::core_routes::invalid_core_id_response;
use crate::core_routes::parse_core_id;
use crate::core_routes::registry_error_response;
use crate::install_template_catalog::install_template;

const EXTENSION_DIRECTORY_LIST_LIMIT: usize = 200;

pub(crate) fn extension_routes() -> Router<PanelState> {
    Router::new().route(
        "/api/v1/cores/{core_id}/instances/{instance_id}/extensions",
        get(list_instance_extensions),
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

    Json(json!({
        "templateId": template.id(),
        "kind": kind,
        "directories": directory_pages,
    }))
    .into_response()
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
