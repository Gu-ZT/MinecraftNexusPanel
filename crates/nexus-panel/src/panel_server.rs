use std::net::SocketAddr;

use axum::Json;
use axum::Router;
use axum::extract::ConnectInfo;
use axum::extract::Request;
use axum::extract::State;
use axum::http::HeaderName;
use axum::http::HeaderValue;
use axum::http::Method;
use axum::http::StatusCode;
use axum::middleware;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use nexus_config::PanelConfig;
use nexus_domain::RequestId;
use nexus_storage::NewAuditEvent;
use nexus_storage::SqliteStore;
use serde_json::Value;
use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::net::TcpListener;
use tokio::task::spawn_blocking;
use uuid::Uuid;

use crate::AuthService;
use crate::CoreRegistry;
use crate::PanelError;
use crate::PanelState;
use crate::SecretCipher;
use crate::VersionMetadataClient;
use crate::audit_routes::audit_routes;
use crate::auth_routes::auth_routes;
use crate::auth_routes::authenticate;
use crate::auth_routes::request_credential;
use crate::auth_routes::run_blocking;
use crate::bedrock_routes::bedrock_routes;
use crate::config_routes::config_routes;
use crate::core_routes::core_routes;
use crate::environment_routes::environment_routes;
use crate::extension_routes::extension_routes;
use crate::extension_source_client::ExtensionSourceClient;
use crate::file_routes::file_routes;
use crate::install_template_routes::install_template_routes;
use crate::instance_routes::instance_routes;
use crate::provision_routes::provision_routes;
use crate::proxy_routes::proxy_routes;
use crate::user_routes::user_routes;
use crate::websocket_routes::websocket_routes;

/// Panel HTTP/TCP 服务入口。
///
/// 绑定时初始化 SQLite、管理员、Core 注册、扩展来源和版本元数据服务；路由共享
/// 同一个 Panel 状态集合，请求 ID 中间件会为每个请求建立可追踪关联。
pub struct PanelServer {
    listen_address: SocketAddr,
    listener: TcpListener,
    state: PanelState,
}

impl PanelServer {
    /// 根据 Panel 配置初始化依赖并绑定 HTTP 监听器。
    pub async fn bind(config: &PanelConfig) -> Result<Self, PanelError> {
        let master_key = config
            .master_key()
            .ok_or(PanelError::MissingPanelMasterKey)?;
        let store = SqliteStore::open_with_audit_retention(
            config.data_directory(),
            config.audit_retention_events(),
        )?;
        let panel_id = store.get_or_create_panel_id(&Uuid::now_v7().to_string())?;
        let auth = AuthService::new(store.clone());
        if let Some(initial_admin) = config.initial_admin() {
            if auth.initialize_admin(initial_admin)? {
                tracing::info!(
                    username = initial_admin.username(),
                    "Initial administrator created"
                );
            }
        }
        if !auth.has_users()? {
            tracing::warn!(
                "Panel has no users; configure MCNP_INITIAL_ADMIN_USERNAME and MCNP_INITIAL_ADMIN_PASSWORD"
            );
        }
        let cores = CoreRegistry::new(store.clone(), SecretCipher::new(master_key), panel_id)?;
        let extension_sources = ExtensionSourceClient::new()?;
        let version_metadata = VersionMetadataClient::new()?;
        if let Some(local_core) = config.local_core() {
            cores.ensure_local_core(local_core).await?;
        }
        let listener = TcpListener::bind(config.listen_address())
            .await
            .map_err(|source| PanelError::Bind {
                address: config.listen_address(),
                source,
            })?;
        let listen_address = listener.local_addr().map_err(|source| PanelError::Bind {
            address: config.listen_address(),
            source,
        })?;

        Ok(Self {
            listen_address,
            listener,
            state: PanelState::new(
                auth,
                config.desktop_session().cloned(),
                cores,
                store,
                extension_sources,
                version_metadata,
            ),
        })
    }

