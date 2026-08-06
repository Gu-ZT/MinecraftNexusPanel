//! Panel 用户级审计查询路由。
//!
//! 审计事件由 HTTP 中间件统一写入 SQLite；本模块只提供受权用户只读查询，避免
//! 让具体业务路由重复实现审计分页和敏感字段过滤。

use std::collections::HashMap;

use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::http::header::CONTENT_DISPOSITION;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use nexus_domain::RequestId;
use nexus_storage::StorageError;
use nexus_storage::StoredAuditEvent;
use serde_json::Error as JsonError;
use serde_json::Value;
use serde_json::json;
use tokio::task::JoinError;
use tokio::task::spawn_blocking;

use crate::PanelState;
use crate::auth_routes::authorize_session;
use crate::auth_routes::error_response;
use crate::permissions::AUDIT_READ;

/// 注册 Panel 审计查询端点。
pub(crate) fn audit_routes() -> Router<PanelState> {
    Router::new()
        .route("/api/v1/audit-events", get(list_audit_events))
        .route("/api/v1/audit-events:export", get(export_audit_events))
}

async fn list_audit_events(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize_audit_read(&state, &headers, request_id).await {
        return response;
    }
    let Some(limit) = parse_limit(&query) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
            "limit must be between 1 and 200",
            request_id,
        );
    };

    let store = state.store().clone();
    match spawn_blocking(move || store.list_audit_events(limit)).await {
        Ok(Ok(events)) => Json(json!({
            "items": events
                .iter()
                .map(audit_event_value)
                .collect::<Vec<_>>(),
        }))
        .into_response(),
        Ok(Err(error)) => audit_storage_error(&error, request_id),
        Err(error) => audit_worker_error(&error, request_id),
    }
}

async fn export_audit_events(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize_audit_read(&state, &headers, request_id).await {
        return response;
    }

    let store = state.store().clone();
    let retention_events = store.audit_retention_events();
    match spawn_blocking(move || store.list_audit_events(retention_events)).await {
        Ok(Ok(events)) => match encode_ndjson(&events) {
            Ok(body) => (
                [
                    (CONTENT_TYPE, "application/x-ndjson; charset=utf-8"),
                    (
                        CONTENT_DISPOSITION,
                        "attachment; filename=\"mcnp-audit-events.ndjson\"",
                    ),
                ],
                body,
            )
                .into_response(),
            Err(error) => {
                tracing::error!(%error, %request_id, "Unable to encode Panel audit export");
                audit_unavailable(request_id)
            }
        },
        Ok(Err(error)) => audit_storage_error(&error, request_id),
        Err(error) => audit_worker_error(&error, request_id),
    }
}

async fn authorize_audit_read(
    state: &PanelState,
    headers: &HeaderMap,
    request_id: RequestId,
) -> Result<(), Response> {
    let session = authorize_session(state, headers, false, request_id).await?;
    if !session.user().has_permission(AUDIT_READ) {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "FORBIDDEN",
            "The requested operation is not permitted",
            request_id,
        ));
    }

    Ok(())
}

fn audit_event_value(event: &StoredAuditEvent) -> Value {
    json!({
        "id": event.id(),
        "occurredAt": event.occurred_at(),
        "userId": event.user_id(),
        "requestId": event.request_id(),
        "sourceIp": event.source_ip(),
        "method": event.method(),
        "path": event.path(),
        "statusCode": event.status_code(),
        "permissionResult": event.permission_result(),
    })
}

fn encode_ndjson(events: &[StoredAuditEvent]) -> Result<String, JsonError> {
    let mut output = String::new();
    for event in events {
        output.push_str(&serde_json::to_string(&audit_event_value(event))?);
        output.push('\n');
    }
    Ok(output)
}

fn audit_storage_error(error: &StorageError, request_id: RequestId) -> Response {
    tracing::error!(%error, %request_id, "Unable to read Panel audit events");
    audit_unavailable(request_id)
}

fn audit_worker_error(error: &JoinError, request_id: RequestId) -> Response {
    tracing::error!(%error, %request_id, "Panel audit query worker failed");
    audit_unavailable(request_id)
}

fn audit_unavailable(request_id: RequestId) -> Response {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_ERROR",
        "Panel audit events are unavailable",
        request_id,
    )
}

fn parse_limit(query: &HashMap<String, String>) -> Option<usize> {
    match query.get("limit") {
        Some(value) => value
            .parse::<usize>()
            .ok()
            .filter(|value| (1..=200).contains(value)),
        None => Some(100),
    }
}
