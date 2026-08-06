//! Panel 用户创建和列表路由。
//!
//! 当前仅管理员可管理用户，并且只能创建非管理员账户；可授予权限集合由实际
//! 完成服务端检查的能力限定，避免先写入无法执行的授权。

use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use nexus_domain::RequestId;
use serde_json::json;

use crate::PanelState;
use crate::UserCreate;
use crate::UserResponse;
use crate::auth_routes::auth_error_response;
use crate::auth_routes::error_response;
use crate::auth_routes::run_blocking;
use crate::core_routes::authorize;

/// 注册用户列表和创建端点。
pub(crate) fn user_routes() -> Router<PanelState> {
    Router::new().route("/api/v1/users", get(list_users).post(create_user))
}

async fn list_users(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }

    let auth = state.auth().clone();
    match run_blocking(move || auth.list_users()).await {
        Ok(users) => Json(json!({
            "items": users.iter().map(UserResponse::from).collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(error) => auth_error_response(error, request_id),
    }
}

async fn create_user(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    payload: Result<Json<UserCreate>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let request = match payload {
        Ok(Json(request)) if request.invalid_field().is_none() => request,
        Ok(Json(request)) => {
            return validation_error(request.invalid_field(), request_id);
        }
        Err(_) => return validation_error(None, request_id),
    };

    let auth = state.auth().clone();
    match run_blocking(move || auth.create_user(&request)).await {
        Ok(Some(user)) => (StatusCode::CREATED, Json(UserResponse::from(&user))).into_response(),
        Ok(None) => error_response(
            StatusCode::CONFLICT,
            "USER_ALREADY_EXISTS",
            "A user with this username already exists",
            request_id,
        ),
        Err(error) => auth_error_response(error, request_id),
    }
}

fn validation_error(field: Option<&str>, request_id: RequestId) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "error": {
                "code": "VALIDATION_FAILED",
                "message": "Request validation failed",
                "requestId": request_id,
                "retryable": false,
                "details": { "field": field },
            }
        })),
    )
        .into_response()
}