    /// 返回实际绑定的 HTTP 地址。
    #[must_use]
    pub const fn listen_address(&self) -> SocketAddr {
        self.listen_address
    }

    /// 启动 HTTP 服务并持续处理请求。
    pub async fn serve(self) -> Result<(), PanelError> {
        tracing::info!(
            listen_address = %self.listen_address,
            "Panel HTTP listener is ready"
        );

        axum::serve(
            self.listener,
            router(self.state).into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .map_err(PanelError::Serve)
    }
}

fn router(state: PanelState) -> Router {
    let audit_state = state.clone();
    let desktop_cors_state = state.clone();

    Router::new()
        .route("/api/v1/health/live", get(health))
        .route("/api/v1/health/ready", get(readiness))
        .merge(audit_routes())
        .merge(auth_routes())
        .merge(user_routes())
        .merge(core_routes())
        .merge(bedrock_routes())
        .merge(config_routes())
        .merge(environment_routes())
        .merge(extension_routes())
        .merge(file_routes())
        .merge(instance_routes())
        .merge(proxy_routes())
        .merge(provision_routes())
        .merge(install_template_routes())
        .merge(websocket_routes())
        .with_state(state)
        .layer(middleware::from_fn_with_state(audit_state, audit_request))
        .layer(middleware::from_fn(assign_request_id))
        .layer(middleware::from_fn_with_state(
            desktop_cors_state,
            desktop_cors,
        ))
}

/// 为 Tauri WebView 开放受限的本地跨源请求。
///
/// Panel 默认仍不向任意来源开放 CORS；只接受 Tauri 本地协议映射的来源，避免把
/// 管理 API 的跨源访问范围扩展到局域网或公网。开发服务器来源仅在 sidecar 已配置
/// Desktop 设备会话时开放。原生 Desktop 使用 Bearer 令牌，因此不依赖跨源 Cookie。
async fn desktop_cors(State(state): State<PanelState>, request: Request, next: Next) -> Response {
    let origin = request
        .headers()
        .get("origin")
        .and_then(|value| value.to_str().ok())
        .filter(|origin| is_desktop_origin(origin, state.desktop_session().is_some()))
        .and_then(|origin| HeaderValue::from_str(origin).ok());

    if request.method() == Method::OPTIONS {
        if let Some(origin) = origin {
            let mut response = StatusCode::NO_CONTENT.into_response();
            add_desktop_cors_headers(&mut response, origin);
            return response;
        }
        return next.run(request).await;
    }

    let mut response = next.run(request).await;
    if let Some(origin) = origin {
        add_desktop_cors_headers(&mut response, origin);
    }
    response
}

fn is_desktop_origin(origin: &str, desktop_session_enabled: bool) -> bool {
    matches!(
        origin,
        "http://tauri.localhost"
            | "https://tauri.localhost"
            | "http://asset.localhost"
            | "https://asset.localhost"
    ) || desktop_session_enabled && origin == "http://127.0.0.1:1420"
}

fn add_desktop_cors_headers(response: &mut Response, origin: HeaderValue) {
    let headers = response.headers_mut();
    headers.insert(
        HeaderName::from_static("access-control-allow-origin"),
        origin,
    );
    headers.insert(
        HeaderName::from_static("access-control-allow-credentials"),
        HeaderValue::from_static("true"),
    );
    headers.insert(
        HeaderName::from_static("access-control-allow-methods"),
        HeaderValue::from_static("GET, POST, PUT, PATCH, DELETE, OPTIONS"),
    );
    headers.insert(
        HeaderName::from_static("access-control-allow-headers"),
        HeaderValue::from_static(
            "Accept, Authorization, Content-Type, X-CSRF-Token, Idempotency-Key, If-Match, Content-SHA256",
        ),
    );
    headers.insert(
        HeaderName::from_static("access-control-expose-headers"),
        HeaderValue::from_static(
            "ETag, Content-SHA256, X-MCNP-File-Transfer-Offset, X-MCNP-File-Transfer-Next-Offset, X-MCNP-File-Transfer-Size, X-MCNP-File-EOF",
        ),
    );
    headers.insert(
        HeaderName::from_static("vary"),
        HeaderValue::from_static("Origin"),
    );
}

/// 在请求完成后写入不含敏感请求体的 Panel 审计事件。
///
/// 审计失败只记录内部日志，不改变已经生成的业务响应；这样数据库短暂不可写时
/// 不会把正常的 Core 控制请求误报为失败。请求路径不包含查询参数，令牌和密码不会
/// 进入审计库。
async fn audit_request(State(state): State<PanelState>, request: Request, next: Next) -> Response {
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .copied()
        .unwrap_or_default();
    let method = request.method().to_string();
    let path = request.uri().path().to_owned();
    let source_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip().to_string());
    let public_path = is_public_path(&path);
    let user_id = authenticated_user_id(&state, request.headers()).await;

