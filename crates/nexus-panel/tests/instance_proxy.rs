use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use nexus_config::CoreConfig;
use nexus_config::InitialAdminConfig;
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
use tokio::time::sleep;
use tokio::time::timeout;

const ADMIN_PASSWORD: &str = "correct horse battery staple";
const CORE_PSK: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY";

#[tokio::test]
async fn proxies_instance_lifecycle_requests_to_a_registered_core() {
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
    let core_address = core_server.listen_address();
    let core_task = spawn(core_server.serve());
    let (panel_address, panel_task) = start_panel(&panel_data).await;
    let access_token = login(panel_address).await;
    let authorization = format!("Bearer {access_token}");
    let core_id = register_core(panel_address, &authorization, core_address).await;

    let created = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/instances"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        Some(safe_process_create("panel-process")),
    )
    .await;
    assert_eq!(created.status, 201);
    assert_eq!(created.body["coreId"], core_id);
    assert_eq!(created.body["runtime"]["state"], "CREATED");
    assert_eq!(
        created.headers.get("etag").map(String::as_str),
        Some("\"1\"")
    );

    let duplicate = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/instances"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        Some(safe_process_create("panel-process")),
    )
    .await;
    assert_eq!(duplicate.status, 409);
    assert_eq!(duplicate.body["error"]["code"], "INSTANCE_ALREADY_EXISTS");

    let listed = send_json_request(
        panel_address,
        "GET",
        &format!("/api/v1/cores/{core_id}/instances?limit=10&state=CREATED"),
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(listed.status, 200);
    assert_eq!(listed.body["items"][0]["id"], "panel-process");
    assert_eq!(listed.body["items"][0]["coreId"], core_id);

    let missing_key = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/instances/panel-process/actions/start"),
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(missing_key.status, 428);

    let started = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/instances/panel-process/actions/start"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        None,
    )
    .await;
    assert_eq!(started.status, 202);
    assert!(started.body["taskId"].is_string());
    wait_for_instance_state(
        panel_address,
        &authorization,
        &core_id,
        "panel-process",
        "RUNNING",
    )
    .await;

    let command = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/instances/panel-process/commands"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        Some(json!({ "command": "say hello" })),
    )
    .await;
    assert_eq!(command.status, 202);
    assert!(
        command.body["acceptedAt"]
            .as_str()
            .is_some_and(|value| value.ends_with('Z'))
    );

    wait_for_log_line(
        panel_address,
        &authorization,
        &core_id,
        "panel-process",
        "received:say hello",
    )
    .await;

    let metrics = send_json_request(
        panel_address,
        "GET",
        &format!("/api/v1/cores/{core_id}/instances/panel-process/metrics?range=current"),
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(metrics.status, 200);
    assert_eq!(metrics.body["series"].as_array().map(Vec::len), Some(1));

    let stopped = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/instances/panel-process/actions/stop"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        Some(json!({ "timeoutSeconds": 5 })),
    )
    .await;
    assert_eq!(stopped.status, 202);
    wait_for_instance_state(
        panel_address,
        &authorization,
        &core_id,
        "panel-process",
        "STOPPED",
    )
    .await;

    core_task.abort();
    let _ = core_task.await;
    stop_panel(panel_task).await;
}

async fn start_panel(data_directory: &TempDir) -> (SocketAddr, JoinHandle<Result<(), PanelError>>) {
    let initial_admin = InitialAdminConfig::new("admin".to_owned(), ADMIN_PASSWORD.to_owned())
        .expect("initial administrator credentials are valid");
    let config = PanelConfig::new(
        "127.0.0.1:0".to_owned(),
        data_directory.path().to_path_buf(),
    )
    .expect("test Panel configuration is valid")
    .with_initial_admin(initial_admin)
    .with_master_key(PanelMasterKey::from_bytes([23_u8; 32]));
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

async fn register_core(
    panel_address: SocketAddr,
    authorization: &str,
    core_address: SocketAddr,
) -> String {
    let response = send_json_request(
        panel_address,
        "POST",
        "/api/v1/cores",
        &[
            ("Authorization", authorization),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        Some(json!({
            "name": "Game Node",
            "address": core_address.to_string(),
            "secret": CORE_PSK,
            "connectTimeoutSeconds": 3,
        })),
    )
    .await;

    assert_eq!(response.status, 201);
    response.body["id"]
        .as_str()
        .expect("registered Core ID is returned")
        .to_owned()
}

async fn wait_for_instance_state(
    address: SocketAddr,
    authorization: &str,
    core_id: &str,
    instance_id: &str,
    expected_state: &str,
) {
    timeout(Duration::from_secs(5), async {
        loop {
            let response = send_json_request(
                address,
                "GET",
                &format!("/api/v1/cores/{core_id}/instances/{instance_id}"),
                &[("Authorization", authorization)],
                None,
            )
            .await;
            assert_eq!(response.status, 200);
            if response.body["runtime"]["state"] == expected_state {
                return;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("instance reaches the expected state")
}

async fn wait_for_log_line(
    address: SocketAddr,
    authorization: &str,
    core_id: &str,
    instance_id: &str,
    expected_line: &str,
) {
    timeout(Duration::from_secs(5), async {
        loop {
            let response = send_json_request(
                address,
                "GET",
                &format!("/api/v1/cores/{core_id}/instances/{instance_id}/logs?limit=200"),
                &[("Authorization", authorization)],
                None,
            )
            .await;
            assert_eq!(response.status, 200);
            if response.body["items"]
                .as_array()
                .is_some_and(|items| items.iter().any(|item| item["line"] == expected_line))
            {
                return;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("expected log line is visible through the Panel REST API")
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

fn safe_process_create(identifier: &str) -> Value {
    json!({
        "id": identifier,
        "name": identifier,
        "kind": "PAPER",
        "directory": format!("instances/{identifier}"),
        "launch": safe_process_launch_config(),
    })
}

#[cfg(windows)]
fn safe_process_launch_config() -> Value {
    json!({
        "executable": "powershell.exe",
        "args": [
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$ErrorActionPreference='Stop'; [Console]::Out.WriteLine('ready'); [Console]::Error.WriteLine('warning'); while (($line = [Console]::In.ReadLine()) -ne $null) { if ($line -eq 'stop') { exit 0 }; [Console]::Out.WriteLine(\"received:$line\") }",
        ],
        "environment": {},
        "stopCommand": "stop",
        "stopTimeoutSeconds": 5,
    })
}

#[cfg(not(windows))]
fn safe_process_launch_config() -> Value {
    json!({
        "executable": "/bin/sh",
        "args": [
            "-c",
            "printf 'ready\\n'; printf 'warning\\n' >&2; while IFS= read -r line; do if [ \"$line\" = stop ]; then exit 0; fi; printf 'received:%s\\n' \"$line\"; done",
        ],
        "environment": {},
        "stopCommand": "stop",
        "stopTimeoutSeconds": 5,
    })
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
            from_str(body).expect("HTTP response body is JSON")
        };

        Self {
            status,
            headers,
            body,
        }
    }
}
