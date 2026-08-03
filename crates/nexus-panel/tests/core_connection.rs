use std::collections::BTreeMap;
use std::slice::from_ref;
use std::time::Duration;

use nexus_config::CoreConfig;
use nexus_core::CoreServer;
use nexus_domain::BedrockManagementKind;
use nexus_domain::BedrockTransport;
use nexus_domain::FileKind;
use nexus_domain::InstanceCreate;
use nexus_domain::InstanceId;
use nexus_domain::InstanceKind;
use nexus_domain::InstanceLogPage;
use nexus_domain::InstanceLogStream;
use nexus_domain::InstanceState;
use nexus_domain::LaunchConfig;
use nexus_domain::ProxySubserver;
use nexus_domain::RequestId;
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
    let certificate_sha256 = server.certificate_sha256().to_owned();
    let server_task = spawn(server.serve());
    let pre_shared_key = PresharedKey::from_base64url(TEST_PSK).expect("test PSK is valid");
    let mut connection = CoreConnection::connect_address(
        &listen_address.to_string(),
        false,
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
    assert_eq!(connection.tls_certificate_sha256(), certificate_sha256);
    assert_eq!(created.revision(), 1);
    assert_eq!(instances.items().len(), 1);
    assert_eq!(instances.items().first(), Some(&created));
    assert_eq!(instances.next_cursor(), None);
    assert_eq!(fetched, created);

    let written = connection
        .write_instance_file(
            definition.id(),
            "server.properties",
            b"motd=MCNP",
            None,
            &RequestId::new().to_string(),
        )
        .await
        .expect("Core writes an instance file");
    assert_eq!(written.kind(), FileKind::File);
    assert_eq!(written.path(), "server.properties");
    let files = connection
        .list_instance_files(definition.id(), "", None, None)
        .await
        .expect("Core lists instance files");
    assert_eq!(files.items().len(), 1);
    assert_eq!(files.items()[0].path(), "server.properties");
    let content = connection
        .read_instance_file(definition.id(), "server.properties", 0, 4)
        .await
        .expect("Core reads an instance file chunk");
    assert_eq!(content.data_base64(), "bW90ZA==");
    assert!(!content.eof());
    let directory = connection
        .create_instance_directory(
            definition.id(),
            "config/server",
            true,
            &RequestId::new().to_string(),
        )
        .await
        .expect("Core creates a recursive instance directory");
    assert_eq!(directory.kind(), FileKind::Directory);
    let moved = connection
        .move_instance_file(
            definition.id(),
            "server.properties",
            "config/server/server.properties",
            false,
            &RequestId::new().to_string(),
        )
        .await
        .expect("Core moves an instance file");
    assert_eq!(moved.path(), "config/server/server.properties");
    let invalid_path = connection
        .list_instance_files(definition.id(), "../outside", None, None)
        .await
        .expect_err("Core rejects an escaping file path");
    assert!(matches!(
        invalid_path,
        CoreConnectionError::Rejected { code } if code == "BAD_REQUEST"
    ));

    let proxy = instance_create_with_kind("geyser", InstanceKind::Geyser);
    let first_target = instance_create_with_kind("java-backend", InstanceKind::Paper);
    let second_target = instance_create_with_kind("java-backend-two", InstanceKind::Paper);
    connection
        .create_instance(&proxy)
        .await
        .expect("Core creates the Geyser proxy");
    connection
        .create_instance(&first_target)
        .await
        .expect("Core creates the first proxy target");
    connection
        .create_instance(&second_target)
        .await
        .expect("Core creates the second proxy target");
    let profile = connection
        .get_bedrock_profile(proxy.id())
        .await
        .expect("Core returns the Geyser Bedrock profile");
    assert_eq!(profile.management_kind(), BedrockManagementKind::Geyser);
    assert_eq!(profile.transport(), BedrockTransport::RaknetUdp);
    assert_eq!(profile.default_port(), 19132);
    assert_eq!(profile.configuration_files(), ["config.yml"]);
    assert_eq!(profile.extension_kind(), None);

    let first_subserver = ProxySubserver::new(
        "default".to_owned(),
        "Default".to_owned(),
        first_target.id().clone(),
        "127.0.0.1".to_owned(),
        25565,
        true,
    )
    .expect("first proxy subserver is valid");
    connection
        .upsert_proxy_subserver(proxy.id(), &first_subserver, "proxy-upsert")
        .await
        .expect("Core accepts the Geyser target");
    let subservers = connection
        .list_proxy_subservers(proxy.id())
        .await
        .expect("Core lists Geyser targets");
    assert_eq!(subservers, from_ref(&first_subserver));

    let second_subserver = ProxySubserver::new(
        "secondary".to_owned(),
        "Secondary".to_owned(),
        second_target.id().clone(),
        "127.0.0.1".to_owned(),
        25566,
        true,
    )
    .expect("second proxy subserver is valid");
    let error = connection
        .upsert_proxy_subserver(proxy.id(), &second_subserver, "proxy-limit")
        .await
        .expect_err("Geyser rejects a second target");
    assert!(matches!(
        error,
        CoreConnectionError::Rejected { code } if code == "PROXY_SUBSERVER_LIMIT_REACHED"
    ));
    connection
        .delete_proxy_subserver(proxy.id(), "default", "proxy-delete")
        .await
        .expect("Core deletes the Geyser target");
    assert!(
        connection
            .list_proxy_subservers(proxy.id())
            .await
            .expect("Core lists the empty Geyser target set")
            .is_empty()
    );

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
    instance_create_with_kind(identifier, InstanceKind::Paper)
}

fn instance_create_with_kind(identifier: &str, kind: InstanceKind) -> InstanceCreate {
    InstanceCreate::new(
        InstanceId::new(identifier.to_owned()).expect("test identifier is valid"),
        identifier.to_owned(),
        kind,
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
