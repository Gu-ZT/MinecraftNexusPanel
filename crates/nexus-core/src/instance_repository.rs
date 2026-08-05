use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use nexus_domain::Instance;
use nexus_domain::InstanceCreate;
use nexus_domain::InstanceId;
use nexus_domain::InstanceRuntime;
use nexus_domain::InstanceState;
use nexus_domain::InstanceUpdate;
use serde_json::from_slice;
use serde_json::to_writer_pretty;
use tempfile::NamedTempFile;

use crate::InstanceRepositoryError;

const STORE_FILE_NAME: &str = "instances.json";

/// 保存实例配置和运行时快照的并发安全仓库。
///
/// 配置更新必须携带调用方看到的修订号；进程状态转换则通过允许状态列表
/// 保证不会覆盖并发请求产生的非法生命周期跳转。Core 通过 [`Self::open`]
/// 使用数据目录持久化，[`Self::new`] 则保留给纯内存嵌入式调用和单元测试。
#[derive(Clone, Default)]
pub struct InstanceRepository {
    instances: Arc<Mutex<BTreeMap<InstanceId, Instance>>>,
    path: Option<Arc<PathBuf>>,
}

impl InstanceRepository {
    /// 创建空实例仓库。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 打开数据目录中的实例仓库，并加载已保存的实例配置。
    ///
    /// 文件不存在时表示首次启动。若上次 Core 关闭前实例处于
    /// `STARTING`、`RUNNING` 或 `STOPPING`，这些状态会恢复为 `UNKNOWN`，因为
    /// 当前进程尚未重新确认对应的操作系统进程。
    pub fn open(data_directory: &Path) -> Result<Self, InstanceRepositoryError> {
        fs::create_dir_all(data_directory).map_err(|source| {
            InstanceRepositoryError::CreateDirectory {
                path: data_directory.to_path_buf(),
                source,
            }
        })?;
        let path = data_directory.join(STORE_FILE_NAME);
        let mut instances = load_instances(&path)?;
        for instance in instances.values_mut() {
            if matches!(
                instance.runtime().state(),
                InstanceState::Starting | InstanceState::Running | InstanceState::Stopping
            ) {
                instance.set_runtime(InstanceRuntime::unknown());
            }
        }

        Ok(Self {
            instances: Arc::new(Mutex::new(instances)),
            path: Some(Arc::new(path)),
        })
    }

    /// 校验并创建实例配置；重复标识会被拒绝。
    pub fn create(&self, definition: InstanceCreate) -> Result<Instance, InstanceRepositoryError> {
        let instance = definition.into_instance()?;
        let instance_id = instance.id().clone();
        let mut instances = self.lock_instances()?;

        if instances.contains_key(&instance_id) {
            return Err(InstanceRepositoryError::AlreadyExists { instance_id });
        }

        let previous = instances.clone();
        instances.insert(instance_id, instance.clone());
        if let Err(error) = self.persist_instances(&instances) {
            *instances = previous;
            return Err(error);
        }

        Ok(instance)
    }

    /// 查询实例的当前快照。
    pub fn get(
        &self,
        instance_id: &InstanceId,
    ) -> Result<Option<Instance>, InstanceRepositoryError> {
        let instances = self.lock_instances()?;

        Ok(instances.get(instance_id).cloned())
    }

    /// 返回当前全部实例快照。
    pub fn list(&self) -> Result<Vec<Instance>, InstanceRepositoryError> {
        let instances = self.lock_instances()?;

        Ok(instances.values().cloned().collect())
    }

    /// 删除实例配置并返回被删除的快照。
    pub fn remove(
        &self,
        instance_id: &InstanceId,
    ) -> Result<Option<Instance>, InstanceRepositoryError> {
        let mut instances = self.lock_instances()?;
        let previous = instances.clone();
        let removed = instances.remove(instance_id);
        if removed.is_some()
            && let Err(error) = self.persist_instances(&instances)
        {
            *instances = previous;
            return Err(error);
        }

        Ok(removed)
    }

