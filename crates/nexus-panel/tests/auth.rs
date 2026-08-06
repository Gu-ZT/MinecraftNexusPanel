use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;

use nexus_config::InitialAdminConfig;
use nexus_config::PanelConfig;
use nexus_config::PanelMasterKey;
use nexus_domain::RequestId;
use nexus_panel::PanelError;
use nexus_panel::PanelServer;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;
use tempfile::tempdir;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

const ADMIN_PASSWORD: &str = "correct horse battery staple";

#[tokio::test]
async fn native_session_rotates_credentials_and_rejects_reuse() {
    let data_directory = tempdir().expect("temporary Panel data directory is created");
    let (listen_address, server_task) = start_panel(&data_directory, "admin", ADMIN_PASSWORD).await;

    let login_response = send_json_request(
        listen_address,
        "POST",
        "/api/v1/auth/login",
        &[],
        Some(json!({
            "username": "admin",
            "password": ADMIN_PASSWORD,
            "clientType": "NATIVE",
        })),
    )
    .await;
    assert_eq!(login_response.status, 200);
    let access_token = login_response.body["session"]["accessToken"]
        .as_str()
        .expect("native access token is returned")
        .to_owned();
    let refresh_token = login_response.body["session"]["refreshToken"]
        .as_str()
        .expect("native refresh token is returned")
        .to_owned();

    let current_user = send_json_request(
        listen_address,
        "GET",
        "/api/v1/auth/me",
        &[("Authorization", &format!("Bearer {access_token}"))],
        None,
    )
    .await;
    assert_eq!(current_user.status, 200);
    assert_eq!(current_user.body["username"], "admin");

    let refresh_response = send_json_request(
        listen_address,
        "POST",
        "/api/v1/auth/refresh",
        &[],
        Some(json!({ "refreshToken": refresh_token })),
    )
    .await;
    assert_eq!(refresh_response.status, 200);
    let replacement_access_token = refresh_response.body["accessToken"]
        .as_str()
        .expect("replacement access token is returned")
        .to_owned();

    let reused_refresh = send_json_request(
        listen_address,
        "POST",
        "/api/v1/auth/refresh",
        &[],
        Some(json!({ "refreshToken": refresh_token })),
    )
    .await;
    assert_eq!(reused_refresh.status, 401);
    assert_eq!(reused_refresh.body["error"]["code"], "AUTH_REFRESH_REUSED");

    let revoked_session = send_json_request(
        listen_address,
        "GET",
        "/api/v1/auth/me",
        &[(
            "Authorization",
            &format!("Bearer {replacement_access_token}"),
        )],
        None,
    )
    .await;
    assert_eq!(revoked_session.status, 401);

    stop_panel(server_task).await;
}

#[tokio::test]
async fn browser_session_requires_csrf_for_logout() {
    let data_directory = tempdir().expect("temporary Panel data directory is created");
    let (listen_address, server_task) = start_panel(&data_directory, "admin", ADMIN_PASSWORD).await;

    let login_response = send_json_request(
        listen_address,
        "POST",
        "/api/v1/auth/login",
        &[],
        Some(json!({
            "username": "admin",
            "password": ADMIN_PASSWORD,
            "clientType": "BROWSER",
        })),
    )
    .await;
    assert_eq!(login_response.status, 200);
    let set_cookie = login_response
        .headers
        .get("set-cookie")
        .expect("browser session cookie is returned");
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("Secure"));
    assert!(set_cookie.contains("SameSite=Lax"));
    let cookie = set_cookie
        .split(';')
        .next()
        .expect("session cookie has a value")
        .to_owned();
    let csrf_token = login_response.body["session"]["csrfToken"]
        .as_str()
        .expect("browser CSRF token is returned")
        .to_owned();
    assert!(login_response.body["session"]["accessToken"].is_null());
    assert!(login_response.body["session"]["refreshToken"].is_null());

    let rejected_logout = send_json_request(
        listen_address,
        "POST",
        "/api/v1/auth/logout",
        &[("Cookie", &cookie)],
        None,
    )
    .await;
    assert_eq!(rejected_logout.status, 403);
    assert_eq!(rejected_logout.body["error"]["code"], "CSRF_REJECTED");

    let logout = send_json_request(
        listen_address,
        "POST",
        "/api/v1/auth/logout",
        &[("Cookie", &cookie), ("X-CSRF-Token", &csrf_token)],
        None,
    )
    .await;
    assert_eq!(logout.status, 204);
    assert!(
        logout
            .headers
            .get("set-cookie")
            .is_some_and(|value| value.contains("Max-Age=0"))
    );

    let revoked_session = send_json_request(
        listen_address,
        "GET",
        "/api/v1/auth/me",
        &[("Cookie", &cookie)],
        None,
    )
    .await;
    assert_eq!(revoked_session.status, 401);

    stop_panel(server_task).await;
}

