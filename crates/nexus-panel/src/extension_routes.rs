use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path as FilePath;
use std::path::PathBuf;
use std::time::Duration;

use axum::Extension;
use axum::Json;
use axum::Router;
use axum::body::Bytes;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use nexus_domain::CoreId;
use nexus_domain::ExtensionArtifact;
use nexus_domain::ExtensionInstall;
use nexus_domain::ExtensionInstallRequest;
use nexus_domain::ExtensionKind;
use nexus_domain::ExtensionPlanItem;
use nexus_domain::ExtensionPlanRequest;
use nexus_domain::ExtensionPlanResolution;
use nexus_domain::FileEntry;
use nexus_domain::FilePage;
use nexus_domain::Instance;
use nexus_domain::InstanceId;
use nexus_domain::RequestId;
use nexus_domain::TaskId;
use serde_json::Value;
use serde_json::from_value;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;
use sha2::Sha512;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::fs::File;
use tokio::fs::remove_file;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::spawn;
use tokio::time::sleep;

use crate::CoreConnectionError;
use crate::CoreRegistryError;
use crate::PanelState;
use crate::auth_routes::error_response;
use crate::auth_routes::header_text;
use crate::core_routes::authorize;
use crate::core_routes::invalid_core_id_response;
use crate::core_routes::parse_core_id;
use crate::core_routes::registry_error_response;
use crate::extension_source_client::ExtensionSourceClient;
use crate::extension_source_client::MAXIMUM_ARTIFACT_BYTES;
use crate::extension_source_error::ExtensionSourceError;
use crate::install_template_catalog::install_template;

const EXTENSION_DIRECTORY_LIST_LIMIT: usize = 200;
const MAXIMUM_EXTENSION_WRITE_BYTES: usize = 1024 * 1024;
const EXTENSION_TRANSFER_CHUNK_BYTES: u64 = 1024 * 1024;

struct StagedArtifact {
    path: PathBuf,
    size: u64,
    sha256: String,
}

pub(crate) fn extension_routes() -> Router<PanelState> {
    Router::new()
        .route(
            "/api/v1/extension-catalog/search",
            get(search_extension_catalog),
        )
        .route(
            "/api/v1/extension-catalog/projects/{source}/{project_id}",
            get(get_extension_project_versions),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/extension-plans:resolve",
            post(resolve_extension_plan),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/extensions",
            get(list_instance_extensions)
                .post(install_instance_extensions)
                .put(write_instance_extension)
                .delete(delete_instance_extension),
        )
        .route(
            "/api/v1/cores/{core_id}/instances/{instance_id}/extensions/{extension_id}/actions/update",
            post(update_instance_extension),
        )
        .route(
            "/api/v1/cores/{core_id}/extension-tasks/{task_id}",
            get(get_extension_install_task),
        )
}

async fn install_instance_extensions(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    headers: HeaderMap,
    payload: Result<Json<ExtensionInstallRequest>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(idempotency_key) =
        header_text(&headers, "idempotency-key").filter(|value| value.parse::<RequestId>().is_ok())
    else {
        return error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "PRECONDITION_REQUIRED",
            "Idempotency-Key is required",
            request_id,
        );
    };
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(instance_id) = instance_id.parse::<InstanceId>().ok() else {
        return validation_error(request_id);
    };
    let request = match payload {
        Ok(Json(request)) if is_valid_plan_request(request.plan()) => request,
        Ok(_) | Err(_) => return validation_error(request_id),
    };
    let plan = request.plan();
    let Some(template) = install_template(plan.template_id()) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Install template does not exist",
            request_id,
        );
    };
    let instance = match state.cores().get_instance(core_id, &instance_id).await {
        Ok(value) => match from_value::<Instance>(value) {
            Ok(instance) => instance,
            Err(_) => return invalid_core_response(request_id),
        },
        Err(error) => return registry_error_response(error, request_id),
    };
    if instance.kind() != template.instance_kind() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TEMPLATE_INSTANCE_KIND_MISMATCH",
            "Install template does not match the instance type",
            request_id,
        );
    }

    let directories = template.extension_directories(plan.kind());
    let Some(directory) = selected_extension_directory(&directories, request.directory()) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_DIRECTORY_INVALID",
            "The selected directory is not declared for this extension kind",
            request_id,
        );
    };
    let plan = match state
        .extension_sources()
        .resolve_dependencies(
            template.id(),
            plan.kind(),
            plan.project_id(),
            plan.version_id(),
            plan.minecraft_version(),
            plan.loader(),
        )
        .await
    {
        Ok(plan) => plan,
        Err(error) => return extension_source_error_response(error, request_id),
    };
    let mut paths = HashSet::with_capacity(plan.items().len());
    for item in plan.items() {
        let Some(path) = extension_install_path(directory, item) else {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "EXTENSION_ARTIFACT_INVALID",
                "The resolved extension artifact has an invalid file name",
                request_id,
            );
        };
        if !paths.insert(path) {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "EXTENSION_ARTIFACT_CONFLICT",
                "Resolved extensions would overwrite the same file",
                request_id,
            );
        }
    }

    let (task_id, created) = match state.extension_tasks().start(
        core_id,
        &instance_id,
        plan.kind(),
        plan.items().len(),
        "EXTENSION_INSTALL",
        idempotency_key,
    ) {
        Ok(result) => result,
        Err(()) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "TASK_STORE_UNAVAILABLE",
                "The extension task store cannot accept more tasks",
                request_id,
            );
        }
    };
    if created {
        let task_state = state.clone();
        let task_directory = directory.to_owned();
        let task_instance_id = instance_id.clone();
        let task_idempotency_key = idempotency_key.to_owned();
        spawn(async move {
            run_extension_install_task(
                task_state,
                task_id,
                core_id,
                task_instance_id,
                task_directory,
                plan,
                None,
                None,
                task_idempotency_key,
            )
            .await;
        });
    }

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "taskId": task_id,
            "acceptedAt": current_timestamp(),
        })),
    )
        .into_response()
}

