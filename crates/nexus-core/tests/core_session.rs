use nexus_config::CoreConfig;
use nexus_core::CoreServer;
use nexus_domain::RequestId;
use nexus_protocol::CURRENT_PROTOCOL_VERSION;
use nexus_protocol::NoiseTransport;
use nexus_protocol::PresharedKey;
use nexus_protocol::WireMessage;
use serde_json::json;
use tempfile::tempdir;
use tokio::net::TcpStream;

const TEST_PSK: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY";

#[tokio::test]
async fn accepts_an_encrypted_session_hello() {
    let data_directory = tempdir().expect("temporary Core data directory is created");
    let config = CoreConfig::new(
        "127.0.0.1:0".to_owned(),
        data_directory.path().to_path_buf(),
        Some(TEST_PSK.to_owned()),
    )
    .expect("test Core configuration is valid");
    let server = CoreServer::bind(&config)
        .await
        .expect("Core listener binds");
    let listen_address = server.listen_address();
    let server_task = tokio::spawn(server.serve());
    let pre_shared_key = PresharedKey::from_base64url(TEST_PSK).expect("test PSK is valid");
    let stream = TcpStream::connect(listen_address)
        .await
        .expect("Panel connects to Core");
    let mut transport = NoiseTransport::connect(stream, &pre_shared_key)
        .await
        .expect("Noise handshake succeeds");
    let request_id = RequestId::new();
    let hello = WireMessage::Request {
        request_id,
        method: "session.hello".to_owned(),
        params: json!({
            "protocol": CURRENT_PROTOCOL_VERSION,
            "panelId": RequestId::new(),
            "panelName": "test-panel",
            "clientVersion": "0.1.0",
            "capabilities": ["events", "instances"],
        }),
        deadline: None,
        idempotency_key: None,
    };

    transport
        .write_message(&hello)
        .await
        .expect("session.hello is sent");

    let response = transport
        .read_message()
        .await
        .expect("session.welcome is received");
    let WireMessage::Response {
        request_id: response_id,
        ok,
        result,
        error,
    } = response
    else {
        panic!("Core returned a non-response message");
    };
    let result = result.expect("successful response includes a result");

    assert_eq!(response_id, request_id);
    assert!(ok);
    assert!(error.is_none());
    assert_eq!(result["protocol"], json!(CURRENT_PROTOCOL_VERSION));
    assert_eq!(result["capabilities"], json!(["events", "instances"]));
    assert!(result["coreId"].as_str().is_some());

    let invalid_request_id = RequestId::new();
    transport
        .write_message(&WireMessage::Request {
            request_id: invalid_request_id,
            method: "instance.create".to_owned(),
            params: json!({
                "id": "outside",
                "name": "Outside",
                "kind": "PAPER",
                "directory": "../outside",
                "launch": {
                    "executable": "java",
                    "args": [],
                    "environment": {},
                    "stopCommand": "stop",
                    "stopTimeoutSeconds": 30,
                },
            }),
            deadline: None,
            idempotency_key: None,
        })
        .await
        .expect("invalid instance request is sent");

    let invalid_response = transport
        .read_message()
        .await
        .expect("Core responds to the invalid instance request");
    let WireMessage::Response {
        request_id: response_id,
        ok,
        result,
        error,
    } = invalid_response
    else {
        panic!("Core returned a non-response message");
    };

    assert_eq!(response_id, invalid_request_id);
    assert!(!ok);
    assert!(result.is_none());
    assert_eq!(
        error.expect("rejected request includes an error").code,
        "BAD_REQUEST"
    );

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn retains_the_same_core_id_between_listener_restarts() {
    let data_directory = tempdir().expect("temporary Core data directory is created");
    let config = CoreConfig::new(
        "127.0.0.1:0".to_owned(),
        data_directory.path().to_path_buf(),
        Some(TEST_PSK.to_owned()),
    )
    .expect("test Core configuration is valid");
    let first_server = CoreServer::bind(&config)
        .await
        .expect("first Core listener binds");
    let first_id = first_server.core_id();

    drop(first_server);

    let second_server = CoreServer::bind(&config)
        .await
        .expect("second Core listener binds");

    assert_eq!(second_server.core_id(), first_id);
}
