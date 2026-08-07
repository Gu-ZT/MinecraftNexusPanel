use std::fs;
use std::net::SocketAddr;

use nexus_config::PanelConfig;
use nexus_config::PanelMasterKey;
use nexus_domain::RequestId;
use nexus_panel::PanelError;
use nexus_panel::PanelServer;
use tempfile::tempdir;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

#[tokio::test]
async fn responds_to_the_liveness_probe() {
    let data_directory = tempdir().expect("temporary Panel data directory is created");
    let config = PanelConfig::new(
        "127.0.0.1:0".to_owned(),
        data_directory.path().to_path_buf(),
    )
    .expect("test Panel configuration is valid")
    .with_master_key(PanelMasterKey::from_bytes([11_u8; 32]));
    let server = PanelServer::bind(&config)
        .await
        .expect("Panel listener binds");
    let listen_address = server.listen_address();
    let server_task = tokio::spawn(server.serve());
    let mut stream = TcpStream::connect(listen_address)
        .await
        .expect("health client connects");
    let request_id = RequestId::new();
    let request = format!(
        "GET /api/v1/health/live HTTP/1.1\r\nHost: localhost\r\nX-Request-Id: {request_id}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .expect("health request is sent");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .await
        .expect("health response is read");

    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains(&format!("x-request-id: {request_id}")));
    assert!(response.contains("\"status\":\"ok\""));

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn refuses_to_start_without_a_panel_master_key() {
    let data_directory = tempdir().expect("temporary Panel data directory is created");
    let config = PanelConfig::new(
        "127.0.0.1:0".to_owned(),
        data_directory.path().to_path_buf(),
    )
    .expect("test Panel configuration is valid");

    assert!(matches!(
        PanelServer::bind(&config).await,
        Err(PanelError::MissingPanelMasterKey)
    ));
    assert!(!data_directory.path().join("panel.db").exists());
}

#[tokio::test]
async fn serves_the_webui_and_preserves_unknown_api_responses() {
    let data_directory = tempdir().expect("temporary Panel data directory is created");
    let web_root = data_directory.path().join("web");
    fs::create_dir(&web_root).expect("WebUI directory is created");
    fs::write(web_root.join("index.html"), "<main>MCNP WebUI</main>")
        .expect("WebUI entry is written");
    fs::write(web_root.join("app.js"), "globalThis.mcnp = true;").expect("WebUI script is written");
    let config = PanelConfig::new(
        "127.0.0.1:0".to_owned(),
        data_directory.path().to_path_buf(),
    )
    .expect("test Panel configuration is valid")
    .with_master_key(PanelMasterKey::from_bytes([12_u8; 32]))
    .with_web_root(web_root);
    let server = PanelServer::bind(&config)
        .await
        .expect("Panel listener binds");
    let listen_address = server.listen_address();
    let server_task = tokio::spawn(server.serve());

    let root = send_get(listen_address, "/").await;
    assert!(root.starts_with("HTTP/1.1 200 OK"));
    assert!(root.contains("<main>MCNP WebUI</main>"));

    let nested_route = send_get(listen_address, "/instances/local/example/console").await;
    assert!(nested_route.starts_with("HTTP/1.1 200 OK"));
    assert!(nested_route.contains("<main>MCNP WebUI</main>"));

    let script = send_get(listen_address, "/app.js").await;
    assert!(script.starts_with("HTTP/1.1 200 OK"));
    assert!(script.contains("content-type: text/javascript"));
    assert!(script.contains("globalThis.mcnp = true;"));

    let unknown_api = send_get(listen_address, "/api/v1/unknown").await;
    assert!(unknown_api.starts_with("HTTP/1.1 404 Not Found"));
    assert!(!unknown_api.contains("MCNP WebUI"));

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn refuses_a_web_root_without_an_entry_document() {
    let data_directory = tempdir().expect("temporary Panel data directory is created");
    let web_root = data_directory.path().join("web");
    fs::create_dir(&web_root).expect("empty WebUI directory is created");
    let expected_entry = web_root.join("index.html");
    let config = PanelConfig::new(
        "127.0.0.1:0".to_owned(),
        data_directory.path().to_path_buf(),
    )
    .expect("test Panel configuration is valid")
    .with_master_key(PanelMasterKey::from_bytes([13_u8; 32]))
    .with_web_root(web_root);

    assert!(matches!(
        PanelServer::bind(&config).await,
        Err(PanelError::MissingWebEntry { path }) if path == expected_entry
    ));
}

async fn send_get(address: SocketAddr, path: &str) -> String {
    let mut stream = TcpStream::connect(address)
        .await
        .expect("HTTP client connects");
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("HTTP request is sent");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .await
        .expect("HTTP response is read");
    response
}
