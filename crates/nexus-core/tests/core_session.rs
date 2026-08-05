use nexus_config::CoreConfig;
use nexus_core::CoreServer;
use nexus_domain::RequestId;
use nexus_protocol::CURRENT_PROTOCOL_VERSION;
use nexus_protocol::NoiseTransport;
use nexus_protocol::PresharedKey;
use nexus_protocol::TlsError;
use nexus_protocol::WireMessage;
use nexus_protocol::connect_tls;
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
    let (stream, certificate_sha256) = connect_tls(stream, "localhost".to_owned(), false)
        .await
        .expect("TLS handshake succeeds");
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
            "capabilities": [
                "cpu-topology",
                "cpu-policy",
                "cpu-reservations",
                "events",
                "instances"
            ],
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
    assert_eq!(
        result["capabilities"],
        json!([
            "cpu-topology",
            "cpu-policy",
            "cpu-reservations",
            "events",
            "instances"
        ])
    );
    assert!(result["coreId"].as_str().is_some());
    assert_eq!(result["tlsCertificateSha256"], certificate_sha256);

    let topology_request_id = RequestId::new();
    transport
        .write_message(&WireMessage::Request {
            request_id: topology_request_id,
            method: "cpu.topology".to_owned(),
            params: json!({}),
            deadline: None,
            idempotency_key: None,
        })
        .await
        .expect("CPU topology request is sent");
    let topology_response = transport
        .read_message()
        .await
        .expect("CPU topology response is received");
    let WireMessage::Response {
        request_id: response_id,
        ok,
        result,
        error,
    } = topology_response
    else {
        panic!("Core returned a non-response message for CPU topology");
    };
    let topology = result.expect("successful CPU topology response includes a result");
    assert_eq!(response_id, topology_request_id);
    assert!(ok);
    assert!(error.is_none());
    assert!(
        topology["logicalCpus"]
            .as_array()
            .is_some_and(|cpus| !cpus.is_empty())
    );
    assert_eq!(topology["detection"]["confidence"], "LOW");
    assert!(
        topology["available"]["performanceCpuIds"]
            .as_array()
            .is_some_and(Vec::is_empty)
    );

    let policy_request_id = RequestId::new();
    transport
        .write_message(&WireMessage::Request {
            request_id: policy_request_id,
            method: "cpu.policy.resolve".to_owned(),
            params: json!({
                "mode": "AUTO",
                "requestedCpuIds": [],
                "minCpus": 1,
                "maxCpus": null,
                "preferPhysicalCores": true,
                "numaNode": null,
                "shareMode": "SHARED",
                "strict": false,
            }),
            deadline: None,
            idempotency_key: None,
        })
        .await
        .expect("CPU policy request is sent");
    let policy_response = transport
        .read_message()
        .await
        .expect("CPU policy response is received");
    let WireMessage::Response {
        request_id: response_id,
        ok,
        result,
        error,
    } = policy_response
    else {
        panic!("Core returned a non-response message for CPU policy");
    };
    let policy = result.expect("successful CPU policy response includes a result");
    assert_eq!(response_id, policy_request_id);
    assert!(ok);
    assert!(error.is_none());
    assert!(policy["candidateCpuIds"].as_array().is_some());
    assert!(policy["selectedCpuIds"].as_array().is_some());

    let first_instance_id = RequestId::new();
    transport
        .write_message(&WireMessage::Request {
            request_id: first_instance_id,
            method: "instance.create".to_owned(),
            params: json!({
                "id": "reserved-first",
                "name": "Reserved First",
                "kind": "PAPER",
                "directory": "instances/reserved-first",
                "launch": {
                    "executable": "java",
                    "args": ["-jar", "server.jar"],
                    "environment": {},
                    "stopCommand": "stop",
                    "stopTimeoutSeconds": 30,
                },
            }),
            deadline: None,
            idempotency_key: None,
        })
        .await
        .expect("first reservation instance request is sent");
    let first_instance_response = transport
        .read_message()
        .await
        .expect("first reservation instance response is received");
    let WireMessage::Response {
        request_id: response_id,
        ok,
        result,
        error,
    } = first_instance_response
    else {
        panic!("Core returned a non-response for the first reservation instance");
    };
    let first_instance = result.expect("first reservation instance includes a result");
    assert_eq!(response_id, first_instance_id);
    assert!(ok);
    assert!(error.is_none());
    assert_eq!(first_instance["revision"], 1);

    let reserve_request_id = RequestId::new();
    transport
        .write_message(&WireMessage::Request {
            request_id: reserve_request_id,
            method: "cpu.reserve".to_owned(),
            params: json!({
                "instanceId": "reserved-first",
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
            }),
            deadline: None,
            idempotency_key: Some(RequestId::new().to_string()),
        })
        .await
        .expect("CPU reservation request is sent");
    let reserve_response = transport
        .read_message()
        .await
        .expect("CPU reservation response is received");
    let WireMessage::Response {
        request_id: response_id,
        ok,
        result,
        error,
    } = reserve_response
    else {
        panic!("Core returned a non-response for CPU reservation");
    };
    let reservation_result = result.expect("successful reservation includes a result");
    assert_eq!(response_id, reserve_request_id);
    assert!(ok);
    assert!(error.is_none());
    let reservation_id = reservation_result["reservation"]["reservationId"]
        .as_str()
        .expect("reservation ID is returned")
        .to_owned();
    assert_eq!(
        reservation_result["reservation"]["instanceId"],
        "reserved-first"
    );
    assert!(
        reservation_result["appliedPolicy"]["selectedCpuIds"]
            .as_array()
            .is_some_and(|ids| !ids.is_empty())
    );

    let list_request_id = RequestId::new();
    transport
        .write_message(&WireMessage::Request {
            request_id: list_request_id,
            method: "cpu.reservation.list".to_owned(),
            params: json!({}),
            deadline: None,
            idempotency_key: None,
        })
        .await
        .expect("CPU reservation list request is sent");
    let list_response = transport
        .read_message()
        .await
        .expect("CPU reservation list response is received");
    let WireMessage::Response {
        request_id: response_id,
        ok,
        result,
        error,
    } = list_response
    else {
        panic!("Core returned a non-response for CPU reservation list");
    };
    let reservations = result.expect("reservation list includes a result");
    assert_eq!(response_id, list_request_id);
    assert!(ok);
    assert!(error.is_none());
    assert_eq!(reservations["items"].as_array().map(Vec::len), Some(1));

    let release_request_id = RequestId::new();
    transport
        .write_message(&WireMessage::Request {
            request_id: release_request_id,
            method: "cpu.release".to_owned(),
            params: json!({ "reservationId": reservation_id }),
            deadline: None,
            idempotency_key: Some(RequestId::new().to_string()),
        })
        .await
        .expect("CPU reservation release request is sent");
    let release_response = transport
        .read_message()
        .await
        .expect("CPU reservation release response is received");
    let WireMessage::Response {
        request_id: response_id,
        ok,
        result,
        error,
    } = release_response
    else {
        panic!("Core returned a non-response for CPU reservation release");
    };
    assert_eq!(response_id, release_request_id);
    assert!(ok);
    assert_eq!(result, Some(json!({})));
    assert!(error.is_none());

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
    let first_certificate_sha256 = first_server.certificate_sha256().to_owned();

    drop(first_server);

    let second_server = CoreServer::bind(&config)
        .await
        .expect("second Core listener binds");

    assert_eq!(second_server.core_id(), first_id);
    assert_eq!(second_server.certificate_sha256(), first_certificate_sha256);
    assert!(data_directory.path().join("tls/core-cert.pem").is_file());
    assert!(data_directory.path().join("tls/core-key.pem").is_file());

    let configured = CoreConfig::new(
        "127.0.0.1:0".to_owned(),
        data_directory.path().to_path_buf(),
        Some(TEST_PSK.to_owned()),
    )
    .expect("test Core configuration is valid")
    .with_tls_identity_paths(
        Some(data_directory.path().join("tls/core-cert.pem")),
        Some(data_directory.path().join("tls/core-key.pem")),
    )
    .expect("configured TLS identity paths are valid");
    drop(second_server);
    let configured_server = CoreServer::bind(&configured)
        .await
        .expect("configured Core TLS identity loads");
    assert_eq!(
        configured_server.certificate_sha256(),
        first_certificate_sha256
    );
}

#[tokio::test]
async fn rejects_the_default_certificate_when_strict_verification_is_requested() {
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
    let stream = TcpStream::connect(listen_address)
        .await
        .expect("Panel connects to Core");

    let error = connect_tls(stream, "localhost".to_owned(), true)
        .await
        .expect_err("strict verification rejects the self-signed certificate");
    assert!(matches!(error, TlsError::Handshake(_)));

    server_task.abort();
    let _ = server_task.await;
}