#[tokio::test]
async fn persists_authenticated_request_audit_with_connection_context() {
    let data_directory = tempdir().expect("temporary Panel data directory is created");
    let (listen_address, server_task) = start_panel(&data_directory, "admin", ADMIN_PASSWORD).await;

    let login_response = login(listen_address, "admin", ADMIN_PASSWORD).await;
    assert_eq!(login_response.status, 200);
    let access_token = login_response.body["session"]["accessToken"]
        .as_str()
        .expect("native access token is returned")
        .to_owned();
    let user_id = login_response.body["user"]["id"]
        .as_str()
        .expect("user ID is returned")
        .to_owned();

    let current_user = send_json_request(
        listen_address,
        "GET",
        "/api/v1/auth/me",
        &[("Authorization", &format!("Bearer {access_token}"))],
        None,
    )
    .await;
    assert_eq!(current_user.status, 200);

    let audit = send_json_request(
        listen_address,
        "GET",
        "/api/v1/audit-events?limit=50",
        &[("Authorization", &format!("Bearer {access_token}"))],
        None,
    )
    .await;
    assert_eq!(audit.status, 200);
    let current_user_event = audit.body["items"]
        .as_array()
        .expect("audit items are returned")
        .iter()
        .find(|event| event["path"] == "/api/v1/auth/me")
        .expect("authenticated user request is audited");
    assert_eq!(current_user_event["userId"], user_id);
    assert!(current_user_event["requestId"].is_string());
    assert_eq!(current_user_event["sourceIp"], "127.0.0.1");
    assert_eq!(current_user_event["method"], "GET");
    assert_eq!(current_user_event["statusCode"], 200);
    assert_eq!(current_user_event["permissionResult"], "ALLOWED");

    stop_panel(server_task).await;
}

