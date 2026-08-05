use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use nexus_domain::CpuReservation;
use nexus_domain::InstanceId;
use nexus_domain::TaskId;
use serde_json::from_slice;
use serde_json::to_writer_pretty;
use tempfile::NamedTempFile;
use thiserror::Error;

const STORE_FILE_NAME: &str = "cpu-reservations.json";

/// Core 内存中的 CPU 独占预留仓储。
///
/// 仓储负责同一 Core 进程内的原子冲突检查，并通过可选的 JSON 文件保存已登记
/// 的预留。预留持久化只恢复“资源登记”事实，不代表宿主机 affinity、Docker
/// cpuset 或跨 Core 调度锁已经应用。
#[derive(Clone, Default)]
pub(crate) struct CpuReservationRepository {
    reservations: Arc<Mutex<HashMap<TaskId, CpuReservation>>>,
    path: Option<Arc<PathBuf>>,
}

/// CPU 预留仓储操作、加载和持久化错误。
#[derive(Debug, Error)]
pub enum CpuReservationRepositoryError {
    /// 请求的 CPU 已被其他实例预留。
    #[error("CPU {cpu_id} is already reserved by reservation {reservation_id}")]
    Conflict {
        /// 冲突的逻辑 CPU ID。
        cpu_id: u32,
        /// 已经占用该 CPU 的预留标识。
        reservation_id: TaskId,
    },
    /// 请求释放的预留不存在。
    #[error("CPU reservation {reservation_id} does not exist")]
    NotFound {
        /// 不存在的预留标识。
        reservation_id: TaskId,
    },
    /// 预留仓储锁被污染。
    #[error("CPU reservation store lock is poisoned")]
    StorePoisoned,
    /// CPU 预留数据目录创建失败。
    #[error("failed to create CPU reservation data directory {path}")]
    CreateDirectory {
        /// 创建失败的数据目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的目录创建错误。
        source: io::Error,
    },
    /// 读取已有 CPU 预留文件失败。
    #[error("failed to read CPU reservation store {path}")]
    Read {
        /// 读取失败的预留文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的读取错误。
        source: io::Error,
    },
    /// 已有 CPU 预留文件不是合法的预留数组。
    #[error("CPU reservation store {path} contains invalid JSON")]
    Decode {
        /// 包含非法 JSON 的预留文件路径。
        path: PathBuf,
        #[source]
        /// JSON 解码错误。
        source: serde_json::Error,
    },
    /// 创建 CPU 预留临时文件失败。
    #[error("failed to create temporary CPU reservation store in {path}")]
    CreateTemporary {
        /// 创建临时文件的目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的创建错误。
        source: io::Error,
    },
    /// 序列化 CPU 预留失败。
    #[error("failed to encode CPU reservation store {path}")]
    Encode {
        /// 无法写入的预留文件路径。
        path: PathBuf,
        #[source]
        /// JSON 编码错误。
        source: serde_json::Error,
    },
    /// 写入或同步 CPU 预留临时文件失败。
    #[error("failed to write CPU reservation store {path}")]
    Write {
        /// 写入失败的预留文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的写入错误。
        source: io::Error,
    },
    /// 用临时文件替换正式 CPU 预留文件失败。
    #[error("failed to atomically replace CPU reservation store {path}")]
    Replace {
        /// 替换失败的正式预留文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的替换错误。
        source: io::Error,
    },
}

impl CpuReservationRepository {
    /// 打开数据目录中的 CPU 预留仓储，并加载已有登记。
    ///
    /// 文件不存在时表示首次启动；文件存在但 JSON 损坏时直接返回错误，避免
    /// Core 在不完整的资源登记上继续运行。
    pub(crate) fn open(data_directory: &Path) -> Result<Self, CpuReservationRepositoryError> {
        fs::create_dir_all(data_directory).map_err(|source| {
            CpuReservationRepositoryError::CreateDirectory {
                path: data_directory.to_path_buf(),
                source,
            }
        })?;
        let path = data_directory.join(STORE_FILE_NAME);
        let reservations = load_reservations(&path)?
            .into_iter()
            .map(|reservation| (reservation.reservation_id(), reservation))
            .collect();

        Ok(Self {
            reservations: Arc::new(Mutex::new(reservations)),
            path: Some(Arc::new(path)),
        })
    }

    /// 列出当前 Core 内所有 CPU 预留。
    pub(crate) fn list(&self) -> Result<Vec<CpuReservation>, CpuReservationRepositoryError> {
        let reservations = self
            .reservations
            .lock()
            .map_err(|_| CpuReservationRepositoryError::StorePoisoned)?;
        Ok(reservations.values().cloned().collect())
    }

    /// 原子创建或替换指定实例的 CPU 独占预留。
    pub(crate) fn reserve(
        &self,
        instance_id: InstanceId,
        cpu_ids: Vec<u32>,
        created_at: String,
    ) -> Result<CpuReservation, CpuReservationRepositoryError> {
        let mut reservations = self
            .reservations
            .lock()
            .map_err(|_| CpuReservationRepositoryError::StorePoisoned)?;
        let previous_reservation_id =
            reservations
                .iter()
                .find_map(|(reservation_id, reservation)| {
                    (reservation.instance_id() == &instance_id).then_some(*reservation_id)
                });
        let previous_reservation = previous_reservation_id
            .and_then(|reservation_id| reservations.get(&reservation_id).cloned());

        for (reservation_id, reservation) in &*reservations {
            if Some(*reservation_id) == previous_reservation_id {
                continue;
            }
            if let Some(cpu_id) = cpu_ids
                .iter()
                .find(|cpu_id| reservation.cpu_ids().contains(cpu_id))
            {
                return Err(CpuReservationRepositoryError::Conflict {
                    cpu_id: *cpu_id,
                    reservation_id: reservation.reservation_id(),
                });
            }
        }

        if let Some(previous_reservation_id) = previous_reservation_id {
            reservations.remove(&previous_reservation_id);
        }
        let reservation = CpuReservation::new(TaskId::new(), instance_id, cpu_ids, created_at);
        reservations.insert(reservation.reservation_id(), reservation.clone());
        if let Err(error) = self.persist(&reservations) {
            reservations.remove(&reservation.reservation_id());
            if let Some(previous) = previous_reservation {
                reservations.insert(previous.reservation_id(), previous);
            }
            return Err(error);
        }

        Ok(reservation)
    }

