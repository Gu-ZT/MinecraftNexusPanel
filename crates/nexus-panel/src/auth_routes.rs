//! 登录、刷新、登出和当前用户 HTTP 路由。
//!
//! 浏览器请求使用 HttpOnly 会话 Cookie 与 CSRF 头，原生请求使用访问令牌；写操作
//! 的鉴权和错误响应在路由边界统一处理。

use std::net::SocketAddr;

use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::ConnectInfo;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::header::COOKIE;
use axum::http::header::RETRY_AFTER;
use axum::http::header::SET_COOKIE;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use nexus_domain::RequestId;
use nexus_storage::StoredSession;
use serde_json::json;
use tokio::task::spawn_blocking;

use crate::AuthError;
use crate::AuthService;
use crate::IssuedSession;
use crate::LoginRequest;
use crate::LoginResponse;
use crate::PanelState;
use crate::RefreshRequest;
use crate::SessionResponse;
use crate::UserResponse;

const SESSION_COOKIE_NAME: &str = "mcnp_session";
const SESSION_COOKIE_MAX_AGE_SECONDS: i64 = 30 * 24 * 60 * 60;

pub(crate) fn auth_routes() -> Router<PanelState> {
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(current_user))
}

async fn login(
    State(state): State<PanelState>,
    ConnectInfo(source_address): ConnectInfo<SocketAddr>,
    Extension(request_id): Extension<RequestId>,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Response {
    let auth = state.auth().clone();
    let request = match payload {
        Ok(Json(request)) if request.is_valid() => request,
        Ok(_) | Err(_) => return validation_error(request_id),
    };
    let result = run_blocking(move || auth.login(&request, source_address.ip())).await;

    match result {
        Ok(session) => issued_session_response(&session, true),
        Err(error) => auth_error_response(error, request_id),
    }
}

async fn refresh(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    payload: Result<Option<Json<RefreshRequest>>, JsonRejection>,
) -> Response {
    let auth = state.auth().clone();
    let request = match payload {
        Ok(request) => request,
        Err(_) => return validation_error(request_id),
    };
    let csrf_token = header_text(&headers, "x-csrf-token").map(str::to_owned);
    let session_cookie = cookie_value(&headers, SESSION_COOKIE_NAME).map(str::to_owned);
    let result = if let Some(Json(request)) = request {
        let refresh_token = request.refresh_token().to_owned();
        run_blocking(move || auth.refresh_native(&refresh_token)).await
    } else if let (Some(session_cookie), Some(csrf_token)) = (session_cookie, csrf_token) {
        run_blocking(move || auth.refresh_browser(&session_cookie, &csrf_token)).await
    } else {
        Err(AuthError::InvalidSession)
    };

    match result {
        Ok(session) => issued_session_response(&session, false),
        Err(error) => auth_error_response(error, request_id),
    }
}

async fn logout(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Response {
    let auth = state.auth().clone();
    let credential = match request_credential(&headers) {
        Some(credential) => credential,
        None => return auth_error_response(AuthError::InvalidSession, request_id),
    };
    let csrf_token = header_text(&headers, "x-csrf-token").map(str::to_owned);
    let browser_session = matches!(&credential, RequestCredential::Browser(_));
    let result = run_blocking(move || {
        let session = authenticate(&auth, &credential)?;
        if matches!(&credential, RequestCredential::Browser(_)) {
            auth.verify_csrf(
                &session,
                csrf_token.as_deref().ok_or(AuthError::InvalidCsrfToken)?,
            )?;
        }
        auth.logout(session.id())
    })
    .await;

    match result {
        Ok(()) => {
            let mut response = StatusCode::NO_CONTENT.into_response();
            if browser_session {
                clear_session_cookie(&mut response);
            }
            response
        }
        Err(error) => auth_error_response(error, request_id),
    }
}

async fn current_user(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Response {
    let auth = state.auth().clone();
    let credential = match request_credential(&headers) {
        Some(credential) => credential,
        None => return auth_error_response(AuthError::InvalidSession, request_id),
    };
    let result = run_blocking(move || authenticate(&auth, &credential)).await;

    match result {
        Ok(session) => Json(UserResponse::from(session.user())).into_response(),
        Err(error) => auth_error_response(error, request_id),
    }
}

pub(crate) fn authenticate(
    auth: &AuthService,
    credential: &RequestCredential,
) -> Result<StoredSession, AuthError> {
    match credential {
        RequestCredential::Bearer(token) => auth.authenticate_access_token(token),
        RequestCredential::Browser(cookie) => auth.authenticate_browser_session(cookie),
    }
}

/// 验证请求会话，并在浏览器写操作中同时校验 CSRF。
///
/// 本函数只建立用户身份，不授予业务权限；具体路由必须随后检查管理员标记或
/// 显式权限，避免把“已登录”误当成“已授权”。
pub(crate) async fn authorize_session(
    state: &PanelState,
    headers: &HeaderMap,
    write: bool,
    request_id: RequestId,
) -> Result<StoredSession, Response> {
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

    Ok(session)
}

pub(crate) async fn run_blocking<T, F>(operation: F) -> Result<T, AuthError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AuthError> + Send + 'static,
{
    spawn_blocking(operation).await?
}

fn issued_session_response(session: &IssuedSession, include_user: bool) -> Response {
    let mut response = if include_user {
        Json(LoginResponse::from(session)).into_response()
    } else {
        Json(SessionResponse::from(session)).into_response()
    };
    if let Some(cookie) = session.browser_cookie() {
        set_session_cookie(&mut response, cookie);
    }

    response
}

fn set_session_cookie(response: &mut Response, value: &str) {
    let cookie = format!(
        "{SESSION_COOKIE_NAME}={value}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={SESSION_COOKIE_MAX_AGE_SECONDS}"
    );
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(SET_COOKIE, value);
    }
}

fn clear_session_cookie(response: &mut Response) {
    let cookie =
        format!("{SESSION_COOKIE_NAME}=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0");
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(SET_COOKIE, value);
    }
}

pub(crate) fn request_credential(headers: &HeaderMap) -> Option<RequestCredential> {
    if let Some(value) =
        header_text(headers, AUTHORIZATION.as_str()).and_then(|value| value.strip_prefix("Bearer "))
    {
        return Some(RequestCredential::Bearer(value.to_owned()));
    }

    cookie_value(headers, SESSION_COOKIE_NAME)
        .map(str::to_owned)
        .map(RequestCredential::Browser)
}

fn cookie_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    header_text(headers, COOKIE.as_str())?
        .split(';')
        .filter_map(|cookie| cookie.trim().split_once('='))
        .find_map(|(cookie_name, value)| (cookie_name == name).then_some(value))
}