async fn update_instance_extension(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id, extension_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    payload: Result<Json<ExtensionInstallRequest>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(idempotency_key) =
        header_text(&headers, "idempotency-key").filter(|value| value.parse::<RequestId>().is_ok())
    else {
        return error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "PRECONDITION_REQUIRED",
            "Idempotency-Key is required",
            request_id,
        );
    };
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(instance_id) = instance_id.parse::<InstanceId>().ok() else {
        return validation_error(request_id);
    };
    let request = match payload {
        Ok(Json(request)) if is_valid_plan_request(request.plan()) => request,
        Ok(_) | Err(_) => return validation_error(request_id),
    };
    let plan_request = request.plan();
    let Some(template) = install_template(plan_request.template_id()) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Install template does not exist",
            request_id,
        );
    };
    let instance = match state.cores().get_instance(core_id, &instance_id).await {
        Ok(value) => match from_value::<Instance>(value) {
            Ok(instance) => instance,
            Err(_) => return invalid_core_response(request_id),
        },
        Err(error) => return registry_error_response(error, request_id),
    };
    if instance.kind() != template.instance_kind() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TEMPLATE_INSTANCE_KIND_MISMATCH",
            "Install template does not match the instance type",
            request_id,
        );
    }

    let directories = template.extension_directories(plan_request.kind());
    if directories.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_KIND_UNSUPPORTED",
            "The install template does not declare this extension kind",
            request_id,
        );
    }
    let installations = match state
        .cores()
        .list_extension_installs(core_id, &instance_id, plan_request.kind())
        .await
    {
        Ok(installations) => installations,
        Err(error) => return registry_error_response(error, request_id),
    };
    let Some(existing) = installations
        .into_iter()
        .find(|installation| installation.id() == extension_id)
    else {
        return error_response(
            StatusCode::NOT_FOUND,
            "EXTENSION_NOT_FOUND",
            "Extension installation does not exist",
            request_id,
        );
    };
    let Some(existing_project_id) = existing.project_id() else {
        return error_response(
            StatusCode::CONFLICT,
            "EXTENSION_UPDATE_UNSUPPORTED",
            "Local extension artifacts cannot be updated from a source plan",
            request_id,
        );
    };
    if existing_project_id != plan_request.project_id() {
        return error_response(
            StatusCode::CONFLICT,
            "EXTENSION_PROJECT_MISMATCH",
            "The update plan does not belong to the selected extension",
            request_id,
        );
    }
    let Some(existing_directory) = directories
        .iter()
        .copied()
        .find(|directory| is_extension_path(existing.path(), directory))
    else {
        return error_response(
            StatusCode::CONFLICT,
            "EXTENSION_PATH_OUTSIDE_LAYOUT",
            "The recorded extension path is outside the template layout",
            request_id,
        );
    };
    let directory = match request.directory() {
        Some(requested) if requested != existing_directory => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "EXTENSION_DIRECTORY_INVALID",
                "The update directory must match the recorded extension directory",
                request_id,
            );
        }
        Some(requested) => selected_extension_directory(&directories, Some(requested)),
        None => Some(existing_directory),
    };
    let Some(directory) = directory else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_DIRECTORY_INVALID",
            "The selected directory is not declared for this extension kind",
            request_id,
        );
    };
    let expected_sha256 = match expected_file_hash(&headers) {
        Ok(Some(value)) if !value.eq_ignore_ascii_case(existing.sha256()) => {
            return error_response(
                StatusCode::PRECONDITION_FAILED,
                "EXTENSION_REVISION_MISMATCH",
                "The extension installation record has changed",
                request_id,
            );
        }
        Ok(_) => Some(existing.sha256().to_owned()),
        Err(()) => return validation_error(request_id),
    };
    let resolved_plan = match state
        .extension_sources()
        .resolve_dependencies(
            template.id(),
            plan_request.kind(),
            plan_request.project_id(),
            plan_request.version_id(),
            plan_request.minecraft_version(),
            plan_request.loader(),
        )
        .await
    {
        Ok(plan) => plan,
        Err(error) => return extension_source_error_response(error, request_id),
    };
    let Some(root_item) = resolved_plan
        .items()
        .iter()
        .find(|item| item.project_id() == existing_project_id)
        .cloned()
    else {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "EXTENSION_PLAN_UNRESOLVED",
            "The update plan does not contain the selected extension",
            request_id,
        );
    };
    let update_plan = ExtensionPlanResolution::new(
        resolved_plan.template_id().to_owned(),
        resolved_plan.kind(),
        resolved_plan.minecraft_version().to_owned(),
        resolved_plan.loader().map(str::to_owned),
        vec![root_item],
    );
    let (task_id, created) = match state.extension_tasks().start(
        core_id,
        &instance_id,
        update_plan.kind(),
        update_plan.items().len(),
        "EXTENSION_UPDATE",
        idempotency_key,
    ) {
        Ok(result) => result,
        Err(()) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "TASK_STORE_UNAVAILABLE",
                "The extension task store cannot accept more tasks",
                request_id,
            );
        }
    };
    if created {
        let task_state = state.clone();
        let task_directory = directory.to_owned();
        let task_instance_id = instance_id.clone();
        let target_path = existing.path().to_owned();
        let task_expected_sha256 = expected_sha256;
        let task_idempotency_key = idempotency_key.to_owned();
        spawn(async move {
            run_extension_install_task(
                task_state,
                task_id,
                core_id,
                task_instance_id,
                task_directory,
                update_plan,
                Some(target_path),
                task_expected_sha256,
                task_idempotency_key,
            )
            .await;
        });
    }

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "taskId": task_id,
            "acceptedAt": current_timestamp(),
        })),
    )
        .into_response()
}

