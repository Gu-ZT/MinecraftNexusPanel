use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use nexus_domain::InstallRuntimeRequirement;
use nexus_domain::InstanceCreate;
use nexus_domain::InstanceKind;
use nexus_domain::InstanceUpdate;
use nexus_domain::LaunchConfig;
use nexus_domain::ProvisionPlan;
use nexus_domain::TaskId;
use serde_json::Value;
use serde_json::from_value;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;
use tokio::fs;
use tokio::spawn;
use tokio::task::spawn_blocking;

use crate::DownloadManager;
use crate::DownloadTask;
use crate::InstanceRepository;
use crate::ProvisionManagerError;
use crate::RuntimeManager;
use crate::archive_extractor;

#[derive(Clone)]
pub(crate) struct ProvisionManager {
    data_directory: PathBuf,
    download: DownloadManager,
    runtimes: RuntimeManager,
    tasks: Arc<Mutex<HashMap<TaskId, Value>>>,
}

impl ProvisionManager {
    pub(crate) fn new(
        data_directory: &Path,
        runtimes: RuntimeManager,
    ) -> Result<Self, ProvisionManagerError> {
        Ok(Self {
            data_directory: data_directory.to_path_buf(),
            download: DownloadManager::new(data_directory)?,
            runtimes,
            tasks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub(crate) fn resolve(&self, plan: &ProvisionPlan) -> Result<Value, ProvisionManagerError> {
        validate_plan(plan)?;
        let plan_hash = plan_hash(plan)?;

        Ok(json!({
            "resolvedPlan": plan,
            "planHash": plan_hash,
        }))
    }

    pub(crate) fn start_execute(
        &self,
        plan: &ProvisionPlan,
        expected_hash: &str,
        instances: &InstanceRepository,
    ) -> Result<TaskId, ProvisionManagerError> {
        validate_plan(plan)?;
        if plan_hash(plan)? != expected_hash {
            return Err(ProvisionManagerError::PlanHashMismatch);
        }
        if instances.get(plan.instance_id())?.is_some() {
            return Err(ProvisionManagerError::AlreadyExists {
                instance_id: plan.instance_id().clone(),
            });
        }

        let task_id = self.start_task()?;
        let manager = self.clone();
        let plan = plan.clone();
        let instances = instances.clone();
        spawn(async move {
            let result = manager.execute(&plan, &instances, task_id).await;
            manager.finish_task(task_id, result);
        });
        Ok(task_id)
    }

    pub(crate) fn task(&self, task_id: TaskId) -> Result<Option<Value>, ProvisionManagerError> {
        let tasks = self
            .tasks
            .lock()
            .map_err(|_| ProvisionManagerError::TaskStorePoisoned)?;
        Ok(tasks.get(&task_id).cloned())
    }

    async fn execute(
        &self,
        plan: &ProvisionPlan,
        instances: &InstanceRepository,
        task_id: TaskId,
    ) -> Result<Value, ProvisionManagerError> {
        let instance_path = self.instance_path(plan)?;
        if fs::try_exists(&instance_path)
            .await
            .map_err(|source| storage("check", &instance_path, source))?
        {
            return Err(ProvisionManagerError::AlreadyExists {
                instance_id: plan.instance_id().clone(),
            });
        }

        let update = metadata_update(plan)?;
        let runtime_executable = self
            .runtimes
            .resolve_executable(plan.runtime_id(), plan.required_runtime())
            .await?;
        let download_task = DownloadTask::new();
        let archive_path = self
            .download
            .download(&download_task, plan.archive())
            .await?;
        let parent = instance_path
            .parent()
            .ok_or(ProvisionManagerError::InvalidPlan {
                field: "instanceDirectory",
            })?;
        fs::create_dir_all(parent)
            .await
            .map_err(|source| storage("create", parent, source))?;
        let temporary_path = parent.join(format!(".{}.{}.partial", plan.instance_id(), task_id));
        remove_directory_if_exists(&temporary_path).await?;
        fs::create_dir_all(&temporary_path)
            .await
            .map_err(|source| storage("create", &temporary_path, source))?;

        let archive_path_for_worker = archive_path.clone();
        let temporary_path_for_worker = temporary_path.clone();
        let archive_format = plan.archive_format();
        let extraction = spawn_blocking(move || {
            archive_extractor::extract(
                &archive_path_for_worker,
                archive_format,
                &temporary_path_for_worker,
            )
        })
        .await
        .map_err(|error| ProvisionManagerError::Archive {
            path: archive_path.clone(),
            message: error.to_string(),
        })?;
        if let Err(error) = extraction {
            remove_directory_if_exists(&temporary_path).await?;
            return Err(ProvisionManagerError::Runtime(error));
        }

        let executable_path = safe_relative_path(plan.executable_path())?;
        let extracted_executable = temporary_path.join(&executable_path);
        let metadata = match fs::metadata(&extracted_executable).await {
            Ok(metadata) => metadata,
            Err(source) => {
                remove_directory_if_exists(&temporary_path).await?;
                return Err(storage("read", &extracted_executable, source));
            }
        };
        if !metadata.is_file() {
            remove_directory_if_exists(&temporary_path).await?;
            return Err(ProvisionManagerError::InvalidPlan {
                field: "executablePath",
            });
        }

        let server_argument = executable_path.to_string_lossy().replace('\\', "/");
        let launch_arguments = launch_arguments(plan, &server_argument);
        let executable = if plan.required_runtime() == InstallRuntimeRequirement::Native {
            instance_path
                .join(&executable_path)
                .to_string_lossy()
                .into_owned()
        } else {
            runtime_executable
        };
        let launch = LaunchConfig::new(
            executable,
            launch_arguments,
            BTreeMap::new(),
            plan.stop_command().to_owned(),
            plan.stop_timeout_seconds(),
        );
        let definition = InstanceCreate::new(
            plan.instance_id().clone(),
            plan.instance_name().to_owned(),
            plan.instance_kind(),
            plan.instance_directory().to_owned(),
            launch,
        )?;

        if let Err(source) = fs::rename(&temporary_path, &instance_path).await {
            remove_directory_if_exists(&temporary_path).await?;
            return Err(storage("finalize", &instance_path, source));
        }

        let instance = match instances.create(definition) {
            Ok(instance) => instance,
            Err(error) => {
                remove_directory_if_exists(&instance_path).await?;
                return Err(error.into());
            }
        };
        let instance = if let Some(update) = update {
            match instances.update(instance.id(), instance.revision(), &update) {
                Ok(instance) => instance,
                Err(error) => {
                    instances.remove(instance.id())?;
                    remove_directory_if_exists(&instance_path).await?;
                    return Err(error.into());
                }
            }
        } else {
            instance
        };

        Ok(json!({
            "instanceId": instance.id(),
            "instance": instance,
        }))
    }

    fn instance_path(&self, plan: &ProvisionPlan) -> Result<PathBuf, ProvisionManagerError> {
        let directory = safe_directory_path(plan.instance_directory())?;
        Ok(self.data_directory.join(directory))
    }

    fn start_task(&self) -> Result<TaskId, ProvisionManagerError> {
        let task_id = TaskId::new();
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|_| ProvisionManagerError::TaskStorePoisoned)?;
        if tasks.len() >= 512 {
            tasks.clear();
        }
        tasks.insert(
            task_id,
            json!({
                "taskId": task_id,
                "kind": "PROVISION",
                "state": "RUNNING",
                "progress": null,
            }),
        );
        Ok(task_id)
    }

    fn finish_task(&self, task_id: TaskId, result: Result<Value, ProvisionManagerError>) {
        let Ok(mut tasks) = self.tasks.lock() else {
            return;
        };
        let Some(task) = tasks.get_mut(&task_id) else {
            return;
        };
        match result {
            Ok(result) => {
                task["state"] = json!("SUCCEEDED");
                if let Some(object) = result.as_object() {
                    for (key, value) in object {
                        task[key] = value.clone();
                    }
                }
            }
            Err(error) => {
                task["state"] = json!("FAILED");
                task["error"] = json!(error.to_string());
            }
        }
    }
}

fn validate_plan(plan: &ProvisionPlan) -> Result<(), ProvisionManagerError> {
    if plan.template_id().is_empty() || plan.template_id().len() > 64 {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "templateId",
        });
    }
    if !plan
        .template_id()
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "templateId",
        });
    }
    if template_kind(plan.template_id()) != Some(plan.instance_kind()) {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "templateId",
        });
    }
    if plan.minecraft_version().is_empty() || plan.minecraft_version().len() > 128 {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "minecraftVersion",
        });
    }
    if plan.build().is_empty() || plan.build().len() > 128 {
        return Err(ProvisionManagerError::InvalidPlan { field: "build" });
    }
    if !plan.archive().supports_current_target() {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "archive.target",
        });
    }
    if required_runtime(plan.instance_kind()) != Some(plan.required_runtime()) {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "requiredRuntime",
        });
    }
    if plan.required_runtime() == InstallRuntimeRequirement::Native && plan.runtime_id().is_some() {
        return Err(ProvisionManagerError::InvalidPlan { field: "runtimeId" });
    }
    if plan
        .runtime_id()
        .is_some_and(|value| !valid_runtime_id(value))
    {
        return Err(ProvisionManagerError::InvalidPlan { field: "runtimeId" });
    }
    safe_directory_path(plan.instance_directory())?;
    safe_relative_path(plan.executable_path())?;
    if plan.launch_arguments().len() > 256
        || plan
            .launch_arguments()
            .iter()
            .any(|argument| argument.len() > 8192 || argument.contains('\0'))
    {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "launchArguments",
        });
    }
    if plan.stop_command().is_empty()
        || plan.stop_command().len() > 8192
        || plan.stop_command().contains('\0')
        || !(1..=300).contains(&plan.stop_timeout_seconds())
    {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "stopCommand",
        });
    }
    let placeholder_launch = LaunchConfig::new(
        "java".to_owned(),
        Vec::new(),
        BTreeMap::new(),
        plan.stop_command().to_owned(),
        plan.stop_timeout_seconds(),
    );
    InstanceCreate::new(
        plan.instance_id().clone(),
        plan.instance_name().to_owned(),
        plan.instance_kind(),
        plan.instance_directory().to_owned(),
        placeholder_launch,
    )
    .map_err(|_| ProvisionManagerError::InvalidPlan { field: "instance" })?;
    metadata_update(plan)?;
    Ok(())
}