    /// 判断某个受管运行时可执行路径是否仍被实例引用。
    pub fn references_runtime(&self, runtime_path: &Path) -> Result<bool, InstanceRepositoryError> {
        let instances = self.lock_instances()?;
        Ok(instances.values().any(|instance| {
            let executable = Path::new(instance.launch().executable());
            executable.is_absolute() && executable.starts_with(runtime_path)
        }))
    }

    /// 替换实例运行时快照，不改变配置修订号。
    pub fn set_runtime(
        &self,
        instance_id: &InstanceId,
        runtime: InstanceRuntime,
    ) -> Result<Instance, InstanceRepositoryError> {
        let mut instances = self.lock_instances()?;
        let previous = instances.clone();
        let updated = {
            let instance = instances.get_mut(instance_id).ok_or_else(|| {
                InstanceRepositoryError::NotFound {
                    instance_id: instance_id.clone(),
                }
            })?;
            instance.set_runtime(runtime);
            instance.clone()
        };
        if let Err(error) = self.persist_instances(&instances) {
            *instances = previous;
            return Err(error);
        }

        Ok(updated)
    }

    /// 将 `FAILED` 或 `UNKNOWN` 实例显式复位为可再次启动的 `STOPPED` 状态。
    ///
    /// `UNKNOWN` 表示 Core 尚未确认旧进程是否仍存在，因此复位必须由上层
    /// 结合显式确认动作触发，仓储本身不会自动放宽这个状态。
    pub fn reset(&self, instance_id: &InstanceId) -> Result<Instance, InstanceRepositoryError> {
        let mut instances = self.lock_instances()?;
        let previous = instances.clone();
        let updated = {
            let instance = instances.get_mut(instance_id).ok_or_else(|| {
                InstanceRepositoryError::NotFound {
                    instance_id: instance_id.clone(),
                }
            })?;
            let state = instance.runtime().state();
            if !matches!(state, InstanceState::Failed | InstanceState::Unknown) {
                return Err(InstanceRepositoryError::StateConflict {
                    instance_id: instance_id.clone(),
                    state,
                });
            }
            instance.set_runtime(InstanceRuntime::created().stopped(None));
            instance.clone()
        };
        if let Err(error) = self.persist_instances(&instances) {
            *instances = previous;
            return Err(error);
        }

        Ok(updated)
    }

    /// 按期望修订号应用实例配置更新。
    ///
    /// 运行中的实例不能修改配置，避免启动命令或工作目录在进程运行期间突变。
    pub fn update(
        &self,
        instance_id: &InstanceId,
        expected_revision: u64,
        update: &InstanceUpdate,
    ) -> Result<Instance, InstanceRepositoryError> {
        let mut instances = self.lock_instances()?;
        let previous = instances.clone();
        let updated = {
            let instance = instances.get_mut(instance_id).ok_or_else(|| {
                InstanceRepositoryError::NotFound {
                    instance_id: instance_id.clone(),
                }
            })?;
            if instance.revision() != expected_revision {
                return Err(InstanceRepositoryError::RevisionMismatch {
                    expected_revision,
                    actual_revision: instance.revision(),
                });
            }
            let state = instance.runtime().state();
            if !matches!(
                state,
                InstanceState::Created | InstanceState::Stopped | InstanceState::Failed
            ) {
                return Err(InstanceRepositoryError::StateConflict {
                    instance_id: instance_id.clone(),
                    state,
                });
            }
            instance.apply_update(update)?;
            instance.clone()
        };
        if let Err(error) = self.persist_instances(&instances) {
            *instances = previous;
            return Err(error);
        }

        Ok(updated)
    }

    /// 在允许的旧状态集合中原子替换运行时快照。
    pub fn transition_runtime(
        &self,
        instance_id: &InstanceId,
        allowed_states: &[InstanceState],
        runtime: InstanceRuntime,
    ) -> Result<Instance, InstanceRepositoryError> {
        let mut instances = self.lock_instances()?;
        let previous = instances.clone();
        let updated = {
            let instance = instances.get_mut(instance_id).ok_or_else(|| {
                InstanceRepositoryError::NotFound {
                    instance_id: instance_id.clone(),
                }
            })?;
            let state = instance.runtime().state();
            if !allowed_states.contains(&state) {
                return Err(InstanceRepositoryError::StateConflict {
                    instance_id: instance_id.clone(),
                    state,
                });
            }
            instance.set_runtime(runtime);
            instance.clone()
        };
        if let Err(error) = self.persist_instances(&instances) {
            *instances = previous;
            return Err(error);
        }

        Ok(updated)
    }

