use std::collections::BTreeMap;
use std::time::Duration;

use nexus_config::CoreConfig;
use nexus_core::CoreServer;
use nexus_domain::Instance;
use nexus_domain::InstanceCreate;
use nexus_domain::InstanceId;
use nexus_domain::InstanceKind;
use nexus_domain::InstanceState;
use nexus_domain::LaunchConfig;
use nexus_domain::RequestId;
use nexus_domain::TaskId;
use nexus_protocol::CURRENT_PROTOCOL_VERSION;
use nexus_protocol::NoiseTransport;
use nexus_protocol::PresharedKey;
use nexus_protocol::WireError;
use nexus_protocol::WireMessage;
use serde_json::Value;
use serde_json::from_value;
use serde_json::json;
use serde_json::to_value;
use tempfile::tempdir;
use tokio::net::TcpStream;
use tokio::spawn;
use tokio::time::timeout;

const TEST_PSK: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY";

#[tokio::test]
async fn starts_stops_and_kills_a_safe_test_process_with_state_events() {
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
    let server_task = spawn(server.serve());
    let pre_shared_key = PresharedKey::from_base64url(TEST_PSK).expect("test PSK is valid");
    let stream = TcpStream::connect(listen_address)
        .await
        .expect("Panel connects to Core");
    let mut transport = NoiseTransport::connect(stream, &pre_shared_key)
        .await
        .expect("Noise handshake succeeds");

    establish_session(&mut transport).await;
    let instance_id = InstanceId::new("safe-process".to_owned()).expect("instance ID is valid");
    let create_request_id = send_request(
        &mut transport,
        "instance.create",
        to_value(instance_create(&instance_id)).expect("instance definition is serializable"),
        None,
    )
    .await;
    read_success(&mut transport, create_request_id, &mut Vec::new()).await;

    let subscribe_request_id = send_request(
        &mut transport,
        "event.subscribe",
        json!({ "topics": ["instance.state"] }),
        None,
    )
    .await;
    let subscription = read_success(&mut transport, subscribe_request_id, &mut Vec::new()).await;
    assert!(subscription["subscriptionId"].as_str().is_some());

    let missing_key_request_id = send_request(
        &mut transport,
        "instance.start",
        json!({ "instanceId": instance_id }),
        None,
    )
    .await;
    assert_eq!(
        read_error(&mut transport, missing_key_request_id)
            .await
            .code,
        "PRECONDITION_REQUIRED"
    );

    let start_request_id = send_request(
        &mut transport,
        "instance.start",
        json!({ "instanceId": instance_id }),
        Some("start-safe-process"),
    )
    .await;
    let mut observed_states = Vec::new();
    let start_result = read_success(&mut transport, start_request_id, &mut observed_states).await;
    start_result["taskId"]
        .as_str()
        .expect("start response includes a task ID")
        .parse::<TaskId>()
        .expect("start task ID is valid");
    wait_for_state(&mut transport, InstanceState::Running, &mut observed_states).await;
    assert_eq!(
        observed_states,
        [InstanceState::Starting, InstanceState::Running]
    );

    let get_request_id = send_request(
        &mut transport,
        "instance.get",
        json!({ "instanceId": instance_id }),
        None,
    )
    .await;
    let running: Instance =
        from_value(read_success(&mut transport, get_request_id, &mut Vec::new()).await)
            .expect("running instance is valid");
    assert_eq!(running.runtime().state(), InstanceState::Running);
    assert!(running.runtime().pid().is_some());

    let duplicate_start_request_id = send_request(
        &mut transport,
        "instance.start",
        json!({ "instanceId": instance_id }),
        Some("duplicate-start"),
    )
    .await;
    assert_eq!(
        read_error(&mut transport, duplicate_start_request_id)
            .await
            .code,
        "INSTANCE_STATE_CONFLICT"
    );

    let stop_request_id = send_request(
        &mut transport,
        "instance.stop",
        json!({
            "instanceId": instance_id,
            "timeoutSeconds": 5,
        }),
        Some("stop-safe-process"),
    )
    .await;
    let mut stopped_states = Vec::new();
    read_success(&mut transport, stop_request_id, &mut stopped_states).await;
    wait_for_state(&mut transport, InstanceState::Stopped, &mut stopped_states).await;
    assert_eq!(
        stopped_states,
        [InstanceState::Stopping, InstanceState::Stopped]
    );

    let restart_request_id = send_request(
        &mut transport,
        "instance.start",
        json!({ "instanceId": instance_id }),
        Some("restart-safe-process"),
    )
    .await;
    let mut restarted_states = Vec::new();
    read_success(&mut transport, restart_request_id, &mut restarted_states).await;
    wait_for_state(
        &mut transport,
        InstanceState::Running,
        &mut restarted_states,
    )
    .await;

    let invalid_kill_request_id = send_request(
        &mut transport,
        "instance.kill",
        json!({
            "instanceId": instance_id,
            "confirmation": "different-instance",
        }),
        Some("invalid-kill"),
    )
    .await;
    assert_eq!(
        read_error(&mut transport, invalid_kill_request_id)
            .await
            .code,
        "BAD_REQUEST"
    );

    let kill_request_id = send_request(
        &mut transport,
        "instance.kill",
        json!({
            "instanceId": instance_id,
            "confirmation": instance_id,
        }),
        Some("kill-safe-process"),
    )
    .await;
    let mut killed_states = Vec::new();
    read_success(&mut transport, kill_request_id, &mut killed_states).await;
    wait_for_state(&mut transport, InstanceState::Stopped, &mut killed_states).await;
    assert_eq!(killed_states, [InstanceState::Stopped]);
    assert!(
        data_directory
            .path()
            .join("instances/safe-process")
            .is_dir()
    );

    let crashing_instance_id =
        InstanceId::new("crashing-process".to_owned()).expect("instance ID is valid");
    let create_crashing_request_id = send_request(
        &mut transport,
        "instance.create",
        to_value(crashing_process_create(&crashing_instance_id))
            .expect("instance definition is serializable"),
        None,
    )
    .await;
    read_success(&mut transport, create_crashing_request_id, &mut Vec::new()).await;
    let start_crashing_request_id = send_request(
        &mut transport,
        "instance.start",
        json!({ "instanceId": crashing_instance_id }),
        Some("start-crashing-process"),
    )
    .await;
    let mut failed_states = Vec::new();
    read_success(
        &mut transport,
        start_crashing_request_id,
        &mut failed_states,
    )
    .await;
    wait_for_state(&mut transport, InstanceState::Failed, &mut failed_states).await;
    assert_eq!(
        failed_states,
        [
            InstanceState::Starting,
            InstanceState::Running,
            InstanceState::Failed,
        ]
    );
    let get_crashing_request_id = send_request(
        &mut transport,
        "instance.get",
        json!({ "instanceId": crashing_instance_id }),
        None,
    )
    .await;
    let failed: Instance =
        from_value(read_success(&mut transport, get_crashing_request_id, &mut Vec::new()).await)
            .expect("failed instance is valid");
    assert_eq!(failed.runtime().state(), InstanceState::Failed);
    assert_eq!(failed.runtime().exit_code(), Some(7));

    server_task.abort();
    let _ = server_task.await;
}

