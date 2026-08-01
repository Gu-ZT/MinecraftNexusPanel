use std::collections::BTreeMap;
use std::time::Duration;

use nexus_config::CoreConfig;
use nexus_core::CoreServer;
use nexus_domain::InstanceCreate;
use nexus_domain::InstanceId;
use nexus_domain::InstanceKind;
use nexus_domain::InstanceLogPage;
use nexus_domain::InstanceLogStream;
use nexus_domain::InstanceState;
use nexus_domain::LaunchConfig;
use nexus_panel::CoreConnection;
use nexus_panel::CoreConnectionError;
use nexus_protocol::PresharedKey;
use tempfile::tempdir;
use tokio::spawn;
use tokio::time::sleep;
use tokio::time::timeout;

const TEST_PSK: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY";

#[tokio::test]
async fn connects_to_a_core_and_reads_its_system_info() {
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
    let mut connection = CoreConnection::connect(
        listen_address,
        &pre_shared_key,
        "test-panel-id",
        "test-panel",
    )
    .await
    .expect("Panel connects to Core");

    let received_at = connection.ping().await.expect("Core responds to ping");
    let system_info = connection
        .system_info()
        .await
        .expect("Core responds with system information");
    let definition = instance_create("survival");
    let created = connection
        .create_instance(&definition)
        .await
        .expect("Core creates the instance");
    let instances = connection
        .list_instances()
        .await
        .expect("Core lists instances");
    let fetched = connection
        .get_instance(definition.id())
        .await
        .expect("Core returns the instance");

    assert!(received_at.ends_with('Z'));
    assert!(connection.capabilities().contains(&"events".to_owned()));
    assert!(connection.capabilities().contains(&"instances".to_owned()));
    assert!(connection.capabilities().contains(&"metrics".to_owned()));
    assert_eq!(system_info["coreId"], connection.core_id().to_string());
    assert_eq!(connection.heartbeat_seconds(), 20);
    assert_eq!(created.revision(), 1);
    assert_eq!(instances.items().len(), 1);
    assert_eq!(instances.items().first(), Some(&created));
    assert_eq!(instances.next_cursor(), None);
    assert_eq!(fetched, created);

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn controls_an_instance_process_through_the_panel_client() {
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
    let mut connection = CoreConnection::connect(
        listen_address,
        &pre_shared_key,
        "test-panel-id",
        "test-panel",
    )
    .await
    .expect("Panel connects to Core");
    let definition = safe_process_create("safe-process");
    connection
        .create_instance(&definition)
        .await
        .expect("Core creates the safe process instance");

    connection
        .start_instance(definition.id(), "panel-start")
        .await
        .expect("Panel starts the instance");
    wait_for_state(&mut connection, definition.id(), InstanceState::Running).await;

    let metrics = connection
        .get_instance_metrics(definition.id(), Some("current"), Some("current"))
        .await
        .expect("Panel reads instance metrics");
    assert_eq!(metrics.len(), 1);

    let accepted_at = connection
        .send_instance_command(definition.id(), "say hello\n")
        .await
        .expect("Panel sends an instance command");
    assert!(accepted_at.ends_with('Z'));
    let logs = wait_for_logs(&mut connection, definition.id(), "received:say hello").await;
    assert!(
        logs.items()
            .iter()
            .any(|line| { line.stream() == InstanceLogStream::Stdout && line.line() == "ready" })
    );
    assert!(
        logs.items()
            .iter()
            .any(|line| { line.stream() == InstanceLogStream::Stderr && line.line() == "warning" })
    );

    let command_error = connection
        .send_instance_command(definition.id(), "\r\n")
        .await
        .expect_err("Panel rejects an empty instance command");
    assert!(matches!(
        command_error,
        CoreConnectionError::Rejected { code } if code == "BAD_REQUEST"
    ));

    connection
        .stop_instance(definition.id(), Some(5), "panel-stop")
        .await
        .expect("Panel stops the instance");
    wait_for_state(&mut connection, definition.id(), InstanceState::Stopped).await;

    connection
        .start_instance(definition.id(), "panel-restart")
        .await
        .expect("Panel starts the instance again");
    wait_for_state(&mut connection, definition.id(), InstanceState::Running).await;
    connection
        .kill_instance(definition.id(), "panel-kill")
        .await
        .expect("Panel kills the instance");
    wait_for_state(&mut connection, definition.id(), InstanceState::Stopped).await;

    server_task.abort();
    let _ = server_task.await;
}

async fn wait_for_state(
    connection: &mut CoreConnection,
    instance_id: &InstanceId,
    expected_state: InstanceState,
) {
    timeout(Duration::from_secs(5), async {
        loop {
            let instance = connection
                .get_instance(instance_id)
                .await
                .expect("Core returns the instance state");
            if instance.runtime().state() == expected_state {
                return;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("instance reaches the expected state before the timeout");
}

async fn wait_for_logs(
    connection: &mut CoreConnection,
    instance_id: &InstanceId,
    expected_line: &str,
) -> InstanceLogPage {
    timeout(Duration::from_secs(5), async {
        loop {
            let page = connection
                .get_instance_logs(instance_id, None, None, Some(200))
                .await
                .expect("Panel reads instance logs");
            if page.items().iter().any(|line| line.line() == expected_line) {
                return page;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("Panel observes the expected log line before the timeout")
}

fn instance_create(identifier: &str) -> InstanceCreate {
    InstanceCreate::new(
        InstanceId::new(identifier.to_owned()).expect("test identifier is valid"),
        identifier.to_owned(),
        InstanceKind::Paper,
        format!("instances/{identifier}"),
        LaunchConfig::new(
            "java".to_owned(),
            vec!["-jar".to_owned(), "paper.jar".to_owned()],
            BTreeMap::new(),
            "stop".to_owned(),
            30,
        ),
    )
    .expect("test instance is valid")
}

fn safe_process_create(identifier: &str) -> InstanceCreate {
    InstanceCreate::new(
        InstanceId::new(identifier.to_owned()).expect("test identifier is valid"),
        identifier.to_owned(),
        InstanceKind::Paper,
        format!("instances/{identifier}"),
        safe_process_launch_config(),
    )
    .expect("test instance is valid")
}

#[cfg(windows)]
fn safe_process_launch_config() -> LaunchConfig {
    LaunchConfig::new(
        "powershell.exe".to_owned(),
        vec![
            "-NoLogo".to_owned(),
            "-NoProfile".to_owned(),
            "-NonInteractive".to_owned(),
            "-Command".to_owned(),
            "$ErrorActionPreference='Stop'; [Console]::Out.WriteLine('ready'); [Console]::Error.WriteLine('warning'); while (($line = [Console]::In.ReadLine()) -ne $null) { if ($line -eq 'stop') { exit 0 }; [Console]::Out.WriteLine(\"received:$line\") }".to_owned(),
        ],
        BTreeMap::new(),
        "stop".to_owned(),
        5,
    )
}

#[cfg(not(windows))]
fn safe_process_launch_config() -> LaunchConfig {
    LaunchConfig::new(
        "/bin/sh".to_owned(),
        vec![
            "-c".to_owned(),
            "printf 'ready\\n'; printf 'warning\\n' >&2; while IFS= read -r line; do if [ \"$line\" = stop ]; then exit 0; fi; printf 'received:%s\\n' \"$line\"; done".to_owned(),
        ],
        BTreeMap::new(),
        "stop".to_owned(),
        5,
    )
}