async fn get_extension_install_task(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, task_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Ok(task_id) = task_id.parse::<TaskId>() else {
        return validation_error(request_id);
    };
    let task = match state.extension_tasks().get(task_id) {
        Ok(Some(task)) => task,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "EXTENSION_TASK_NOT_FOUND",
                "Extension installation task does not exist",
                request_id,
            );
        }
        Err(()) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "TASK_STORE_UNAVAILABLE",
                "The extension task store is unavailable",
                request_id,
            );
        }
    };
    let core_id_text = core_id.to_string();
    if task.get("coreId").and_then(Value::as_str) != Some(core_id_text.as_str()) {
        return error_response(
            StatusCode::NOT_FOUND,
            "EXTENSION_TASK_NOT_FOUND",
            "Extension installation task does not exist",
            request_id,
        );
    }

    Json(task).into_response()
}

#[allow(clippy::too_many_arguments)]
async fn run_extension_install_task(
    state: PanelState,
    task_id: TaskId,
    core_id: CoreId,
    instance_id: InstanceId,
    directory: String,
    plan: ExtensionPlanResolution,
    target_path: Option<String>,
    expected_sha256: Option<String>,
    idempotency_key: String,
) {
    let directory_idempotency_key = RequestId::new().to_string();
    if let Err(error) = state
        .cores()
        .create_instance_directory(
            core_id,
            &instance_id,
            &directory,
            true,
            &directory_idempotency_key,
        )
        .await
    {
        fail_extension_task(&state, task_id, 0, &[], error.to_string());
        return;
    }

    let total = plan.items().len();
    let mut installations = Vec::with_capacity(total);
    for (index, item) in plan.items().iter().enumerate() {
        let path = if index == 0 {
            target_path
                .clone()
                .or_else(|| extension_install_path(&directory, item))
        } else {
            extension_install_path(&directory, item)
        };
        let Some(path) = path else {
            fail_extension_task(
                &state,
                task_id,
                index,
                &installations,
                "The resolved extension artifact has an invalid file name".to_owned(),
            );
            return;
        };
        let transfer_idempotency_key = if index == 0 {
            idempotency_key.clone()
        } else {
            RequestId::new().to_string()
        };
        match install_extension_artifact(
            &state,
            core_id,
            &instance_id,
            &path,
            plan.kind(),
            item,
            if index == 0 {
                expected_sha256.as_deref()
            } else {
                None
            },
            &transfer_idempotency_key,
        )
        .await
        {
            Ok(installation) => installations.push(installation),
            Err(error) => {
                fail_extension_task(&state, task_id, index, &installations, error);
                return;
            }
        }
        if let Err(error) = state
            .extension_tasks()
            .update_progress(task_id, index + 1, total)
        {
            tracing::error!(?error, %task_id, "Failed to update extension task progress");
        }
    }

    if let Err(error) = state.extension_tasks().complete(task_id, &installations) {
        tracing::error!(?error, %task_id, "Failed to complete extension task");
    }
}

fn fail_extension_task(
    state: &PanelState,
    task_id: TaskId,
    completed: usize,
    installations: &[ExtensionInstall],
    error: String,
) {
    tracing::warn!(%task_id, %error, "Extension installation task failed");
    if let Err(store_error) =
        state
            .extension_tasks()
            .fail(task_id, completed, installations, &error)
    {
        tracing::error!(?store_error, %task_id, "Failed to record extension task failure");
    }
}