    /// 释放一个现有 CPU 预留。
    pub(crate) fn release(
        &self,
        reservation_id: TaskId,
    ) -> Result<CpuReservation, CpuReservationRepositoryError> {
        let mut reservations = self
            .reservations
            .lock()
            .map_err(|_| CpuReservationRepositoryError::StorePoisoned)?;
        let removed = reservations
            .remove(&reservation_id)
            .ok_or(CpuReservationRepositoryError::NotFound { reservation_id })?;
        if let Err(error) = self.persist(&reservations) {
            reservations.insert(removed.reservation_id(), removed.clone());
            return Err(error);
        }

        Ok(removed)
    }

    /// 将当前快照写入持久化文件；纯内存仓储不产生文件操作。
    fn persist(
        &self,
        reservations: &HashMap<TaskId, CpuReservation>,
    ) -> Result<(), CpuReservationRepositoryError> {
        let Some(path) = self.path.as_deref() else {
            return Ok(());
        };
        persist_reservations(path, reservations.values())
    }
}

/// 从磁盘加载 CPU 预留；缺失文件等价于首次启动。
fn load_reservations(path: &Path) -> Result<Vec<CpuReservation>, CpuReservationRepositoryError> {
    let content = match fs::read(path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(CpuReservationRepositoryError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    from_slice(&content).map_err(|source| CpuReservationRepositoryError::Decode {
        path: path.to_path_buf(),
        source,
    })
}

/// 将 CPU 预留完整快照写入同目录临时文件并原子替换正式文件。
fn persist_reservations<'a, I>(
    path: &Path,
    reservations: I,
) -> Result<(), CpuReservationRepositoryError>
where
    I: IntoIterator<Item = &'a CpuReservation>,
{
    let mut values = reservations.into_iter().cloned().collect::<Vec<_>>();
    values.sort_by_key(|reservation| reservation.reservation_id().to_string());

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temporary = NamedTempFile::new_in(parent).map_err(|source| {
        CpuReservationRepositoryError::CreateTemporary {
            path: parent.to_path_buf(),
            source,
        }
    })?;
    to_writer_pretty(temporary.as_file_mut(), &values).map_err(|source| {
        CpuReservationRepositoryError::Encode {
            path: path.to_path_buf(),
            source,
        }
    })?;
    temporary
        .write_all(b"\n")
        .and_then(|_| temporary.flush())
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|source| CpuReservationRepositoryError::Write {
            path: path.to_path_buf(),
            source,
        })?;
    temporary
        .persist(path)
        .map_err(|error| CpuReservationRepositoryError::Replace {
            path: path.to_path_buf(),
            source: error.error,
        })
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::CpuReservationRepository;
    use super::CpuReservationRepositoryError;
    use super::STORE_FILE_NAME;
    use nexus_domain::InstanceId;
    use tempfile::tempdir;

    #[test]
    fn rejects_overlapping_reservations_and_replaces_the_same_instance() {
        let repository = CpuReservationRepository::default();
        let first_instance = InstanceId::new("first".to_owned()).expect("instance ID is valid");
        let second_instance = InstanceId::new("second".to_owned()).expect("instance ID is valid");
        let first = repository
            .reserve(
                first_instance.clone(),
                vec![0, 1],
                "2026-08-05T00:00:00Z".to_owned(),
            )
            .expect("first reservation succeeds");

        let conflict = repository
            .reserve(
                second_instance,
                vec![1, 2],
                "2026-08-05T00:00:01Z".to_owned(),
            )
            .expect_err("overlapping reservation is rejected");
        assert!(matches!(
            conflict,
            CpuReservationRepositoryError::Conflict { cpu_id: 1, .. }
        ));

        let replacement = repository
            .reserve(first_instance, vec![2], "2026-08-05T00:00:02Z".to_owned())
            .expect("same instance can replace its reservation");
        assert_ne!(replacement.reservation_id(), first.reservation_id());
        assert_eq!(repository.list().expect("list succeeds").len(), 1);
    }

    #[test]
    fn reloads_persisted_reservations_after_repository_recreation() {
        let directory = tempdir().expect("temporary reservation directory is created");
        let instance_id = InstanceId::new("survival".to_owned()).expect("instance ID is valid");
        let repository =
            CpuReservationRepository::open(directory.path()).expect("repository opens");
        let expected = repository
            .reserve(instance_id, vec![2, 4], "2026-08-06T00:00:00Z".to_owned())
            .expect("reservation is persisted");
        drop(repository);

        let reloaded =
            CpuReservationRepository::open(directory.path()).expect("repository reloads");
        assert_eq!(
            reloaded.list().expect("reservations are listed"),
            vec![expected]
        );
    }

    #[test]
    fn refuses_to_open_a_corrupted_reservation_store() {
        let directory = tempdir().expect("temporary reservation directory is created");
        fs::write(directory.path().join(STORE_FILE_NAME), b"not-json")
            .expect("corrupted reservation store is written");

        assert!(matches!(
            CpuReservationRepository::open(directory.path()),
            Err(CpuReservationRepositoryError::Decode { .. })
        ));
    }
}