fn metadata_update(plan: &ProvisionPlan) -> Result<Option<InstanceUpdate>, ProvisionManagerError> {
    if plan.update_command().is_none() && plan.expires_at().is_none() {
        return Ok(None);
    }
    let update: InstanceUpdate = from_value(json!({
        "updateCommand": plan.update_command(),
        "expiresAt": plan.expires_at(),
    }))
    .map_err(|_| ProvisionManagerError::InvalidPlan {
        field: "instanceMetadata",
    })?;
    update
        .validate()
        .map_err(|_| ProvisionManagerError::InvalidPlan {
            field: "instanceMetadata",
        })?;
    Ok(Some(update))
}

fn required_runtime(kind: InstanceKind) -> Option<InstallRuntimeRequirement> {
    match kind {
        InstanceKind::BedrockDedicatedServer => Some(InstallRuntimeRequirement::Native),
        InstanceKind::PocketMineMp => Some(InstallRuntimeRequirement::Php),
        InstanceKind::Vanilla
        | InstanceKind::Paper
        | InstanceKind::Velocity
        | InstanceKind::Fabric
        | InstanceKind::NeoForge
        | InstanceKind::Forge
        | InstanceKind::Bukkit
        | InstanceKind::Spigot
        | InstanceKind::Purpur
        | InstanceKind::Pufferfish
        | InstanceKind::Folia
        | InstanceKind::Leaf
        | InstanceKind::Mohist
        | InstanceKind::Magma
        | InstanceKind::Sponge
        | InstanceKind::Arclight
        | InstanceKind::Youer
        | InstanceKind::AsyncYouer
        | InstanceKind::Silkard
        | InstanceKind::CatServer
        | InstanceKind::Lingshu
        | InstanceKind::Waterfall
        | InstanceKind::BungeeCord
        | InstanceKind::Lightfall
        | InstanceKind::Geyser
        | InstanceKind::Nukkit
        | InstanceKind::CloudburstNukkit => Some(InstallRuntimeRequirement::Java),
        InstanceKind::Custom | InstanceKind::Unknown => None,
    }
}