#[allow(clippy::too_many_arguments)]
async fn install_extension_artifact(
    state: &PanelState,
    core_id: CoreId,
    instance_id: &InstanceId,
    path: &str,
    kind: ExtensionKind,
    item: &ExtensionPlanItem,
    expected_sha256: Option<&str>,
    idempotency_key: &str,
) -> Result<ExtensionInstall, String> {
    let temporary_path = temporary_artifact_path();
    let result = async {
        let staged =
            download_artifact_to_file(state.extension_sources(), item.artifact(), &temporary_path)
                .await?;
        let entry = upload_artifact_to_core(
            state,
            core_id,
            instance_id,
            path,
            &staged,
            expected_sha256,
            idempotency_key,
        )
        .await?;
        let Some(core_sha256) = entry.sha256() else {
            return Err("Core did not return an extension file hash".to_owned());
        };
        if !core_sha256.eq_ignore_ascii_case(&staged.sha256) {
            return Err("Core returned an unexpected extension file hash".to_owned());
        }

        let installation = ExtensionInstall::new(
            RequestId::new().to_string(),
            kind,
            path.to_owned(),
            staged.sha256,
            item.source().to_owned(),
            Some(item.project_id().to_owned()),
            Some(item.version_id().to_owned()),
            current_timestamp(),
        );
        state
            .cores()
            .upsert_extension_install(core_id, instance_id, &installation)
            .await
            .map_err(|error| error.to_string())?;
        Ok(installation)
    }
    .await;

    if let Err(error) = remove_file(&temporary_path).await {
        tracing::debug!(%error, path = %temporary_path.display(), "Failed to remove extension staging file");
    }

    result
}

async fn download_artifact_to_file(
    source: &ExtensionSourceClient,
    artifact: &ExtensionArtifact,
    path: &FilePath,
) -> Result<StagedArtifact, String> {
    let response = source
        .download_artifact(artifact)
        .await
        .map_err(|error| error.to_string())?;
    if response
        .content_length()
        .is_some_and(|length| length != artifact.size())
    {
        return Err("Extension artifact size does not match source metadata".to_owned());
    }

    let mut file = File::create(path)
        .await
        .map_err(|error| format!("failed to create extension staging file: {error}"))?;
    let mut sha256 = Sha256::new();
    let mut sha512 = Sha512::new();
    let mut size = 0_u64;
    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("failed to read extension artifact: {error}"))?
    {
        let chunk_size = u64::try_from(chunk.len())
            .map_err(|_| "extension artifact chunk size is invalid".to_owned())?;
        size = size
            .checked_add(chunk_size)
            .ok_or_else(|| "extension artifact size overflowed".to_owned())?;
        if size > artifact.size() || size > MAXIMUM_ARTIFACT_BYTES {
            return Err("Extension artifact exceeds its declared size".to_owned());
        }
        file.write_all(&chunk)
            .await
            .map_err(|error| format!("failed to stage extension artifact: {error}"))?;
        sha256.update(&chunk);
        sha512.update(&chunk);
    }
    file.flush()
        .await
        .map_err(|error| format!("failed to flush extension staging file: {error}"))?;

    if size != artifact.size() {
        return Err("Extension artifact size does not match source metadata".to_owned());
    }
    let actual_sha512 = digest_hex(sha512.finalize());
    if !actual_sha512.eq_ignore_ascii_case(artifact.sha512()) {
        return Err("Extension artifact SHA-512 does not match source metadata".to_owned());
    }

    Ok(StagedArtifact {
        path: path.to_path_buf(),
        size,
        sha256: digest_hex(sha256.finalize()),
    })
}

async fn upload_artifact_to_core(
    state: &PanelState,
    core_id: CoreId,
    instance_id: &InstanceId,
    path: &str,
    staged: &StagedArtifact,
    expected_sha256: Option<&str>,
    idempotency_key: &str,
) -> Result<FileEntry, String> {
    let start = state
        .cores()
        .begin_file_upload_with_expected(
            core_id,
            instance_id,
            path,
            staged.size,
            &staged.sha256,
            expected_sha256,
            idempotency_key,
        )
        .await
        .map_err(|error| error.to_string())?;
    let Some(transfer_id) = start
        .get("transferId")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<TaskId>().ok())
    else {
        return Err("Core did not return an extension transfer ID".to_owned());
    };

    let result =
        upload_artifact_chunks(state, core_id, &transfer_id, &staged.path, staged.size).await;
    let result = match result {
        Ok(()) => {
            let commit_idempotency_key = RequestId::new().to_string();
            state
                .cores()
                .commit_file_upload(core_id, &transfer_id, &commit_idempotency_key)
                .await
                .map_err(|error| error.to_string())
        }
        Err(error) => Err(error),
    };
    if result.is_err() {
        let abort_idempotency_key = RequestId::new().to_string();
        if let Err(error) = state
            .cores()
            .abort_file_upload(core_id, &transfer_id, &abort_idempotency_key)
            .await
        {
            tracing::error!(%error, %transfer_id, "Failed to abort extension upload");
        }
    }

    result
}

