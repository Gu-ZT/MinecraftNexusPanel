use std::net::SocketAddr;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
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
use sha1::Digest;
use sha1::Sha1;
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
async fn issues_a_ticket_and_streams_a_core_status_snapshot() {
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
    let ticket_response = send_json_request(
        panel_address,
        "POST",
        "/api/v1/ws/tickets",
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(ticket_response.status, 201);
    let ticket = ticket_response.body["ticket"]
        .as_str()
        .expect("ticket is returned");

    let mut websocket = connect_websocket(panel_address, ticket).await;
    let ready = read_websocket_json(&mut websocket).await;
    assert_eq!(ready["type"], "ready");

    write_websocket_json(
        &mut websocket,
        json!({
            "type": "subscribe",
            "messageId": RequestId::new().to_string(),
            "topic": format!("core/{core_id}/status"),
        }),
    )
    .await;
    let ack = read_websocket_json(&mut websocket).await;
    let event = read_websocket_json(&mut websocket).await;

    assert_eq!(ack["type"], "ack");
    assert_eq!(event["type"], "event");
    assert_eq!(event["topic"], format!("core/{core_id}/status"));
    assert_eq!(event["data"]["id"], core_id);
    assert_eq!(event["data"]["status"], "ONLINE");

    panel_task.abort();
    let _ = panel_task.await;
    core_task.abort();
    let _ = core_task.await;
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
    .with_master_key(PanelMasterKey::from_bytes([31_u8; 32]));
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

async fn connect_websocket(address: SocketAddr, ticket: &str) -> TcpStream {
    let key = "dGhlIHNhbXBsZSBub25jZQ==";
    let expected_accept = websocket_accept(key);
    let request = format!(
        "GET /api/v1/ws?ticket={ticket} HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    let mut stream = TcpStream::connect(address)
        .await
        .expect("WebSocket client connects");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("WebSocket upgrade request is sent");
    let response = read_http_head(&mut stream).await;

    assert!(response.starts_with("HTTP/1.1 101 Switching Protocols"));
    assert!(
        response
            .to_ascii_lowercase()
            .contains(&format!("sec-websocket-accept: {}", expected_accept).to_ascii_lowercase())
    );

    stream
}

async fn read_websocket_json(stream: &mut TcpStream) -> Value {
    let mut header = [0_u8; 2];
    stream
        .read_exact(&mut header)
        .await
        .expect("WebSocket frame header is read");
    let opcode = header[0] & 0x0f;
    let masked = header[1] & 0x80 != 0;
    let mut length = u64::from(header[1] & 0x7f);
    if length == 126 {
        let mut bytes = [0_u8; 2];
        stream
            .read_exact(&mut bytes)
            .await
            .expect("WebSocket extended frame length is read");
        length = u64::from(u16::from_be_bytes(bytes));
    }
    assert_eq!(opcode, 1);
    assert!(!masked);
    let mut payload = vec![0_u8; usize::try_from(length).expect("frame length fits in memory")];
    stream
        .read_exact(&mut payload)
        .await
        .expect("WebSocket frame payload is read");

    from_str(std::str::from_utf8(&payload).expect("payload is UTF-8")).expect("payload is JSON")
}

async fn write_websocket_json(stream: &mut TcpStream, value: Value) {
    let payload = value.to_string().into_bytes();
    assert!(payload.len() <= u16::MAX.into());
    let mask = [1_u8, 2, 3, 4];
    let mut frame = Vec::with_capacity(8 + payload.len());
    frame.push(0x81);
    if payload.len() <= 125 {
        frame.push(
            0x80 | u8::try_from(payload.len()).expect("payload length fits in a short frame"),
        );
    } else {
        frame.push(0x80 | 126);
        frame.extend_from_slice(
            &u16::try_from(payload.len())
                .expect("payload length fits in an extended frame")
                .to_be_bytes(),
        );
    }
    frame.extend_from_slice(&mask);
    frame.extend(
        payload
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ mask[index % mask.len()]),
    );
    stream
        .write_all(&frame)
        .await
        .expect("WebSocket frame is sent");
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

async fn read_http_head(stream: &mut TcpStream) -> String {
    let mut response = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        stream
            .read_exact(&mut byte)
            .await
            .expect("HTTP response byte is read");
        response.push(byte[0]);
        if response.ends_with(b"\r\n\r\n") {
            break;
        }
    }

    String::from_utf8(response).expect("HTTP response head is UTF-8")
}

fn websocket_accept(key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");

    STANDARD.encode(hasher.finalize())
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