fn template_kind(template_id: &str) -> Option<InstanceKind> {
    match template_id {
        "vanilla" => Some(InstanceKind::Vanilla),
        "paper" => Some(InstanceKind::Paper),
        "velocity" => Some(InstanceKind::Velocity),
        "fabric" => Some(InstanceKind::Fabric),
        "neoforge" => Some(InstanceKind::NeoForge),
        "forge" => Some(InstanceKind::Forge),
        "bukkit" => Some(InstanceKind::Bukkit),
        "spigot" => Some(InstanceKind::Spigot),
        "purpur" => Some(InstanceKind::Purpur),
        "pufferfish" => Some(InstanceKind::Pufferfish),
        "folia" => Some(InstanceKind::Folia),
        "leaf" => Some(InstanceKind::Leaf),
        "mohist" => Some(InstanceKind::Mohist),
        "magma" => Some(InstanceKind::Magma),
        "sponge" => Some(InstanceKind::Sponge),
        "arclight" => Some(InstanceKind::Arclight),
        "youer" => Some(InstanceKind::Youer),
        "async-youer" => Some(InstanceKind::AsyncYouer),
        "silkard" => Some(InstanceKind::Silkard),
        "catserver" => Some(InstanceKind::CatServer),
        "lingshu" => Some(InstanceKind::Lingshu),
        "waterfall" => Some(InstanceKind::Waterfall),
        "bungeecord" => Some(InstanceKind::BungeeCord),
        "lightfall" => Some(InstanceKind::Lightfall),
        "geyser" => Some(InstanceKind::Geyser),
        "bedrock-dedicated-server" => Some(InstanceKind::BedrockDedicatedServer),
        "pocketmine-mp" => Some(InstanceKind::PocketMineMp),
        "nukkit" => Some(InstanceKind::Nukkit),
        "cloudburst-nukkit" => Some(InstanceKind::CloudburstNukkit),
        _ => None,
    }
}