async fn upload_artifact_chunks(
    state: &PanelState,
    core_id: CoreId,
    transfer_id: &TaskId,
    temporary_path: &FilePath,
    size: u64,
) -> Result<(), String> {
    let mut file = File::open(temporary_path)
        .await
        .map_err(|error| format!("failed to open extension staging file: {error}"))?;
    let mut offset = 0_u64;
    while offset < size {
        let chunk_size = (size - offset).min(EXTENSION_TRANSFER_CHUNK_BYTES);
        let chunk_size = usize::try_from(chunk_size)
            .map_err(|_| "extension transfer chunk size is invalid".to_owned())?;
        let mut chunk = vec![0; chunk_size];
        file.read_exact(&mut chunk)
            .await
            .map_err(|error| format!("failed to read extension staging file: {error}"))?;
        let chunk_sha256 = digest_hex(Sha256::digest(&chunk));
        let idempotency_key = RequestId::new().to_string();
        state
            .cores()
            .upload_file_chunk(
                core_id,
                transfer_id,
                offset,
                &chunk,
                &chunk_sha256,
                &idempotency_key,
            )
            .await
            .map_err(|error| error.to_string())?;
        offset += u64::try_from(chunk_size)
            .map_err(|_| "extension transfer offset overflowed".to_owned())?;
    }
    if offset != size {
        return Err("Extension upload did not consume the complete artifact".to_owned());
    }
    Ok(())
}

fn is_valid_plan_request(request: &ExtensionPlanRequest) -> bool {
    !request.template_id().is_empty()
        && !request.project_id().is_empty()
        && !request.version_id().is_empty()
        && !request.minecraft_version().is_empty()
        && request.minecraft_version().chars().count() <= 64
        && request.loader().is_none_or(|loader| {
            !loader.is_empty() && loader.chars().count() <= 64 && !loader.contains('\0')
        })
}

fn selected_extension_directory<'a>(
    directories: &[&'a str],
    requested: Option<&str>,
) -> Option<&'a str> {
    match requested {
        Some(requested) => directories
            .iter()
            .copied()
            .find(|directory| *directory == requested),
        None if directories.len() == 1 => directories.first().copied(),
        None => None,
    }
}

fn extension_install_path(directory: &str, item: &ExtensionPlanItem) -> Option<String> {
    let file_name = item.artifact().file_name();
    if file_name.is_empty()
        || file_name == "."
        || file_name == ".."
        || file_name.chars().count() > 255
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains('\0')
    {
        return None;
    }
    Some(format!("{directory}/{file_name}"))
}

fn temporary_artifact_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("mcnp-extension-{}.part", TaskId::new()));
    path
}

fn digest_hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn resolve_extension_plan(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    headers: HeaderMap,
    payload: Result<Json<ExtensionPlanRequest>, JsonRejection>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(instance_id) = instance_id.parse::<InstanceId>().ok() else {
        return validation_error(request_id);
    };
    let request = match payload {
        Ok(Json(request)) if is_valid_plan_request(&request) => request,
        Ok(_) | Err(_) => return validation_error(request_id),
    };
    let Some(template) = install_template(request.template_id()) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Install template does not exist",
            request_id,
        );
    };
    if template.extension_directories(request.kind()).is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_KIND_UNSUPPORTED",
            "The install template does not declare this extension kind",
            request_id,
        );
    }

    let instance = match state.cores().get_instance(core_id, &instance_id).await {
        Ok(value) => match from_value::<Instance>(value) {
            Ok(instance) => instance,
            Err(_) => return invalid_core_response(request_id),
        },
        Err(error) => return registry_error_response(error, request_id),
    };
    if instance.kind() != template.instance_kind() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TEMPLATE_INSTANCE_KIND_MISMATCH",
            "Install template does not match the instance type",
            request_id,
        );
    }

    match state
        .extension_sources()
        .resolve_dependencies(
            template.id(),
            request.kind(),
            request.project_id(),
            request.version_id(),
            request.minecraft_version(),
            request.loader(),
        )
        .await
    {
        Ok(result) => Json(result).into_response(),
        Err(error) => extension_source_error_response(error, request_id),
    }
}

async fn search_extension_catalog(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some(search_text) = query
        .get("query")
        .map(String::as_str)
        .filter(|value| !value.is_empty() && value.chars().count() <= 128)
    else {
        return validation_error(request_id);
    };
    let Some(kind) = query
        .get("type")
        .and_then(|value| from_value::<ExtensionKind>(json!(value)).ok())
    else {
        return validation_error(request_id);
    };
    let source = query
        .get("source")
        .map(String::as_str)
        .unwrap_or("modrinth");
    if source != "modrinth" {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_SOURCE_UNSUPPORTED",
            "The requested extension source is not supported",
            request_id,
        );
    }
    let minecraft_version = match optional_search_parameter(&query, "minecraftVersion", 64) {
        Ok(value) => value,
        Err(()) => return validation_error(request_id),
    };
    let loader = match optional_search_parameter(&query, "loader", 64) {
        Ok(value) => value,
        Err(()) => return validation_error(request_id),
    };
    let limit = match query.get("limit") {
        None => 20,
        Some(value) => match value.parse::<usize>() {
            Ok(value @ 1..=50) => value,
            _ => return validation_error(request_id),
        },
    };
    let offset = match query.get("offset") {
        None => 0,
        Some(value) => match value.parse::<usize>() {
            Ok(value) if value <= 10_000 => value,
            _ => return validation_error(request_id),
        },
    };

    match state
        .extension_sources()
        .search(
            search_text,
            kind,
            minecraft_version.as_deref(),
            loader.as_deref(),
            limit,
            offset,
        )
        .await
    {
        Ok(result) => Json(result).into_response(),
        Err(error) => extension_source_error_response(error, request_id),
    }
}

