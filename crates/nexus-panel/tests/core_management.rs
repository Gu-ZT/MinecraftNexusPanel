use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
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
async fn registers_encrypts_restores_and_reconnects_a_core() {
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
    let create_idempotency_key = RequestId::new().to_string();
    let created = send_json_request(
        panel_address,
        "POST",
        "/api/v1/cores",
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", create_idempotency_key.as_str()),
        ],
        Some(json!({
            "name": "Game Node",
            "address": core_address.to_string(),
            "secret": CORE_PSK,
            "connectTimeoutSeconds": 3,
            "tags": ["production", "cn-east", "production"],
        })),
    )
    .await;

    assert_eq!(created.status, 201);
    assert_eq!(created.body["status"], "ONLINE");
    assert_eq!(created.body["tags"], json!(["cn-east", "production"]));
    assert!(created.body.get("secret").is_none());
    assert_eq!(
        created.headers.get("etag").map(String::as_str),
        Some("\"1\"")
    );
    let core_id = created.body["id"]
        .as_str()
        .expect("registered Core ID is returned")
        .to_owned();
    assert_secret_is_not_persisted_in_plaintext(panel_data.path());

    let test_idempotency_key = RequestId::new().to_string();
    let tested = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/actions/test"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", test_idempotency_key.as_str()),
        ],
        None,
    )
    .await;
    assert_eq!(tested.status, 200);
    assert_eq!(tested.body["success"], true);
    assert_eq!(tested.body["protocolVersion"], "1.0");

    let topology = send_json_request(
        panel_address,
        "GET",
        &format!("/api/v1/cores/{core_id}/cpu-topology"),
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(topology.status, 200);
    assert!(
        topology.body["logicalCpus"]
            .as_array()
            .is_some_and(|cpus| !cpus.is_empty())
    );
    assert_eq!(topology.body["detection"]["confidence"], "LOW");

    let policy = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/cpu-policies:resolve"),
        &[("Authorization", authorization.as_str())],
        Some(serde_json::json!({
            "mode": "AUTO",
            "requestedCpuIds": [],
            "minCpus": 1,
            "maxCpus": null,
            "preferPhysicalCores": true,
            "numaNode": null,
            "shareMode": "SHARED",
            "strict": false,
        })),
    )
    .await;
    assert_eq!(policy.status, 200);
    assert!(policy.body["candidateCpuIds"].as_array().is_some());

    let instance = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/instances"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", RequestId::new().to_string().as_str()),
        ],
        Some(json!({
            "id": "cpu-reserved",
            "name": "CPU Reserved",
            "kind": "PAPER",
            "directory": "instances/cpu-reserved",
            "launch": {
                "executable": "java",
                "args": ["-jar", "server.jar"],
                "environment": {},
                "stopCommand": "stop",
                "stopTimeoutSeconds": 30,
            },
        })),
    )
    .await;
    assert_eq!(instance.status, 201);
    assert_eq!(instance.body["revision"], 1);

    let reservation_without_key = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/cpu-reservations"),
        &[("Authorization", authorization.as_str())],
        Some(json!({
            "instanceId": "cpu-reserved",
            "revision": 1,
            "policy": {
                "mode": "AUTO",
                "requestedCpuIds": [],
                "minCpus": 1,
                "maxCpus": null,
                "preferPhysicalCores": true,
                "numaNode": null,
                "shareMode": "EXCLUSIVE",
                "strict": false,
            },
        })),
    )
    .await;
    assert_eq!(reservation_without_key.status, 428);

    let reservation = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/cpu-reservations"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", RequestId::new().to_string().as_str()),
        ],
        Some(json!({
            "instanceId": "cpu-reserved",
            "revision": 1,
            "policy": {
                "mode": "AUTO",
                "requestedCpuIds": [],
                "minCpus": 1,
                "maxCpus": null,
                "preferPhysicalCores": true,
                "numaNode": null,
                "shareMode": "EXCLUSIVE",
                "strict": false,
            },
        })),
    )
    .await;
    assert_eq!(reservation.status, 201);
    assert_eq!(
        reservation.body["reservation"]["instanceId"],
        "cpu-reserved"
    );
    assert!(
        reservation.body["appliedPolicy"]["selectedCpuIds"]
            .as_array()
            .is_some_and(|ids| !ids.is_empty())
    );
    let reservation_id = reservation.body["reservation"]["reservationId"]
        .as_str()
        .expect("Panel returns a CPU reservation ID")
        .to_owned();

    let listed_reservations = send_json_request(
        panel_address,
        "GET",
        &format!("/api/v1/cores/{core_id}/cpu-reservations"),
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(listed_reservations.status, 200);
    assert_eq!(
        listed_reservations.body["items"].as_array().map(Vec::len),
        Some(1)
    );

    let released = send_json_request(
        panel_address,
        "DELETE",
        &format!("/api/v1/cores/{core_id}/cpu-reservations/{reservation_id}"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", RequestId::new().to_string().as_str()),
        ],
        None,
    )
    .await;
    assert_eq!(released.status, 204);

    let listed_after_release = send_json_request(
        panel_address,
        "GET",
        &format!("/api/v1/cores/{core_id}/cpu-reservations"),
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(listed_after_release.status, 200);
    assert_eq!(
        listed_after_release.body["items"].as_array().map(Vec::len),
        Some(0)
    );

    stop_panel(panel_task).await;
    let (restored_address, restored_task) = start_panel(&panel_data).await;
    let restored_token = login(restored_address).await;
    let restored_authorization = format!("Bearer {restored_token}");
    let restored = wait_for_core_status(
        restored_address,
        &restored_authorization,
        &core_id,
        "ONLINE",
    )
    .await;
    assert_eq!(restored["id"], core_id);
    assert_eq!(restored["name"], "Game Node");

    core_task.abort();
    let _ = core_task.await;
    let reconnect = send_json_request(
        restored_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/actions/reconnect"),
        &[("Authorization", restored_authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(reconnect.status, 202);
    wait_for_core_status(
        restored_address,
        &restored_authorization,
        &core_id,
        "OFFLINE",
    )
    .await;

    stop_panel(restored_task).await;
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
    .with_master_key(PanelMasterKey::from_bytes([19_u8; 32]));
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

async fn wait_for_core_status(
    address: SocketAddr,
    authorization: &str,
    core_id: &str,
    expected_status: &str,
) -> Value {
    timeout(Duration::from_secs(5), async {
        loop {
            let response = send_json_request(
                address,
                "GET",
                &format!("/api/v1/cores/{core_id}"),
                &[("Authorization", authorization)],
                None,
            )
            .await;
            assert_eq!(response.status, 200);
            if response.body["status"] == expected_status {
                return response.body;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("Core reaches the expected connection status")
}

fn assert_secret_is_not_persisted_in_plaintext(data_directory: &Path) {
    for entry in fs::read_dir(data_directory).expect("Panel data directory is readable") {
        let path = entry.expect("Panel data entry is readable").path();
        if !path.is_file() {
            continue;
        }
        let contents = fs::read(&path).expect("Panel data file is readable");
        assert!(
            !contents
                .windows(CORE_PSK.len())
                .any(|window| window == CORE_PSK.as_bytes()),
            "Core PSK was persisted in plaintext in {}",
            path.display()
        );
    }
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
            from_str(body).expect("HTTP response body is JSON")
        };

        Self {
            status,
            headers,
            body,
        }
    }
}
