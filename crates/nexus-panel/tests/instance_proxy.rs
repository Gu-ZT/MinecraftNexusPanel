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
use serde_json::from_slice;
use serde_json::from_str;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;
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

    let templates = send_json_request(
        panel_address,
        "GET",
        "/api/v1/install-templates",
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(templates.status, 200);
    assert_eq!(templates.body["items"].as_array().map(Vec::len), Some(29));
    assert_eq!(templates.body["items"][0]["id"], "vanilla");
    assert_eq!(templates.body["items"][3]["id"], "fabric");
    assert_eq!(templates.body["items"][12]["id"], "mohist");
    assert_eq!(
        templates.body["items"][12]["extensionLayouts"][0]["kind"],
        "PLUGIN"
    );
    assert_eq!(
        templates.body["items"][12]["extensionLayouts"][1]["kind"],
        "MOD"
    );
    assert_eq!(templates.body["items"][24]["id"], "geyser");
    assert_eq!(templates.body["items"][24]["proxyTopology"], "ONE_TO_ONE");

    let missing_template = send_json_request(
        panel_address,
        "GET",
        "/api/v1/install-templates/missing/versions",
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(missing_template.status, 404);
    assert_eq!(missing_template.body["error"]["code"], "NOT_FOUND");

    let core_id = register_core(panel_address, &authorization, core_address).await;

    let runtimes = send_json_request(
        panel_address,
        "GET",
        &format!("/api/v1/cores/{core_id}/environments"),
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(runtimes.status, 200);
    assert!(runtimes.body["items"].is_array());

    for (identifier, kind) in [
        ("geyser-proxy", "GEYSER"),
        ("java-backend", "PAPER"),
        ("java-backend-two", "PAPER"),
    ] {
        let response = send_json_request(
            panel_address,
            "POST",
            &format!("/api/v1/cores/{core_id}/instances"),
            &[
                ("Authorization", authorization.as_str()),
                ("Idempotency-Key", &RequestId::new().to_string()),
            ],
            Some(instance_create_with_kind(identifier, kind)),
        )
        .await;
        assert_eq!(response.status, 201);
    }

    let bedrock_profile = send_json_request(
        panel_address,
        "GET",
        &format!("/api/v1/cores/{core_id}/instances/geyser-proxy/bedrock-profile"),
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(bedrock_profile.status, 200);
    assert_eq!(bedrock_profile.body["managementKind"], "GEYSER");
    assert_eq!(bedrock_profile.body["transport"], "RAKNET_UDP");
    assert_eq!(bedrock_profile.body["defaultPort"], 19132);
    assert_eq!(bedrock_profile.body["configurationFiles"][0], "config.yml");
    assert!(bedrock_profile.body["extensionKind"].is_null());

    let proxy_path = format!("/api/v1/cores/{core_id}/instances/geyser-proxy/proxy-subservers");
    let first_subserver = send_json_request(
        panel_address,
        "POST",
        &proxy_path,
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        Some(json!({
            "id": "default",
            "name": "Default",
            "targetInstanceId": "java-backend",
            "host": "127.0.0.1",
            "port": 25565,
            "enabled": true,
        })),
    )
    .await;
    assert_eq!(first_subserver.status, 200);
    assert_eq!(first_subserver.body["targetInstanceId"], "java-backend");

    let listed_subservers = send_json_request(
        panel_address,
        "GET",
        &proxy_path,
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(listed_subservers.status, 200);
    assert_eq!(
        listed_subservers.body["items"].as_array().map(Vec::len),
        Some(1)
    );

    let second_subserver = send_json_request(
        panel_address,
        "POST",
        &proxy_path,
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        Some(json!({
            "id": "secondary",
            "name": "Secondary",
            "targetInstanceId": "java-backend-two",
            "host": "127.0.0.1",
            "port": 25566,
            "enabled": true,
        })),
    )
    .await;
    assert_eq!(second_subserver.status, 409);
    assert_eq!(
        second_subserver.body["error"]["code"],
        "PROXY_SUBSERVER_LIMIT_REACHED"
    );

    let deleted_subserver = send_json_request(
        panel_address,
        "DELETE",
        &format!("{proxy_path}/default"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        None,
    )
    .await;
    assert_eq!(deleted_subserver.status, 204);

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

    let listed_files = send_json_request(
        panel_address,
        "GET",
        &format!("/api/v1/cores/{core_id}/instances/panel-process/files?path="),
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(listed_files.status, 200);
    assert_eq!(listed_files.body["items"].as_array().map(Vec::len), Some(0));

    let written_file = send_raw_request(
        panel_address,
        "PUT",
        &format!(
            "/api/v1/cores/{core_id}/instances/panel-process/file-content?path=server.properties"
        ),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        b"motd=Panel",
    )
    .await;
    assert_eq!(written_file.status, 200);
    let written_file_body: Value =
        from_slice(&written_file.body).expect("file write response is JSON");
    let file_etag = written_file
        .headers
        .get("etag")
        .cloned()
        .expect("file write returns an ETag");
    assert_eq!(written_file_body["kind"], "FILE");
    assert_eq!(written_file_body["path"], "server.properties");

    let read_file = send_raw_request(
        panel_address,
        "GET",
        &format!(
            "/api/v1/cores/{core_id}/instances/panel-process/file-content?path=server.properties&offset=0&length=32"
        ),
        &[("Authorization", authorization.as_str())],
        &[],
    )
    .await;
    assert_eq!(read_file.status, 200);
    assert_eq!(read_file.body, b"motd=Panel");
    assert_eq!(read_file.headers.get("etag"), Some(&file_etag));
    assert_eq!(
        read_file.headers.get("x-mcnp-file-eof"),
        Some(&"true".to_owned())
    );

    let stale_file = send_raw_request(
        panel_address,
        "PUT",
        &format!(
            "/api/v1/cores/{core_id}/instances/panel-process/file-content?path=server.properties"
        ),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
            ("If-Match", &format!("\"{}\"", "0".repeat(64))),
        ],
        b"motd=Stale",
    )
    .await;
    assert_eq!(stale_file.status, 412);

    let created_directory = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/instances/panel-process/directories"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        Some(json!({ "path": "config/server", "recursive": true })),
    )
    .await;
    assert_eq!(created_directory.status, 200);
    assert_eq!(created_directory.body["kind"], "DIRECTORY");
    assert_eq!(created_directory.body["path"], "config/server");

    let moved_file = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/instances/panel-process/file-actions/move"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        Some(json!({
            "from": "server.properties",
            "to": "config/server/server.properties",
            "overwrite": false,
        })),
    )
    .await;
    assert_eq!(moved_file.status, 200);
    assert_eq!(moved_file.body["kind"], "FILE");
    assert_eq!(moved_file.body["path"], "config/server/server.properties");

    let batch_file_action = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/instances/panel-process/file-actions/batch"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        Some(json!({
            "operations": [
                { "kind": "MKDIR", "path": "batch", "recursive": true },
                {
                    "kind": "WRITE",
                    "path": "batch/source.txt",
                    "dataBase64": "YmF0Y2g="
                },
                {
                    "kind": "MOVE",
                    "from": "batch/source.txt",
                    "to": "batch/renamed.txt"
                },
                {
                    "kind": "DELETE",
                    "path": "batch/renamed.txt",
                    "confirmation": "DELETE"
                },
                {
                    "kind": "DELETE",
                    "path": "batch",
                    "recursive": true,
                    "confirmation": "DELETE"
                }
            ]
        })),
    )
    .await;
    assert_eq!(batch_file_action.status, 202);
    let batch_task_id = batch_file_action.body["taskId"]
        .as_str()
        .expect("batch file task ID is returned")
        .to_owned();
    let batch_task =
        wait_for_file_task(panel_address, &authorization, &core_id, &batch_task_id).await;
    assert_eq!(batch_task["kind"], "FILE_BATCH");
    assert_eq!(batch_task["state"], "SUCCEEDED");
    assert_eq!(batch_task["progress"]["completed"], 5);
    assert_eq!(batch_task["results"].as_array().map(Vec::len), Some(5));

    let upload_content = b"panel chunked upload";
    let upload_sha256 = sha256_hex(upload_content);
    let started_upload = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/instances/panel-process/uploads"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        Some(json!({
            "path": "config/server/uploaded.properties",
            "sizeBytes": upload_content.len(),
            "sha256": upload_sha256.clone(),
        })),
    )
    .await;
    assert_eq!(started_upload.status, 201);
    assert_eq!(started_upload.body["chunkSize"], 1024 * 1024);
    let transfer_id = started_upload.body["transferId"]
        .as_str()
        .expect("upload transfer ID is returned")
        .to_owned();
    let uploaded_part = send_raw_request(
        panel_address,
        "PUT",
        &format!("/api/v1/cores/{core_id}/uploads/{transfer_id}/parts/0"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
            ("Content-SHA256", &upload_sha256),
        ],
        upload_content,
    )
    .await;
    assert_eq!(uploaded_part.status, 200);
    let uploaded_part_body: Value =
        from_slice(&uploaded_part.body).expect("upload part response is JSON");
    assert_eq!(uploaded_part_body["nextOffset"], upload_content.len());
    let completed_upload = send_json_request(
        panel_address,
        "POST",
        &format!("/api/v1/cores/{core_id}/uploads/{transfer_id}/complete"),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        None,
    )
    .await;
    assert_eq!(completed_upload.status, 200);
    assert_eq!(
        completed_upload.body["path"],
        "config/server/uploaded.properties"
    );

    let non_recursive_delete = send_json_request(
        panel_address,
        "DELETE",
        &format!(
            "/api/v1/cores/{core_id}/instances/panel-process/files?path=config/server/server.properties&confirmation=DELETE"
        ),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        None,
    )
    .await;
    assert_eq!(non_recursive_delete.status, 202);
    let delete_task_id = non_recursive_delete.body["taskId"]
        .as_str()
        .expect("file deletion task ID is returned")
        .to_owned();
    let delete_task =
        wait_for_file_task(panel_address, &authorization, &core_id, &delete_task_id).await;
    assert_eq!(delete_task["state"], "SUCCEEDED");
    assert_eq!(delete_task["deleted"], true);

    let nested_file = send_raw_request(
        panel_address,
        "PUT",
        &format!(
            "/api/v1/cores/{core_id}/instances/panel-process/file-content?path=config/server/nested.properties"
        ),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        b"nested",
    )
    .await;
    assert_eq!(nested_file.status, 200);

    let rejected_non_recursive_delete = send_json_request(
        panel_address,
        "DELETE",
        &format!(
            "/api/v1/cores/{core_id}/instances/panel-process/files?path=config/server&confirmation=DELETE"
        ),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        None,
    )
    .await;
    assert_eq!(rejected_non_recursive_delete.status, 409);
    assert_eq!(
        rejected_non_recursive_delete.body["error"]["code"],
        "FILE_DIRECTORY_NOT_EMPTY"
    );

    let recursive_delete = send_json_request(
        panel_address,
        "DELETE",
        &format!(
            "/api/v1/cores/{core_id}/instances/panel-process/files?path=config/server&confirmation=DELETE&recursive=true"
        ),
        &[
            ("Authorization", authorization.as_str()),
            ("Idempotency-Key", &RequestId::new().to_string()),
        ],
        None,
    )
    .await;
    assert_eq!(recursive_delete.status, 202);
    let recursive_task_id = recursive_delete.body["taskId"]
        .as_str()
        .expect("recursive deletion task ID is returned")
        .to_owned();
    let recursive_task =
        wait_for_file_task(panel_address, &authorization, &core_id, &recursive_task_id).await;
    assert_eq!(recursive_task["state"], "SUCCEEDED");
    assert_eq!(recursive_task["path"], "config/server");

    let invalid_file = send_json_request(
        panel_address,
        "GET",
        &format!("/api/v1/cores/{core_id}/instances/panel-process/files?path=../outside"),
        &[("Authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(invalid_file.status, 400);
    assert_eq!(invalid_file.body["error"]["code"], "VALIDATION_FAILED");

    let updated = send_json_request(
        panel_address,
        "PATCH",
        &format!("/api/v1/cores/{core_id}/instances/panel-process"),
        &[
            ("Authorization", authorization.as_str()),
            ("If-Match", "\"1\""),
        ],
        Some(json!({
            "name": "Configured Panel Process",
            "directory": "instances/configured-panel-process",
            "updateCommand": "./update.sh",
            "expiresAt": "2030-01-01T00:00:00Z",
        })),
    )
    .await;
    assert_eq!(updated.status, 200);
    assert_eq!(updated.body["name"], "Configured Panel Process");
    assert_eq!(
        updated.body["directory"],
        "instances/configured-panel-process"
    );
    assert_eq!(updated.body["updateCommand"], "./update.sh");
    assert_eq!(updated.body["expiresAt"], "2030-01-01T00:00:00Z");
    assert_eq!(
        updated.headers.get("etag").map(String::as_str),
        Some("\"2\"")
    );

    let stale_update = send_json_request(
        panel_address,
        "PATCH",
        &format!("/api/v1/cores/{core_id}/instances/panel-process"),
        &[
            ("Authorization", authorization.as_str()),
            ("If-Match", "\"1\""),
        ],
        Some(json!({ "name": "Stale Settings" })),
    )
    .await;
    assert_eq!(stale_update.status, 412);
    assert_eq!(stale_update.body["error"]["code"], "REVISION_MISMATCH");

    let cleared = send_json_request(
        panel_address,
        "PATCH",
        &format!("/api/v1/cores/{core_id}/instances/panel-process"),
        &[
            ("Authorization", authorization.as_str()),
            ("If-Match", "\"2\""),
        ],
        Some(json!({ "updateCommand": null, "expiresAt": null })),
    )
    .await;
    assert_eq!(cleared.status, 200);
    assert!(cleared.body["updateCommand"].is_null());
    assert!(cleared.body["expiresAt"].is_null());
    assert_eq!(
        cleared.headers.get("etag").map(String::as_str),
        Some("\"3\"")
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
    let panel_process = listed.body["items"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["id"] == "panel-process"))
        .expect("instance list contains panel-process");
    assert_eq!(panel_process["coreId"], core_id);

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

    let running_update = send_json_request(
        panel_address,
        "PATCH",
        &format!("/api/v1/cores/{core_id}/instances/panel-process"),
        &[
            ("Authorization", authorization.as_str()),
            ("If-Match", "\"3\""),
        ],
        Some(json!({ "name": "Unsafe Running Update" })),
    )
    .await;
    assert_eq!(running_update.status, 409);
    assert_eq!(
        running_update.body["error"]["code"],
        "INSTANCE_STATE_CONFLICT"
    );

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

async fn wait_for_file_task(
    address: SocketAddr,
    authorization: &str,
    core_id: &str,
    task_id: &str,
) -> Value {
    timeout(Duration::from_secs(5), async {
        loop {
            let response = send_json_request(
                address,
                "GET",
                &format!("/api/v1/cores/{core_id}/file-tasks/{task_id}"),
                &[("Authorization", authorization)],
                None,
            )
            .await;
            assert_eq!(response.status, 200);
            if response.body["state"] != "RUNNING" {
                return response.body;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("file task finishes before the timeout")
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

async fn send_raw_request(
    address: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> TestRawHttpResponse {
    let request_id = RequestId::new();
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nX-Request-Id: {request_id}\r\nConnection: close\r\n"
    );
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    if !body.is_empty() {
        request.push_str("Content-Type: application/octet-stream\r\n");
    }
    request.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));

    let mut stream = TcpStream::connect(address)
        .await
        .expect("HTTP client connects");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("HTTP request headers are sent");
    stream
        .write_all(body)
        .await
        .expect("HTTP request body is sent");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("HTTP response is read");

    TestRawHttpResponse::parse(&response)
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

fn sha256_hex(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn instance_create_with_kind(identifier: &str, kind: &str) -> Value {
    let mut definition = safe_process_create(identifier);
    definition["kind"] = json!(kind);
    definition
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

struct TestRawHttpResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl TestRawHttpResponse {
    fn parse(response: &[u8]) -> Self {
        let boundary = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("HTTP response has a header boundary");
        let head = String::from_utf8_lossy(&response[..boundary]);
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

        Self {
            status,
            headers,
            body: response[boundary + 4..].to_vec(),
        }
    }
}