fn optional_search_parameter(
    query: &HashMap<String, String>,
    key: &str,
    maximum_length: usize,
) -> Result<Option<String>, ()> {
    query
        .get(key)
        .map(|value| {
            if value.is_empty() || value.chars().count() > maximum_length || value.contains('\0') {
                Err(())
            } else {
                Ok(value.clone())
            }
        })
        .transpose()
}

async fn get_extension_project_versions(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((source, project_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    if source != "modrinth" {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_SOURCE_UNSUPPORTED",
            "The requested extension source is not supported",
            request_id,
        );
    }
    let minecraft_version = match optional_search_parameter(&query, "minecraftVersion", 64) {
        Ok(value) => value,
        Err(()) => return validation_error(request_id),
    };
    let loader = match optional_search_parameter(&query, "loader", 64) {
        Ok(value) => value,
        Err(()) => return validation_error(request_id),
    };

    match state
        .extension_sources()
        .list_versions(&project_id, minecraft_version.as_deref(), loader.as_deref())
        .await
    {
        Ok(result) => Json(result).into_response(),
        Err(error) => extension_source_error_response(error, request_id),
    }
}

fn extension_source_error_response(error: ExtensionSourceError, request_id: RequestId) -> Response {
    tracing::warn!(%error, %request_id, "Extension source lookup failed");

    if matches!(&error, ExtensionSourceError::InvalidRequest) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
            "Extension source request parameters are invalid",
            request_id,
        );
    }
    if matches!(
        &error,
        ExtensionSourceError::VersionNotFound { .. }
            | ExtensionSourceError::NoCompatibleVersion { .. }
            | ExtensionSourceError::NoArtifact { .. }
            | ExtensionSourceError::MissingDependencyProject { .. }
            | ExtensionSourceError::DependencyConflict { .. }
            | ExtensionSourceError::DependencyGraphTooLarge { .. }
    ) {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "EXTENSION_PLAN_UNRESOLVED",
            "Extension dependencies could not be resolved",
            request_id,
        );
    }

    error_response(
        StatusCode::BAD_GATEWAY,
        "EXTENSION_SOURCE_UNAVAILABLE",
        "Extension source metadata is unavailable",
        request_id,
    )
}

async fn list_instance_extensions(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, false, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(instance_id) = instance_id.parse::<InstanceId>().ok() else {
        return validation_error(request_id);
    };
    let Some(template_id) = query.get("templateId").filter(|value| !value.is_empty()) else {
        return validation_error(request_id);
    };
    let Some(kind) = query
        .get("kind")
        .and_then(|value| from_value::<ExtensionKind>(json!(value)).ok())
    else {
        return validation_error(request_id);
    };
    let Some(template) = install_template(template_id) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Install template does not exist",
            request_id,
        );
    };

    let instance = match state.cores().get_instance(core_id, &instance_id).await {
        Ok(value) => match from_value::<Instance>(value) {
            Ok(instance) => instance,
            Err(_) => return invalid_core_response(request_id),
        },
        Err(error) => return registry_error_response(error, request_id),
    };
    if instance.kind() != template.instance_kind() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TEMPLATE_INSTANCE_KIND_MISMATCH",
            "Install template does not match the instance type",
            request_id,
        );
    }

    let directories = template.extension_directories(kind);
    if directories.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_KIND_UNSUPPORTED",
            "The install template does not declare this extension kind",
            request_id,
        );
    }

    let mut directory_pages = Vec::with_capacity(directories.len());
    for directory in directories {
        let page = match state
            .cores()
            .list_instance_files(
                core_id,
                &instance_id,
                directory,
                None,
                Some(EXTENSION_DIRECTORY_LIST_LIMIT),
            )
            .await
        {
            Ok(page) => page,
            Err(error) if is_missing_directory(&error) => FilePage::new(Vec::new(), None),
            Err(error) => return registry_error_response(error, request_id),
        };
        directory_pages.push(json!({
            "path": directory,
            "page": page,
        }));
    }
    let installations = match state
        .cores()
        .list_extension_installs(core_id, &instance_id, kind)
        .await
    {
        Ok(installations) => installations,
        Err(error) => return registry_error_response(error, request_id),
    };

    Json(json!({
        "templateId": template.id(),
        "kind": kind,
        "directories": directory_pages,
        "installations": installations,
    }))
    .into_response()
}

