use std::collections::HashMap;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use nexus_domain::ManagedRuntime;
use nexus_domain::RuntimeInstallManifest;
use nexus_domain::RuntimeKind;
use nexus_domain::TaskId;
use serde_json::Value;
use serde_json::json;
use tokio::fs;
use tokio::spawn;
use tokio::task::spawn_blocking;

use crate::DownloadManager;
use crate::DownloadTask;
use crate::InstanceRepository;
use crate::RuntimeManagerError;
use crate::archive_extractor;
use crate::runtime_discovery::RuntimeDiscovery;

const DESCRIPTOR_FILE_NAME: &str = ".mcnp-runtime.json";

#[derive(Clone)]
pub(crate) struct RuntimeManager {
    discovery: RuntimeDiscovery,
    download: DownloadManager,
    tasks: Arc<Mutex<HashMap<TaskId, Value>>>,
}

impl RuntimeManager {
    pub(crate) fn new(data_directory: &Path) -> Result<Self, RuntimeManagerError> {
        Ok(Self {
            discovery: RuntimeDiscovery::new(data_directory),
            download: DownloadManager::new(data_directory)?,
            tasks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub(crate) async fn discover(&self) -> Vec<ManagedRuntime> {
        self.discovery.discover().await
    }

    pub(crate) fn start_install(
        &self,
        manifest: &RuntimeInstallManifest,
        set_as_default: bool,
    ) -> Result<TaskId, RuntimeManagerError> {
        validate_manifest(manifest)?;
        let task_id = self.start_task(task_kind("install"))?;
        let manager = self.clone();
        let manifest = manifest.clone();
        spawn(async move {
            let result = manager.install(&manifest, set_as_default).await;
            manager.finish_task(task_id, result.map(|runtime| json!({ "runtime": runtime })));
        });
        Ok(task_id)
    }

    pub(crate) fn start_verify(&self, runtime_id: &str) -> Result<TaskId, RuntimeManagerError> {
        validate_runtime_id(runtime_id)?;
        let task_id = self.start_task(task_kind("verify"))?;
        let manager = self.clone();
        let runtime_id = runtime_id.to_owned();
        spawn(async move {
            let result = manager.verify(&runtime_id).await;
            manager.finish_task(task_id, result.map(|runtime| json!({ "runtime": runtime })));
        });
        Ok(task_id)
    }

    pub(crate) fn start_delete(
        &self,
        runtime_id: &str,
        instances: &InstanceRepository,
    ) -> Result<TaskId, RuntimeManagerError> {
        validate_runtime_id(runtime_id)?;
        let task_id = self.start_task(task_kind("delete"))?;
        let manager = self.clone();
        let runtime_id = runtime_id.to_owned();
        let instances = instances.clone();
        spawn(async move {
            let result = manager.delete(&runtime_id, &instances).await;
            manager.finish_task(task_id, result.map(|()| json!({})));
        });
        Ok(task_id)
    }

    pub(crate) fn task(&self, task_id: TaskId) -> Result<Option<Value>, RuntimeManagerError> {
        let tasks = self
            .tasks
            .lock()
            .map_err(|_| RuntimeManagerError::TaskStorePoisoned)?;
        Ok(tasks.get(&task_id).cloned())
    }

    pub(crate) async fn install(
        &self,
        manifest: &RuntimeInstallManifest,
        set_as_default: bool,
    ) -> Result<ManagedRuntime, RuntimeManagerError> {
        validate_manifest(manifest)?;
        let runtime_path = self.runtime_path(manifest.kind(), manifest.runtime_id());
        if fs::try_exists(&runtime_path)
            .await
            .map_err(|source| storage("check", &runtime_path, source))?
        {
            return Err(RuntimeManagerError::AlreadyExists {
                runtime_id: manifest.runtime_id().to_owned(),
            });
        }

        let task = DownloadTask::new();
        let archive_path = self.download.download(&task, manifest.archive()).await?;
        let kind_directory = runtime_path
            .parent()
            .ok_or(RuntimeManagerError::InvalidRuntimeId)?;
        fs::create_dir_all(kind_directory)
            .await
            .map_err(|source| storage("create", kind_directory, source))?;
        let temporary_path =
            kind_directory.join(format!(".{}.{}.partial", manifest.runtime_id(), task.id()));
        remove_if_exists(&temporary_path).await?;
        fs::create_dir_all(&temporary_path)
            .await
            .map_err(|source| storage("create", &temporary_path, source))?;

        let archive_format = manifest.archive_format();
        let archive_path_for_worker = archive_path.clone();
        let temporary_path_for_worker = temporary_path.clone();
        let extraction = spawn_blocking(move || {
            archive_extractor::extract(
                &archive_path_for_worker,
                archive_format,
                &temporary_path_for_worker,
            )
        })
        .await
        .map_err(|error| RuntimeManagerError::Archive {
            operation: "extract",
            path: archive_path.clone(),
            message: error.to_string(),
        })?;
        if let Err(error) = extraction {
            remove_if_exists(&temporary_path).await?;
            return Err(error);
        }

        let executable = safe_relative_path(manifest.executable_path())?;
        let executable_path = temporary_path.join(executable);
        let metadata = fs::metadata(&executable_path)
            .await
            .map_err(|source| storage("read", &executable_path, source))?;
        if !metadata.is_file() {
            remove_if_exists(&temporary_path).await?;
            return Err(RuntimeManagerError::InvalidExecutable {
                path: executable_path,
            });
        }
        let descriptor_path = temporary_path.join(DESCRIPTOR_FILE_NAME);
        let descriptor = serde_json::to_vec_pretty(manifest)
            .map_err(|_| RuntimeManagerError::InvalidManifest { field: "manifest" })?;
        fs::write(&descriptor_path, descriptor)
            .await
            .map_err(|source| storage("write", &descriptor_path, source))?;
        fs::rename(&temporary_path, &runtime_path)
            .await
            .map_err(|source| storage("finalize", &runtime_path, source))?;

        if set_as_default {
            self.set_default(manifest.kind(), manifest.runtime_id())
                .await?;
        }

        self.discovery
            .find_managed(manifest.runtime_id())
            .await
            .ok_or_else(|| RuntimeManagerError::NotFound {
                runtime_id: manifest.runtime_id().to_owned(),
            })
    }

    pub(crate) async fn verify(
        &self,
        runtime_id: &str,
    ) -> Result<ManagedRuntime, RuntimeManagerError> {
        validate_runtime_id(runtime_id)?;
        self.discovery
            .find_managed(runtime_id)
            .await
            .ok_or_else(|| RuntimeManagerError::NotFound {
                runtime_id: runtime_id.to_owned(),
            })
    }

    pub(crate) async fn delete(
        &self,
        runtime_id: &str,
        instances: &InstanceRepository,
    ) -> Result<(), RuntimeManagerError> {
        validate_runtime_id(runtime_id)?;
        let Some(runtime_path) = self.discovery.find_managed_path(runtime_id) else {
            return Err(RuntimeManagerError::NotFound {
                runtime_id: runtime_id.to_owned(),
            });
        };
        if instances.references_runtime(&runtime_path)? {
            return Err(RuntimeManagerError::InUse {
                runtime_id: runtime_id.to_owned(),
            });
        }
        fs::remove_dir_all(&runtime_path)
            .await
            .map_err(|source| storage("remove", &runtime_path, source))?;
        Ok(())
    }

    fn runtime_path(&self, kind: RuntimeKind, runtime_id: &str) -> PathBuf {
        self.discovery
            .managed_root()
            .join(kind_directory(kind))
            .join(runtime_id)
    }

    async fn set_default(
        &self,
        kind: RuntimeKind,
        runtime_id: &str,
    ) -> Result<(), RuntimeManagerError> {
        let path = self
            .discovery
            .managed_root()
            .join(format!("default-{}", kind_directory(kind)));
        fs::create_dir_all(self.discovery.managed_root())
            .await
            .map_err(|source| storage("create", self.discovery.managed_root(), source))?;
        fs::write(&path, runtime_id)
            .await
            .map_err(|source| storage("write", &path, source))
    }

    fn start_task(&self, kind: String) -> Result<TaskId, RuntimeManagerError> {
        let task_id = TaskId::new();
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|_| RuntimeManagerError::TaskStorePoisoned)?;
        if tasks.len() >= 512 {
            tasks.clear();
        }
        tasks.insert(
            task_id,
            json!({
                "taskId": task_id,
                "kind": kind,
                "state": "RUNNING",
                "progress": null,
            }),
        );
        Ok(task_id)
    }

    fn finish_task(&self, task_id: TaskId, result: Result<Value, RuntimeManagerError>) {
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

fn task_kind(operation: &str) -> String {
    format!("RUNTIME_{}", operation.to_ascii_uppercase())
}

async fn remove_if_exists(path: &Path) -> Result<(), RuntimeManagerError> {
    match fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(storage("remove", path, source)),
    }
}

fn validate_manifest(manifest: &RuntimeInstallManifest) -> Result<(), RuntimeManagerError> {
    validate_runtime_id(manifest.runtime_id())?;
    if manifest.distribution().is_empty() {
        return Err(RuntimeManagerError::InvalidManifest {
            field: "distribution",
        });
    }
    if manifest.version().is_empty() {
        return Err(RuntimeManagerError::InvalidManifest { field: "version" });
    }
    safe_relative_path(manifest.executable_path())?;
    Ok(())
}

fn validate_runtime_id(runtime_id: &str) -> Result<(), RuntimeManagerError> {
    let bytes = runtime_id.as_bytes();
    if !(1..=64).contains(&bytes.len())
        || !bytes[0].is_ascii_alphanumeric()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(RuntimeManagerError::InvalidRuntimeId);
    }
    Ok(())
}

fn safe_relative_path(value: &str) -> Result<PathBuf, RuntimeManagerError> {
    if value.is_empty() || value.contains('\0') {
        return Err(RuntimeManagerError::InvalidManifest {
            field: "executablePath",
        });
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(RuntimeManagerError::InvalidManifest {
            field: "executablePath",
        });
    }
    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => relative.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(RuntimeManagerError::InvalidManifest {
                    field: "executablePath",
                });
            }
        }
    }
    if relative.as_os_str().is_empty() {
        return Err(RuntimeManagerError::InvalidManifest {
            field: "executablePath",
        });
    }
    Ok(relative)
}

