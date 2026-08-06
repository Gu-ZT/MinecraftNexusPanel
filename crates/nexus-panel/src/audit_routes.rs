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
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use nexus_domain::RequestId;
use serde_json::json;
use tokio::task::spawn_blocking;

use crate::PanelState;
use crate::auth_routes::authorize_session;
use crate::auth_routes::error_response;
use crate::permissions::AUDIT_READ;

/// 注册 Panel 审计查询端点。
pub(crate) fn audit_routes() -> Router<PanelState> {
    Router::new().route("/api/v1/audit-events", get(list_audit_events))
}

async fn list_audit_events(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<HashMap<String, String>>,
    headers: axum::http::HeaderMap,
) -> Response {
    let session = match authorize_session(&state, &headers, false, request_id).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    if !session.user().has_permission(AUDIT_READ) {
        return error_response(
            StatusCode::FORBIDDEN,
            "FORBIDDEN",
            "The requested operation is not permitted",
            request_id,
        );
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
                .into_iter()
                .map(|event| {
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
                })
                .collect::<Vec<_>>(),
        }))
        .into_response(),
        Ok(Err(error)) => {
            tracing::error!(%error, %request_id, "Unable to read Panel audit events");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Panel audit events are unavailable",
                request_id,
            )
        }
        Err(error) => {
            tracing::error!(%error, %request_id, "Panel audit query worker failed");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Panel audit events are unavailable",
                request_id,
            )
        }
    }
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