    /// 将已取得锁的实例快照写入同目录临时文件并原子替换正式文件。
    fn persist_instances(
        &self,
        instances: &BTreeMap<InstanceId, Instance>,
    ) -> Result<(), InstanceRepositoryError> {
        let Some(path) = self.path.as_deref() else {
            return Ok(());
        };
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let mut temporary = NamedTempFile::new_in(parent).map_err(|source| {
            InstanceRepositoryError::CreateTemporary {
                path: parent.to_path_buf(),
                source,
            }
        })?;
        to_writer_pretty(temporary.as_file_mut(), instances).map_err(|source| {
            InstanceRepositoryError::Encode {
                path: path.to_path_buf(),
                source,
            }
        })?;
        temporary
            .write_all(b"\n")
            .and_then(|_| temporary.flush())
            .and_then(|_| temporary.as_file().sync_all())
            .map_err(|source| InstanceRepositoryError::Write {
                path: path.to_path_buf(),
                source,
            })?;
        temporary
            .persist(path)
            .map_err(|error| InstanceRepositoryError::Replace {
                path: path.to_path_buf(),
                source: error.error,
            })
            .map(|_| ())
    }

    fn lock_instances(
        &self,
    ) -> Result<MutexGuard<'_, BTreeMap<InstanceId, Instance>>, InstanceRepositoryError> {
        self.instances
            .lock()
            .map_err(|_| InstanceRepositoryError::LockPoisoned)
    }
}

