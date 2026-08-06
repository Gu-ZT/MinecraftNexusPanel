//! Panel 用户创建、查询、更新和删除路由。
//!
//! 当前仅管理员可管理用户，并且只能创建非管理员账户；可授予权限集合由实际
//! 完成服务端检查的能力限定，避免先写入无法执行的授权。

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
use axum::routing::get;
use nexus_domain::RequestId;
use serde_json::json;

use crate::PanelState;
use crate::UserCreate;
use crate::UserResponse;
use crate::UserUpdate;
use crate::auth_routes::auth_error_response;
use crate::auth_routes::error_response;
use crate::auth_routes::run_blocking;
use crate::core_routes::authorize;

/// 注册用户列表和创建端点。
pub(crate) fn user_routes() -> Router<PanelState> {
    Router::new()
        .route("/api/v1/users", get(list_users).post(create_user))
        .route(
            "/api/v1/users/{user_id}",
            get(get_user).patch(update_user).delete(delete_user),
        )
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

async fn get_user(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    if !valid_user_id(&user_id) {
        return validation_error(Some("userId"), request_id);
    }

    let auth = state.auth().clone();
    match run_blocking(move || auth.find_user(&user_id)).await {
        Ok(Some(user)) => Json(UserResponse::from(&user)).into_response(),
        Ok(None) => user_not_found(request_id),
        Err(error) => auth_error_response(error, request_id),
    }
}

async fn update_user(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<UserUpdate>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    if !valid_user_id(&user_id) {
        return validation_error(Some("userId"), request_id);
    }
    let request = match payload {
        Ok(Json(request)) if request.invalid_field().is_none() => request,
        Ok(Json(request)) => return validation_error(request.invalid_field(), request_id),
        Err(_) => return validation_error(None, request_id),
    };

    let auth = state.auth().clone();
    let lookup_auth = auth.clone();
    let lookup_user_id = user_id.clone();
    match run_blocking(move || lookup_auth.find_user(&lookup_user_id)).await {
        Ok(Some(user)) if user.is_admin() => return protected_administrator(request_id),
        Ok(Some(_)) => {}
        Ok(None) => return user_not_found(request_id),
        Err(error) => return auth_error_response(error, request_id),
    }

    match run_blocking(move || auth.update_user(&user_id, &request)).await {
        Ok(Some(user)) => Json(UserResponse::from(&user)).into_response(),
        Ok(None) => user_not_found(request_id),
        Err(error) => auth_error_response(error, request_id),
    }
}

async fn delete_user(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let session = match authorize(&state, &headers, true, request_id).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    if !valid_user_id(&user_id) {
        return validation_error(Some("userId"), request_id);
    }
    if session.user().id() == user_id {
        return error_response(
            StatusCode::CONFLICT,
            "USER_SELF_DELETE_FORBIDDEN",
            "The active administrator cannot delete its own account",
            request_id,
        );
    }

    let auth = state.auth().clone();
    let lookup_auth = auth.clone();
    let lookup_user_id = user_id.clone();
    match run_blocking(move || lookup_auth.find_user(&lookup_user_id)).await {
        Ok(Some(user)) if user.is_admin() => return protected_administrator(request_id),
        Ok(Some(_)) => {}
        Ok(None) => return user_not_found(request_id),
        Err(error) => return auth_error_response(error, request_id),
    }

    match run_blocking(move || auth.delete_user(&user_id)).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => user_not_found(request_id),
        Err(error) => auth_error_response(error, request_id),
    }
}

fn valid_user_id(user_id: &str) -> bool {
    !user_id.is_empty() && user_id.len() <= 128 && !user_id.contains('\0')
}

fn user_not_found(request_id: RequestId) -> Response {
    error_response(
        StatusCode::NOT_FOUND,
        "USER_NOT_FOUND",
        "The requested user does not exist",
        request_id,
    )
}

fn protected_administrator(request_id: RequestId) -> Response {
    error_response(
        StatusCode::CONFLICT,
        "USER_ADMIN_PROTECTED",
        "Administrator accounts cannot be modified or deleted",
        request_id,
    )
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