#[tokio::test]
async fn grants_audit_read_without_granting_administrator_access() {
    let data_directory = tempdir().expect("temporary Panel data directory is created");
    let (listen_address, server_task) = start_panel(&data_directory, "admin", ADMIN_PASSWORD).await;
    let admin_login = login(listen_address, "admin", ADMIN_PASSWORD).await;
    let admin_token = admin_login.body["session"]["accessToken"]
        .as_str()
        .expect("administrator access token is returned");
    let admin_id = admin_login.body["user"]["id"]
        .as_str()
        .expect("administrator user ID is returned")
        .to_owned();
    let admin_authorization = format!("Bearer {admin_token}");

    let unsupported_permission = send_json_request(
        listen_address,
        "POST",
        "/api/v1/users",
        &[("Authorization", admin_authorization.as_str())],
        Some(json!({
            "username": "unsafe-grant",
            "displayName": "Unsupported Grant",
            "password": "unsupported secure password",
            "permissions": ["core.manage"],
        })),
    )
    .await;
    assert_eq!(unsupported_permission.status, 400);
    assert_eq!(
        unsupported_permission.body["error"]["details"]["field"],
        "permissions"
    );

    let audit_reader = send_json_request(
        listen_address,
        "POST",
        "/api/v1/users",
        &[("Authorization", admin_authorization.as_str())],
        Some(json!({
            "username": "auditor",
            "displayName": "Audit Reader",
            "password": "auditor secure password",
            "permissions": ["audit.read", "audit.read"],
        })),
    )
    .await;
    assert_eq!(audit_reader.status, 201);
    assert_eq!(audit_reader.body["permissions"], json!(["audit.read"]));
    let audit_reader_id = audit_reader.body["id"]
        .as_str()
        .expect("audit reader ID is returned")
        .to_owned();

    let ordinary_user = send_json_request(
        listen_address,
        "POST",
        "/api/v1/users",
        &[("Authorization", admin_authorization.as_str())],
        Some(json!({
            "username": "operator",
            "displayName": "Server Operator",
            "password": "operator secure password",
            "permissions": [],
        })),
    )
    .await;
    assert_eq!(ordinary_user.status, 201);
    let ordinary_user_id = ordinary_user.body["id"]
        .as_str()
        .expect("ordinary user ID is returned")
        .to_owned();

    let users = send_json_request(
        listen_address,
        "GET",
        "/api/v1/users",
        &[("Authorization", admin_authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(users.status, 200);
    assert_eq!(users.body["items"].as_array().map(Vec::len), Some(3));

    let listed_reader = send_json_request(
        listen_address,
        "GET",
        &format!("/api/v1/users/{audit_reader_id}"),
        &[("Authorization", admin_authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(listed_reader.status, 200);
    assert_eq!(listed_reader.body["username"], "auditor");

    let reader_login = login(listen_address, "auditor", "auditor secure password").await;
    let reader_token = reader_login.body["session"]["accessToken"]
        .as_str()
        .expect("audit reader access token is returned");
    let reader_authorization = format!("Bearer {reader_token}");
    let reader_audit = send_json_request(
        listen_address,
        "GET",
        "/api/v1/audit-events",
        &[("Authorization", reader_authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(reader_audit.status, 200);

    let audit_export = send_json_request(
        listen_address,
        "GET",
        "/api/v1/audit-events:export",
        &[("Authorization", reader_authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(audit_export.status, 200);
    assert!(
        audit_export
            .headers
            .get("content-type")
            .is_some_and(|value| value.starts_with("application/x-ndjson"))
    );
    assert!(
        audit_export
            .headers
            .get("content-disposition")
            .is_some_and(|value| value.contains("mcnp-audit-events.ndjson"))
    );
    let exported_events = audit_export
        .body_text
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("NDJSON line is valid JSON"))
        .collect::<Vec<_>>();
    assert!(!exported_events.is_empty());
    assert!(
        exported_events
            .iter()
            .all(|event| event.get("requestId").is_some()
                && event.get("path").is_some()
                && event.get("body").is_none()
                && event.get("query").is_none()
                && event.get("authorization").is_none())
    );

    let updated_reader = send_json_request(
        listen_address,
        "PATCH",
        &format!("/api/v1/users/{audit_reader_id}"),
        &[("Authorization", admin_authorization.as_str())],
        Some(json!({
            "displayName": "Former Audit Reader",
            "permissions": [],
        })),
    )
    .await;
    assert_eq!(updated_reader.status, 200);
    assert_eq!(updated_reader.body["displayName"], "Former Audit Reader");
    assert_eq!(updated_reader.body["permissions"], json!([]));

    let revoked_reader_audit = send_json_request(
        listen_address,
        "GET",
        "/api/v1/audit-events",
        &[("Authorization", reader_authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(revoked_reader_audit.status, 403);

    let rejected_user_management = send_json_request(
        listen_address,
        "GET",
        "/api/v1/users",
        &[("Authorization", reader_authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(rejected_user_management.status, 403);

    let operator_login = login(listen_address, "operator", "operator secure password").await;
    let operator_token = operator_login.body["session"]["accessToken"]
        .as_str()
        .expect("ordinary user access token is returned");
    let operator_authorization = format!("Bearer {operator_token}");
    let rejected_audit = send_json_request(
        listen_address,
        "GET",
        "/api/v1/audit-events",
        &[("Authorization", operator_authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(rejected_audit.status, 403);
    assert_eq!(rejected_audit.body["error"]["code"], "FORBIDDEN");

    let deleted_user = send_json_request(
        listen_address,
        "DELETE",
        &format!("/api/v1/users/{ordinary_user_id}"),
        &[("Authorization", admin_authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(deleted_user.status, 204);
    let deleted_user_session = send_json_request(
        listen_address,
        "GET",
        "/api/v1/auth/me",
        &[("Authorization", operator_authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(deleted_user_session.status, 401);

    let protected_admin = send_json_request(
        listen_address,
        "DELETE",
        &format!("/api/v1/users/{admin_id}"),
        &[("Authorization", admin_authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(protected_admin.status, 409);
    assert_eq!(
        protected_admin.body["error"]["code"],
        "USER_SELF_DELETE_FORBIDDEN"
    );

    stop_panel(server_task).await;
}

#[tokio::test]
async fn initial_administrator_is_not_replaced_on_restart() {
    let data_directory = tempdir().expect("temporary Panel data directory is created");
    let (first_address, first_task) =
        start_panel(&data_directory, "first-admin", ADMIN_PASSWORD).await;
    let first_login = login(first_address, "first-admin", ADMIN_PASSWORD).await;
    assert_eq!(first_login.status, 200);
    stop_panel(first_task).await;

    let (second_address, second_task) = start_panel(
        &data_directory,
        "replacement-admin",
        "a different secure password",
    )
    .await;
    let retained_login = login(second_address, "first-admin", ADMIN_PASSWORD).await;
    assert_eq!(retained_login.status, 200);
    let rejected_replacement = login(
        second_address,
        "replacement-admin",
        "a different secure password",
    )
    .await;
    assert_eq!(rejected_replacement.status, 401);
    assert_eq!(
        rejected_replacement.body["error"]["code"],
        "AUTH_INVALID_CREDENTIALS"
    );
    assert!(rejected_replacement.body["error"]["requestId"].is_string());

    stop_panel(second_task).await;
}

#[tokio::test]
async fn repeated_login_failures_are_rate_limited() {
    let data_directory = tempdir().expect("temporary Panel data directory is created");
    let (listen_address, server_task) = start_panel(&data_directory, "admin", ADMIN_PASSWORD).await;

    for _ in 0..5 {
        let response = login(listen_address, "admin", "incorrect password").await;
        assert_eq!(response.status, 401);
    }
    let limited_response = login(listen_address, "admin", ADMIN_PASSWORD).await;
    assert_eq!(limited_response.status, 429);
    assert_eq!(limited_response.body["error"]["code"], "RATE_LIMITED");
    assert!(
        limited_response
            .headers
            .get("retry-after")
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|seconds| seconds > 0)
    );

    stop_panel(server_task).await;
}

async fn login(address: SocketAddr, username: &str, password: &str) -> TestHttpResponse {
    send_json_request(
        address,
        "POST",
        "/api/v1/auth/login",
        &[],
        Some(json!({
            "username": username,
            "password": password,
            "clientType": "NATIVE",
        })),
    )
    .await
}

async fn start_panel(
    data_directory: &TempDir,
    username: &str,
    password: &str,
) -> (SocketAddr, JoinHandle<Result<(), PanelError>>) {
    let config = panel_config(data_directory.path(), username, password);
    let server = PanelServer::bind(&config)
        .await
        .expect("Panel listener binds");
    let listen_address = server.listen_address();
    let server_task = tokio::spawn(server.serve());

    (listen_address, server_task)
}

fn panel_config(data_directory: &Path, username: &str, password: &str) -> PanelConfig {
    let initial_admin = InitialAdminConfig::new(username.to_owned(), password.to_owned())
        .expect("initial administrator credentials are valid");
    PanelConfig::new("127.0.0.1:0".to_owned(), data_directory.to_path_buf())
        .expect("test Panel configuration is valid")
        .with_initial_admin(initial_admin)
        .with_master_key(PanelMasterKey::from_bytes([11_u8; 32]))
}

async fn stop_panel(server_task: JoinHandle<Result<(), PanelError>>) {
    server_task.abort();
    let _ = server_task.await;
}

async fn send_json_request(
    address: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<Value>,
) -> TestHttpResponse {
    let body = body.map_or_else(String::new, |value| value.to_string());
    let request_id = RequestId::new();
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nX-Request-Id: {request_id}\r\nConnection: close\r\n"
    );
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    if !body.is_empty() {
        request.push_str("Content-Type: application/json\r\n");
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("\r\n");
    request.push_str(&body);

    let mut stream = TcpStream::connect(address)
        .await
        .expect("HTTP client connects");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("HTTP request is sent");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .await
        .expect("HTTP response is read");

    TestHttpResponse::parse(&response)
}

struct TestHttpResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: Value,
    body_text: String,
}

impl TestHttpResponse {
    fn parse(response: &str) -> Self {
        let (head, body) = response
            .split_once("\r\n\r\n")
            .expect("HTTP response has a header boundary");
        let mut lines = head.lines();
        let status = lines
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|status| status.parse().ok())
            .expect("HTTP response has a numeric status");
        let headers: HashMap<String, String> = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
            .collect();
        let body_text = body.to_owned();
        let body = if body.is_empty() {
            Value::Null
        } else if headers
            .get("content-type")
            .is_some_and(|value| value.starts_with("application/json"))
        {
            serde_json::from_str(body).expect("HTTP response body is JSON")
        } else {
            Value::Null
        };

        Self {
            status,
            headers,
            body,
            body_text,
        }
    }
}