/// 从磁盘加载实例配置；缺失文件等价于首次启动。
fn load_instances(path: &Path) -> Result<BTreeMap<InstanceId, Instance>, InstanceRepositoryError> {
    let content = match fs::read(path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(source) => {
            return Err(InstanceRepositoryError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    from_slice(&content).map_err(|source| InstanceRepositoryError::Decode {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use super::InstanceRepository;
    use crate::InstanceRepositoryError;
    use nexus_domain::InstanceCreate;
    use nexus_domain::InstanceId;
    use nexus_domain::InstanceKind;
    use nexus_domain::InstanceRuntime;
    use nexus_domain::InstanceState;
    use nexus_domain::InstanceUpdate;
    use nexus_domain::LaunchConfig;
    use serde_json::from_value;
    use serde_json::json;
    use tempfile::tempdir;

    use super::STORE_FILE_NAME;

    #[test]
    fn creates_instances_once_and_lists_them_by_identifier() {
        let repository = InstanceRepository::new();
        let survival = instance_create("survival");
        let creative = instance_create("creative");

        repository
            .create(survival.clone())
            .expect("first instance is created");
        repository
            .create(creative)
            .expect("second instance is created");

        assert!(matches!(
            repository.create(survival),
            Err(InstanceRepositoryError::AlreadyExists { .. })
        ));

        let identifiers = repository
            .list()
            .expect("instances are listed")
            .into_iter()
            .map(|instance| instance.id().to_string())
            .collect::<Vec<_>>();

        assert_eq!(identifiers, ["creative", "survival"]);
    }

    #[test]
    fn updates_stopped_instances_with_revision_checks() {
        let repository = InstanceRepository::new();
        let instance = instance_create("survival");
        repository
            .create(instance)
            .expect("instance is created for settings updates");
        let instance_id = InstanceId::new("survival".to_owned()).expect("test identifier is valid");
        let update: InstanceUpdate = from_value(json!({
            "name": "Configured Survival",
            "directory": "instances/configured-survival",
            "updateCommand": "./update.sh",
            "expiresAt": "2030-01-01T00:00:00Z",
        }))
        .expect("update payload is valid");

        let updated = repository
            .update(&instance_id, 1, &update)
            .expect("stopped instance settings are updated");

        assert_eq!(updated.name(), "Configured Survival");
        assert_eq!(updated.directory(), "instances/configured-survival");
        assert_eq!(updated.update_command(), Some("./update.sh"));
        assert_eq!(updated.expires_at(), Some("2030-01-01T00:00:00Z"));
        assert_eq!(updated.revision(), 2);
        assert!(matches!(
            repository.update(&instance_id, 1, &update),
            Err(InstanceRepositoryError::RevisionMismatch {
                expected_revision: 1,
                actual_revision: 2,
            })
        ));

        repository
            .set_runtime(
                &instance_id,
                InstanceRuntime::running(42, "2030-01-01T00:00:00Z".to_owned()),
            )
            .expect("instance is marked running");
        assert!(matches!(
            repository.update(&instance_id, 2, &update),
            Err(InstanceRepositoryError::StateConflict {
                state: InstanceState::Running,
                ..
            })
        ));
    }

    #[test]
    fn reloads_configuration_and_recovers_transient_runtime_as_unknown() {
        let directory = tempdir().expect("temporary instance directory is created");
        let repository =
            InstanceRepository::open(directory.path()).expect("instance repository opens");
        let instance = instance_create("survival");
        repository
            .create(instance)
            .expect("instance configuration is persisted");
        let instance_id = InstanceId::new("survival".to_owned()).expect("instance ID is valid");
        repository
            .set_runtime(
                &instance_id,
                InstanceRuntime::running(42, "2026-08-06T00:00:00Z".to_owned()),
            )
            .expect("runtime snapshot is persisted");
        drop(repository);

        let reloaded =
            InstanceRepository::open(directory.path()).expect("instance repository reloads");
        let instance = reloaded
            .get(&instance_id)
            .expect("instance lookup succeeds")
            .expect("persisted instance exists");

        assert_eq!(instance.name(), "survival");
        assert_eq!(instance.runtime().state(), InstanceState::Unknown);
        assert_eq!(instance.runtime().pid(), None);
    }

    #[test]
    fn explicitly_resets_unknown_instances_to_stopped() {
        let directory = tempdir().expect("temporary instance directory is created");
        let repository =
            InstanceRepository::open(directory.path()).expect("instance repository opens");
        let instance = instance_create("survival");
        repository
            .create(instance)
            .expect("instance configuration is persisted");
        let instance_id = InstanceId::new("survival".to_owned()).expect("instance ID is valid");
        repository
            .set_runtime(
                &instance_id,
                InstanceRuntime::running(42, "2026-08-06T00:00:00Z".to_owned()),
            )
            .expect("runtime snapshot is persisted");
        drop(repository);

        let reloaded =
            InstanceRepository::open(directory.path()).expect("instance repository reloads");
        let reset = reloaded
            .reset(&instance_id)
            .expect("unknown instance resets");

        assert_eq!(reset.runtime().state(), InstanceState::Stopped);
        assert_eq!(
            reloaded
                .get(&instance_id)
                .expect("instance lookup succeeds")
                .expect("instance exists")
                .runtime()
                .state(),
            InstanceState::Stopped
        );
    }

    #[test]
    fn refuses_to_open_a_corrupted_instance_store() {
        let directory = tempdir().expect("temporary instance directory is created");
        fs::write(directory.path().join(STORE_FILE_NAME), b"not-json")
            .expect("corrupted instance store is written");

        assert!(matches!(
            InstanceRepository::open(directory.path()),
            Err(InstanceRepositoryError::Decode { .. })
        ));
    }

    fn instance_create(identifier: &str) -> InstanceCreate {
        InstanceCreate::new(
            InstanceId::new(identifier.to_owned()).expect("test identifier is valid"),
            identifier.to_owned(),
            InstanceKind::Paper,
            format!("instances/{identifier}"),
            LaunchConfig::new(
                "java".to_owned(),
                Vec::new(),
                BTreeMap::new(),
                "stop".to_owned(),
                30,
            ),
        )
        .expect("test instance is valid")
    }
}