fn launch_arguments(plan: &ProvisionPlan, server_argument: &str) -> Vec<String> {
    let arguments = if plan.launch_arguments().is_empty() {
        match plan.required_runtime() {
            InstallRuntimeRequirement::Java => vec!["-jar".to_owned(), "{server}".to_owned()],
            InstallRuntimeRequirement::NodeJs
            | InstallRuntimeRequirement::Python
            | InstallRuntimeRequirement::Php
            | InstallRuntimeRequirement::Native => vec!["{server}".to_owned()],
        }
    } else {
        plan.launch_arguments().to_vec()
    };
    arguments
        .into_iter()
        .map(|argument| argument.replace("{server}", server_argument))
        .collect()
}

fn plan_hash(plan: &ProvisionPlan) -> Result<String, ProvisionManagerError> {
    let bytes = serde_json::to_vec(plan).map_err(ProvisionManagerError::Serialization)?;
    Ok(hex_digest(Sha256::digest(bytes)))
}

fn safe_directory_path(value: &str) -> Result<PathBuf, ProvisionManagerError> {
    if value.is_empty() || value.contains('\0') {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "instanceDirectory",
        });
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "instanceDirectory",
        });
    }
    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => relative.push(value),
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(ProvisionManagerError::InvalidPlan {
                    field: "instanceDirectory",
                });
            }
        }
    }
    if relative.as_os_str().is_empty() {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "instanceDirectory",
        });
    }
    Ok(relative)
}

fn safe_relative_path(value: &str) -> Result<PathBuf, ProvisionManagerError> {
    if value.is_empty() || value.contains('\0') {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "executablePath",
        });
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "executablePath",
        });
    }
    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => relative.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ProvisionManagerError::InvalidPlan {
                    field: "executablePath",
                });
            }
        }
    }
    if relative.as_os_str().is_empty() {
        return Err(ProvisionManagerError::InvalidPlan {
            field: "executablePath",
        });
    }
    Ok(relative)
}