async fn establish_session(transport: &mut NoiseTransport<TcpStream>) {
    let request_id = send_request(
        transport,
        "session.hello",
        json!({
            "protocol": CURRENT_PROTOCOL_VERSION,
            "panelId": RequestId::new(),
            "panelName": "lifecycle-test",
            "clientVersion": "0.1.0",
            "capabilities": ["events", "instances"],
        }),
        None,
    )
    .await;

    read_success(transport, request_id, &mut Vec::new()).await;
}

async fn send_request(
    transport: &mut NoiseTransport<TcpStream>,
    method: &str,
    params: Value,
    idempotency_key: Option<&str>,
) -> RequestId {
    let request_id = RequestId::new();
    transport
        .write_message(&WireMessage::Request {
            request_id,
            method: method.to_owned(),
            params,
            deadline: None,
            idempotency_key: idempotency_key.map(str::to_owned),
        })
        .await
        .expect("Core request is sent");

    request_id
}

async fn read_success(
    transport: &mut NoiseTransport<TcpStream>,
    expected_request_id: RequestId,
    observed_states: &mut Vec<InstanceState>,
) -> Value {
    loop {
        match transport
            .read_message()
            .await
            .expect("Core message is received")
        {
            WireMessage::Response {
                request_id,
                ok,
                result,
                error,
            } => {
                assert_eq!(request_id, expected_request_id);
                assert!(ok, "Core rejected request: {error:?}");
                return result.expect("successful response includes a result");
            }
            event @ WireMessage::Event { .. } => {
                record_state(event, observed_states);
            }
            WireMessage::Request { .. } => panic!("Core sent an unexpected request"),
        }
    }
}

