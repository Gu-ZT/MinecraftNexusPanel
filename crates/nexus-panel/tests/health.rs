use nexus_config::PanelConfig;
use nexus_domain::RequestId;
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
    .expect("test Panel configuration is valid");
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