async fn write_instance_extension(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    if body.len() > MAXIMUM_EXTENSION_WRITE_BYTES {
        return error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "PAYLOAD_TOO_LARGE",
            "Extension content exceeds the maximum size",
            request_id,
        );
    }
    let Some(idempotency_key) =
        header_text(&headers, "idempotency-key").filter(|value| value.parse::<RequestId>().is_ok())
    else {
        return error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "PRECONDITION_REQUIRED",
            "Idempotency-Key is required",
            request_id,
        );
    };
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(instance_id) = instance_id.parse::<InstanceId>().ok() else {
        return validation_error(request_id);
    };
    let Some(template_id) = query.get("templateId").filter(|value| !value.is_empty()) else {
        return validation_error(request_id);
    };
    let Some(kind) = query
        .get("kind")
        .and_then(|value| from_value::<ExtensionKind>(json!(value)).ok())
    else {
        return validation_error(request_id);
    };
    let Some(path) = query.get("path").filter(|value| !value.is_empty()) else {
        return validation_error(request_id);
    };
    let expected_sha256 = match expected_file_hash(&headers) {
        Ok(hash) => hash,
        Err(()) => return validation_error(request_id),
    };
    let Some(template) = install_template(template_id) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Install template does not exist",
            request_id,
        );
    };

    let instance = match state.cores().get_instance(core_id, &instance_id).await {
        Ok(value) => match from_value::<Instance>(value) {
            Ok(instance) => instance,
            Err(_) => return invalid_core_response(request_id),
        },
        Err(error) => return registry_error_response(error, request_id),
    };
    if instance.kind() != template.instance_kind() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TEMPLATE_INSTANCE_KIND_MISMATCH",
            "Install template does not match the instance type",
            request_id,
        );
    }

    let directories = template.extension_directories(kind);
    if directories.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_KIND_UNSUPPORTED",
            "The install template does not declare this extension kind",
            request_id,
        );
    }
    if !directories
        .iter()
        .any(|directory| is_extension_path(path, directory))
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_PATH_OUTSIDE_LAYOUT",
            "Extension path is outside the declared template directories",
            request_id,
        );
    }

    let entry = match state
        .cores()
        .write_instance_file(
            core_id,
            &instance_id,
            path,
            &body,
            expected_sha256.as_deref(),
            idempotency_key,
        )
        .await
    {
        Ok(entry) => entry,
        Err(error) => return registry_error_response(error, request_id),
    };
    let Some(sha256) = entry.sha256() else {
        return error_response(
            StatusCode::BAD_GATEWAY,
            "INTERNAL_ERROR",
            "Core did not return an extension file hash",
            request_id,
        );
    };
    let install = ExtensionInstall::new(
        RequestId::new().to_string(),
        kind,
        path.to_owned(),
        sha256.to_owned(),
        "LOCAL".to_owned(),
        None,
        None,
        current_timestamp(),
    );
    match state
        .cores()
        .upsert_extension_install(core_id, &instance_id, &install)
        .await
    {
        Ok(_) => Json(entry).into_response(),
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn delete_instance_extension(
    State(state): State<PanelState>,
    Extension(request_id): Extension<RequestId>,
    Path((core_id, instance_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authorize(&state, &headers, true, request_id).await {
        return response;
    }
    let Some(core_id) = parse_core_id(&core_id) else {
        return invalid_core_id_response(request_id);
    };
    let Some(instance_id) = instance_id.parse::<InstanceId>().ok() else {
        return validation_error(request_id);
    };
    let Some(template_id) = query.get("templateId").filter(|value| !value.is_empty()) else {
        return validation_error(request_id);
    };
    let Some(kind) = query
        .get("kind")
        .and_then(|value| from_value::<ExtensionKind>(json!(value)).ok())
    else {
        return validation_error(request_id);
    };
    let Some(path) = query.get("path").filter(|value| !value.is_empty()) else {
        return validation_error(request_id);
    };
    if query.get("confirmation").map(String::as_str) != Some("DELETE") {
        return validation_error(request_id);
    }
    let Some(idempotency_key) =
        header_text(&headers, "idempotency-key").filter(|value| value.parse::<RequestId>().is_ok())
    else {
        return error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "PRECONDITION_REQUIRED",
            "Idempotency-Key is required",
            request_id,
        );
    };
    let Some(template) = install_template(template_id) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Install template does not exist",
            request_id,
        );
    };

    let instance = match state.cores().get_instance(core_id, &instance_id).await {
        Ok(value) => match from_value::<Instance>(value) {
            Ok(instance) => instance,
            Err(_) => return invalid_core_response(request_id),
        },
        Err(error) => return registry_error_response(error, request_id),
    };
    if instance.kind() != template.instance_kind() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TEMPLATE_INSTANCE_KIND_MISMATCH",
            "Install template does not match the instance type",
            request_id,
        );
    }

    let directories = template.extension_directories(kind);
    if directories.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_KIND_UNSUPPORTED",
            "The install template does not declare this extension kind",
            request_id,
        );
    }
    if !directories
        .iter()
        .any(|directory| is_extension_path(path, directory))
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EXTENSION_PATH_OUTSIDE_LAYOUT",
            "Extension path is outside the declared template directories",
            request_id,
        );
    }

    let expected_install_id = match state
        .cores()
        .list_extension_installs(core_id, &instance_id, kind)
        .await
    {
        Ok(installations) => installations
            .into_iter()
            .find(|installation| installation.path() == path)
            .map(|installation| installation.id().to_owned()),
        Err(error) => return registry_error_response(error, request_id),
    };

    match state
        .cores()
        .delete_instance_file(core_id, &instance_id, path, false, idempotency_key)
        .await
    {
        Ok(task) => {
            let Some(task_id) = task
                .get("taskId")
                .and_then(Value::as_str)
                .and_then(|value| value.parse::<TaskId>().ok())
            else {
                return invalid_core_response(request_id);
            };
            let task_state = state.clone();
            let task_instance_id = instance_id.clone();
            let task_path = path.to_owned();
            spawn(async move {
                finalize_extension_delete_task(
                    task_state,
                    core_id,
                    task_instance_id,
                    task_path,
                    kind,
                    expected_install_id,
                    task_id,
                )
                .await;
            });
            (StatusCode::ACCEPTED, Json(task)).into_response()
        }
        Err(error) => registry_error_response(error, request_id),
    }
}

