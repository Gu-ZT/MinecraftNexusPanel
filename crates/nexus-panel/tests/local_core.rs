use std::net::SocketAddr;

use nexus_config::CoreConfig;
use nexus_config::InitialAdminConfig;
use nexus_config::LocalCoreConfig;
use nexus_config::PanelConfig;
use nexus_config::PanelMasterKey;
use nexus_core::CoreServer;
use nexus_domain::RequestId;
use nexus_panel::PanelError;
use nexus_panel::PanelServer;
use serde_json::Value;
use serde_json::from_str;
use serde_json::json;
use tempfile::TempDir;
use tempfile::tempdir;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::spawn;
use tokio::task::JoinHandle;

const ADMIN_PASSWORD: &str = "correct horse battery staple";
const CORE_PSK: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY";

#[tokio::test]
async fn registers_a_loopback_core_when_panel_starts() {
    let core_data = tempdir().expect("temporary Core data directory is created");
    let panel_data = tempdir().expect("temporary Panel data directory is created");
    let core_config = CoreConfig::new(
        "127.0.0.1:0".to_owned(),
        core_data.path().to_path_buf(),
        Some(CORE_PSK.to_owned()),
    )
    .expect("test Core configuration is valid");
    let core_server = CoreServer::bind(&core_config)
        .await
        .expect("Core listener binds");
    let local_core = LocalCoreConfig::new(
        core_server.core_id(),
        core_server.listen_address(),
        CORE_PSK.to_owned(),
    );
    let local_core_address = local_core.listen_address().to_string();
    let core_task = spawn(core_server.serve());
    let (panel_address, panel_task) = start_panel(&panel_data, local_core).await;
    let access_token = login(panel_address).await;
    let authorization = format!("Bearer {access_token}");

    let response = send_json_request(
        panel_address,
        "GET",
        "/api/v1/cores",
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;

    assert_eq!(response.status, 200);
    assert_eq!(response.body["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(response.body["items"][0]["name"], "Loopback Core");
    assert_eq!(response.body["items"][0]["status"], "ONLINE");
    assert_eq!(response.body["items"][0]["address"], local_core_address);
    assert_eq!(
        response.body["items"][0]["tags"],
        json!(["local", "loopback"])
    );
    assert!(response.body["items"][0].get("secret").is_none());

    stop_panel(panel_task).await;
    core_task.abort();
    let _ = core_task.await;
}

async fn start_panel(
    data_directory: &TempDir,
    local_core: LocalCoreConfig,
) -> (SocketAddr, JoinHandle<Result<(), PanelError>>) {
    let initial_admin = InitialAdminConfig::new("admin".to_owned(), ADMIN_PASSWORD.to_owned())
        .expect("initial administrator credentials are valid");
    let config = PanelConfig::new(
        "127.0.0.1:0".to_owned(),
        data_directory.path().to_path_buf(),
    )
    .expect("test Panel configuration is valid")
    .with_initial_admin(initial_admin)
    .with_local_core(local_core)
    .with_master_key(PanelMasterKey::from_bytes([29_u8; 32]));
    let server = PanelServer::bind(&config)
        .await
        .expect("Panel listener binds");
    let listen_address = server.listen_address();
    let server_task = spawn(server.serve());

    (listen_address, server_task)
}

async fn login(address: SocketAddr) -> String {
    let response = send_json_request(
        address,
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

    assert_eq!(response.status, 200);
    response.body["session"]["accessToken"]
        .as_str()
        .expect("native access token is returned")
        .to_owned()
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
        let body = if body.is_empty() {
            Value::Null
        } else {
            from_str(body).expect("HTTP response body is JSON")
        };

        Self { status, body }
    }
}