fn kind_directory(kind: RuntimeKind) -> &'static str {
    match kind {
        RuntimeKind::Java => "java",
        RuntimeKind::NodeJs => "node",
        RuntimeKind::Python => "python",
    }
}

fn storage(operation: &'static str, path: &Path, source: std::io::Error) -> RuntimeManagerError {
    RuntimeManagerError::Storage {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::io::Write;

    use nexus_domain::DownloadArchitecture;
    use nexus_domain::DownloadManifest;
    use nexus_domain::DownloadPlatform;
    use nexus_domain::RuntimeArchiveFormat;
    use nexus_domain::RuntimeInstallManifest;
    use nexus_domain::RuntimeKind;
    use nexus_domain::RuntimeSource;
    use nexus_domain::Sha256Digest;
    use sha2::Digest;
    use sha2::Sha256;
    use tempfile::tempdir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::RuntimeManager;

    #[tokio::test]
    async fn installs_verifies_and_deletes_a_cached_runtime_archive() {
        let data_directory = tempdir().expect("temporary data directory is created");
        let archive = archive_with_executable("runtime/bin/fake", b"not an executable");
        let digest = Sha256::digest(&archive);
        let digest = Sha256Digest::from_hex(&hex_digest(digest)).expect("archive digest is valid");
        let cache_directory = data_directory.path().join("downloads");
        std::fs::create_dir_all(&cache_directory).expect("download cache is created");
        std::fs::write(cache_directory.join(digest.as_str()), &archive)
            .expect("cached archive is written");

        let manifest = RuntimeInstallManifest::new(
            "java-temurin-21".to_owned(),
            RuntimeKind::Java,
            "TEMURIN".to_owned(),
            "21.0.8".to_owned(),
            DownloadManifest::new(
                "https://example.invalid/runtime.zip".to_owned(),
                archive.len() as u64,
                digest,
                DownloadPlatform::current().expect("test platform is supported"),
                DownloadArchitecture::current().expect("test architecture is supported"),
            ),
            RuntimeArchiveFormat::Zip,
            "runtime/bin/fake".to_owned(),
        );
        let manager =
            RuntimeManager::new(data_directory.path()).expect("runtime manager is created");

        let installed = manager
            .install(&manifest, true)
            .await
            .expect("cached runtime is installed");
        assert_eq!(installed.runtime_id(), Some("java-temurin-21"));
        assert_eq!(installed.source(), RuntimeSource::Managed);
        assert_eq!(installed.distribution(), Some("TEMURIN"));
        assert_eq!(
            std::fs::read(
                data_directory
                    .path()
                    .join("runtimes/java/java-temurin-21/runtime/bin/fake")
            )
            .expect("runtime executable is extracted"),
            b"not an executable"
        );
        assert_eq!(
            std::fs::read_to_string(data_directory.path().join("runtimes/default-java"))
                .expect("default runtime is recorded"),
            "java-temurin-21"
        );

        let verified = manager
            .verify("java-temurin-21")
            .await
            .expect("installed runtime is verified");
        assert_eq!(verified.runtime_id(), Some("java-temurin-21"));

        manager
            .delete("java-temurin-21", &crate::InstanceRepository::new())
            .await
            .expect("unused runtime is deleted");
        assert!(
            !data_directory
                .path()
                .join("runtimes/java/java-temurin-21")
                .exists()
        );
    }

    fn archive_with_executable(path: &str, body: &[u8]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(path, SimpleFileOptions::default())
            .expect("zip file entry is created");
        writer.write_all(body).expect("zip file entry is written");
        writer
            .finish()
            .expect("zip archive is finalized")
            .into_inner()
    }

    fn hex_digest(digest: impl AsRef<[u8]>) -> String {
        digest
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}