fn valid_runtime_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=64).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

async fn remove_directory_if_exists(path: &Path) -> Result<(), ProvisionManagerError> {
    match fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(storage("remove", path, source)),
    }
}

fn storage(operation: &'static str, path: &Path, source: std::io::Error) -> ProvisionManagerError {
    ProvisionManagerError::Storage {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(digest.as_ref().len() * 2);
    for byte in digest.as_ref() {
        value.push(char::from(HEX[usize::from(*byte >> 4)]));
        value.push(char::from(HEX[usize::from(*byte & 0x0f)]));
    }
    value
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::io::Write;

    use nexus_domain::DownloadArchitecture;
    use nexus_domain::DownloadManifest;
    use nexus_domain::DownloadPlatform;
    use nexus_domain::InstallRuntimeRequirement;
    use nexus_domain::InstanceId;
    use nexus_domain::InstanceKind;
    use nexus_domain::ProvisionPlan;
    use nexus_domain::RuntimeArchiveFormat;
    use nexus_domain::Sha256Digest;
    use nexus_domain::TaskId;
    use sha2::Digest;
    use sha2::Sha256;
    use tempfile::tempdir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::ProvisionManager;
    use crate::InstanceRepository;

    #[tokio::test]
    async fn resolves_and_executes_a_cached_server_archive() {
        let data_directory = tempdir().expect("temporary data directory is created");
        let archive = archive_with_file("server.jar", b"server");
        let digest = Sha256Digest::from_hex(&hex_digest(Sha256::digest(&archive)))
            .expect("archive digest is valid");
        let cache_directory = data_directory.path().join("downloads");
        std::fs::create_dir_all(&cache_directory).expect("download cache is created");
        std::fs::write(cache_directory.join(digest.as_str()), &archive)
            .expect("cached archive is written");

        let plan = ProvisionPlan::new(
            "paper".to_owned(),
            "1.21.8".to_owned(),
            "latest".to_owned(),
            InstanceId::new("survival".to_owned()).expect("instance ID is valid"),
            "Survival".to_owned(),
            InstanceKind::Paper,
            "instances/survival".to_owned(),
            None,
            None,
            InstallRuntimeRequirement::Java,
            None,
            DownloadManifest::new(
                "https://example.invalid/server.zip".to_owned(),
                archive.len() as u64,
                digest,
                DownloadPlatform::current().expect("test platform is supported"),
                DownloadArchitecture::current().expect("test architecture is supported"),
            ),
            RuntimeArchiveFormat::Zip,
            "server.jar".to_owned(),
            Vec::new(),
            "stop".to_owned(),
            30,
        );
        let runtimes =
            crate::RuntimeManager::new(data_directory.path()).expect("runtime manager is created");
        let manager = ProvisionManager::new(data_directory.path(), runtimes)
            .expect("provision manager is created");
        let repository = InstanceRepository::new();

        let resolved = manager.resolve(&plan).expect("plan is resolved");
        let plan_hash = resolved["planHash"]
            .as_str()
            .expect("plan hash is returned");
        let task_id = TaskId::new();
        let result = manager
            .execute(&plan, &repository, task_id)
            .await
            .expect("cached server archive is provisioned");

        assert_eq!(result["instanceId"], "survival");
        assert_eq!(result["instance"]["kind"], "PAPER");
        assert_eq!(plan_hash.len(), 64);
        assert_eq!(
            std::fs::read(data_directory.path().join("instances/survival/server.jar"))
                .expect("server archive is extracted"),
            b"server"
        );
        assert!(
            repository
                .get(&InstanceId::new("survival".to_owned()).expect("instance ID is valid"))
                .expect("instance repository is readable")
                .is_some()
        );
    }

    fn archive_with_file(path: &str, body: &[u8]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(path, SimpleFileOptions::default())
            .expect("archive file is created");
        writer.write_all(body).expect("archive file is written");
        writer.finish().expect("archive is finalized").into_inner()
    }

    fn hex_digest(digest: impl AsRef<[u8]>) -> String {
        digest
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}