    let response = next.run(request).await;
    let permission_result = permission_result(response.status(), user_id.is_some(), public_path);
    let event = NewAuditEvent {
        id: Uuid::now_v7().to_string(),
        occurred_at: current_timestamp(),
        user_id,
        request_id: request_id.to_string(),
        source_ip,
        method,
        path,
        status_code: response.status().as_u16(),
        permission_result: permission_result.to_owned(),
    };
    let store = state.store().clone();
    match spawn_blocking(move || store.append_audit_event(&event)).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            tracing::error!(%error, %request_id, "Unable to persist Panel audit event")
        }
        Err(error) => tracing::error!(%error, %request_id, "Panel audit worker failed"),
    }

    response
}

/// 尽力解析当前请求的用户；认证失败时返回空而不影响原请求继续由正式鉴权处理。
async fn authenticated_user_id(
    state: &PanelState,
    headers: &axum::http::HeaderMap,
) -> Option<String> {
    let credential = request_credential(headers)?;
    let auth = state.auth().clone();
    run_blocking(move || authenticate(&auth, &credential))
        .await
        .ok()
        .map(|session| session.user().id().to_owned())
}

fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        "/api/v1/health/live"
            | "/api/v1/health/ready"
            | "/api/v1/auth/login"
            | "/api/v1/auth/refresh"
            | "/api/v1/auth/desktop-session"
    )
}

fn permission_result(status: StatusCode, has_user: bool, public_path: bool) -> &'static str {
    if public_path {
        return "NOT_REQUIRED";
    }
    if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        return "DENIED";
    }
    if has_user { "ALLOWED" } else { "DENIED" }
}

async fn assign_request_id(mut request: Request, next: Next) -> Response {
    let request_id: RequestId = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .unwrap_or_default();
    request.extensions_mut().insert(request_id);

    let mut response = next.run(request).await;
    if let Ok(header_value) = HeaderValue::from_str(&request_id.to_string()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-request-id"), header_value);
    }

    response
}

async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "time": current_timestamp(),
    }))
}

async fn readiness(State(state): State<PanelState>) -> Response {
    match state.auth().has_users() {
        Ok(true) => health().await.into_response(),
        Ok(false) | Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "time": current_timestamp(),
            })),
        )
            .into_response(),
    }
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(test)]
mod tests {
    use super::is_desktop_origin;

    #[test]
    fn limits_packaged_desktop_cors_to_tauri_local_origins() {
        for origin in [
            "http://tauri.localhost",
            "https://tauri.localhost",
            "http://asset.localhost",
            "https://asset.localhost",
        ] {
            assert!(is_desktop_origin(origin, false));
        }

        for origin in [
            "null",
            "http://localhost",
            "http://127.0.0.1:1420",
            "http://tauri.localhost.example.com",
            "https://example.com",
        ] {
            assert!(!is_desktop_origin(origin, false));
        }
    }

    #[test]
    fn allows_the_vite_origin_only_for_desktop_sidecars() {
        assert!(is_desktop_origin("http://127.0.0.1:1420", true));
        assert!(!is_desktop_origin("http://127.0.0.1:1421", true));
    }
}