pub(crate) fn header_text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

fn validation_error(request_id: RequestId) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "VALIDATION_FAILED",
        "Request validation failed",
        request_id,
    )
}

pub(crate) fn auth_error_response(error: AuthError, request_id: RequestId) -> Response {
    match error {
        AuthError::InvalidCredentials => error_response(
            StatusCode::UNAUTHORIZED,
            "AUTH_INVALID_CREDENTIALS",
            "Username or password is invalid",
            request_id,
        ),
        AuthError::InvalidSession => error_response(
            StatusCode::UNAUTHORIZED,
            "AUTH_SESSION_REVOKED",
            "Session is invalid or expired",
            request_id,
        ),
        AuthError::RefreshReused => error_response(
            StatusCode::UNAUTHORIZED,
            "AUTH_REFRESH_REUSED",
            "A rotated refresh credential was reused",
            request_id,
        ),
        AuthError::InvalidCsrfToken => error_response(
            StatusCode::FORBIDDEN,
            "CSRF_REJECTED",
            "CSRF token is invalid",
            request_id,
        ),
        AuthError::RateLimited {
            retry_after_seconds,
        } => {
            let mut response = error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                "Too many login attempts",
                request_id,
            );
            if let Ok(value) = HeaderValue::from_str(&retry_after_seconds.to_string()) {
                response.headers_mut().insert(RETRY_AFTER, value);
            }
            response
        }
        internal_error => {
            tracing::error!(error = %internal_error, %request_id, "Authentication request failed");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "The request could not be completed",
                request_id,
            )
        }
    }
}

pub(crate) fn error_response(
    status: StatusCode,
    code: &str,
    message: &str,
    request_id: RequestId,
) -> Response {
    (
        status,
        Json(json!({
            "error": {
                "code": code,
                "message": message,
                "requestId": request_id,
                "retryable": false,
            }
        })),
    )
        .into_response()
}

#[derive(Clone)]
pub(crate) enum RequestCredential {
    Bearer(String),
    Browser(String),
}