async fn finalize_extension_delete_task(
    state: PanelState,
    core_id: CoreId,
    instance_id: InstanceId,
    path: String,
    kind: ExtensionKind,
    expected_install_id: Option<String>,
    task_id: TaskId,
) {
    let Some(expected_install_id) = expected_install_id else {
        return;
    };
    for _ in 0..600 {
        match state.cores().get_file_task(core_id, &task_id).await {
            Ok(task) => match task.get("state").and_then(Value::as_str) {
                Some("SUCCEEDED") => {
                    match state
                        .cores()
                        .list_extension_installs(core_id, &instance_id, kind)
                        .await
                    {
                        Ok(installations) => {
                            let current_install = installations
                                .into_iter()
                                .find(|installation| installation.path() == path);
                            if current_install.as_ref().is_some_and(|installation| {
                                installation.id() == expected_install_id
                            }) {
                                if let Err(error) = state
                                    .cores()
                                    .delete_extension_install(core_id, &instance_id, &path)
                                    .await
                                {
                                    tracing::error!(
                                        %error,
                                        %path,
                                        "Failed to remove extension installation record"
                                    );
                                }
                            }
                        }
                        Err(error) => {
                            tracing::warn!(%error, %path, "Failed to verify extension record after deletion");
                        }
                    }
                    return;
                }
                Some("FAILED") => {
                    tracing::warn!(%task_id, %path, "Extension file deletion failed; installation record retained");
                    return;
                }
                _ => {}
            },
            Err(error) => {
                tracing::debug!(%error, %task_id, "Extension deletion task is not yet readable");
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
    tracing::warn!(%task_id, %path, "Extension deletion task did not finish; installation record retained");
}

fn is_extension_path(path: &str, directory: &str) -> bool {
    path.strip_prefix(directory)
        .is_some_and(|suffix| suffix.starts_with('/') && suffix.len() > 1)
}

fn expected_file_hash(headers: &HeaderMap) -> Result<Option<String>, ()> {
    let Some(value) = header_text(headers, "if-match") else {
        return Ok(None);
    };
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(());
    };
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(());
    }

    Ok(Some(value.to_owned()))
}

fn is_missing_directory(error: &CoreRegistryError) -> bool {
    matches!(
        error,
        CoreRegistryError::Connection(CoreConnectionError::Rejected { code })
            if code == "FILE_NOT_FOUND"
    )
}

fn invalid_core_response(request_id: RequestId) -> Response {
    error_response(
        StatusCode::BAD_GATEWAY,
        "INTERNAL_ERROR",
        "Core returned an invalid instance response",
        request_id,
    )
}

fn validation_error(request_id: RequestId) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "VALIDATION_FAILED",
        "Extension scan parameters are invalid",
        request_id,
    )
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(test)]
mod tests {
    use super::digest_hex;
    use super::extension_install_path;
    use super::selected_extension_directory;
    use nexus_domain::ExtensionArtifact;
    use nexus_domain::ExtensionDependency;
    use nexus_domain::ExtensionPlanItem;
    use sha2::Digest;
    use sha2::Sha256;

    #[test]
    fn requires_a_directory_when_a_kind_has_multiple_layouts() {
        assert_eq!(
            selected_extension_directory(&["plugins", "extra-plugins"], None),
            None
        );
        assert_eq!(
            selected_extension_directory(&["plugins", "extra-plugins"], Some("extra-plugins")),
            Some("extra-plugins")
        );
    }

    #[test]
    fn rejects_artifact_names_that_escape_the_declared_directory() {
        let item = plan_item("../escape.jar");

        assert_eq!(extension_install_path("plugins", &item), None);
    }

    #[test]
    fn builds_a_path_for_a_simple_artifact_name() {
        let item = plan_item("example.jar");

        assert_eq!(
            extension_install_path("plugins", &item),
            Some("plugins/example.jar".to_owned())
        );
    }

    #[test]
    fn encodes_digests_as_lowercase_hex() {
        assert_eq!(
            digest_hex(Sha256::digest(b"MCNP")),
            "ae682c35f1e161b90c064080705fd2c48406b5fad3d6ea5f8995980954ac43a6"
        );
    }

    fn plan_item(file_name: &str) -> ExtensionPlanItem {
        ExtensionPlanItem::new(
            "modrinth".to_owned(),
            "project".to_owned(),
            "version".to_owned(),
            "1.0.0".to_owned(),
            ExtensionArtifact::new(
                file_name.to_owned(),
                "https://cdn.modrinth.com/example.jar".to_owned(),
                4,
                None,
                "a".repeat(128),
                true,
            ),
            Vec::<ExtensionDependency>::new(),
        )
    }
}
