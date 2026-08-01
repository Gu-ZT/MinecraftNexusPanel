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
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
            .collect();
        let body = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(body).expect("HTTP response body is JSON")
        };

        Self {
            status,
            headers,
            body,
        }
    }
}