async fn read_error(
    transport: &mut NoiseTransport<TcpStream>,
    expected_request_id: RequestId,
) -> WireError {
    loop {
        match transport
            .read_message()
            .await
            .expect("Core message is received")
        {
            WireMessage::Response {
                request_id,
                ok,
                result,
                error,
            } => {
                assert_eq!(request_id, expected_request_id);
                assert!(!ok);
                assert!(result.is_none());
                return error.expect("rejected response includes an error");
            }
            WireMessage::Event { .. } => {}
            WireMessage::Request { .. } => panic!("Core sent an unexpected request"),
        }
    }
}

async fn wait_for_state(
    transport: &mut NoiseTransport<TcpStream>,
    expected_state: InstanceState,
    observed_states: &mut Vec<InstanceState>,
) {
    timeout(Duration::from_secs(5), async {
        loop {
            let message = transport
                .read_message()
                .await
                .expect("Core state event is received");
            let state = record_state(message, observed_states);
            if state == Some(expected_state) {
                return;
            }
        }
    })
    .await
    .expect("Core publishes the expected state before the timeout");
}

fn record_state(
    message: WireMessage,
    observed_states: &mut Vec<InstanceState>,
) -> Option<InstanceState> {
    let WireMessage::Event { topic, data, .. } = message else {
        panic!("Core sent an unexpected response while waiting for an event");
    };
    assert_eq!(topic, "instance.state");
    let state = from_value(data["runtime"]["state"].clone())
        .expect("state event includes a valid runtime state");
    observed_states.push(state);

    Some(state)
}

fn instance_create(instance_id: &InstanceId) -> InstanceCreate {
    InstanceCreate::new(
        instance_id.clone(),
        "Safe process".to_owned(),
        InstanceKind::Paper,
        format!("instances/{instance_id}"),
        test_launch_config(),
    )
    .expect("test instance is valid")
}

fn crashing_process_create(instance_id: &InstanceId) -> InstanceCreate {
    InstanceCreate::new(
        instance_id.clone(),
        "Crashing process".to_owned(),
        InstanceKind::Paper,
        format!("instances/{instance_id}"),
        crashing_process_launch_config(),
    )
    .expect("test instance is valid")
}

#[cfg(windows)]
fn test_launch_config() -> LaunchConfig {
    LaunchConfig::new(
        "cmd.exe".to_owned(),
        vec![
            "/D".to_owned(),
            "/Q".to_owned(),
            "/C".to_owned(),
            "set /p line=".to_owned(),
        ],
        BTreeMap::new(),
        "stop".to_owned(),
        5,
    )
}

#[cfg(windows)]
fn crashing_process_launch_config() -> LaunchConfig {
    LaunchConfig::new(
        "cmd.exe".to_owned(),
        vec![
            "/D".to_owned(),
            "/Q".to_owned(),
            "/C".to_owned(),
            "exit /b 7".to_owned(),
        ],
        BTreeMap::new(),
        "stop".to_owned(),
        5,
    )
}

#[cfg(not(windows))]
fn test_launch_config() -> LaunchConfig {
    LaunchConfig::new(
        "/bin/sh".to_owned(),
        vec!["-c".to_owned(), "IFS= read -r line".to_owned()],
        BTreeMap::new(),
        "stop".to_owned(),
        5,
    )
}

#[cfg(not(windows))]
fn crashing_process_launch_config() -> LaunchConfig {
    LaunchConfig::new(
        "/bin/sh".to_owned(),
        vec!["-c".to_owned(), "exit 7".to_owned()],
        BTreeMap::new(),
        "stop".to_owned(),
        5,
    )
}
