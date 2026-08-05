use std::collections::VecDeque;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use nexus_domain::InstanceAuditPage;
use nexus_domain::InstanceAuditRecord;
use nexus_domain::InstanceId;
use serde_json::from_slice;
use serde_json::to_writer_pretty;
use tempfile::NamedTempFile;
use thiserror::Error;

const MAXIMUM_RECORDS: usize = 2048;
const STORE_FILE_NAME: &str = "instance-audit.json";

/// Core 的实例生命周期审计仓储。
///
/// 审计记录按产生顺序保留，并在查询时按最新记录优先返回。仓储使用数据目录
/// 下的 JSON 文件保存记录，追加通过同目录临时文件和原子替换完成，避免 Core
/// 重启后丢失已经确认的生命周期事实。
#[derive(Clone)]
pub(crate) struct InstanceAuditRepository {
    records: Arc<Mutex<VecDeque<InstanceAuditRecord>>>,
    path: Arc<PathBuf>,
}

/// 实例审计仓储操作错误。
#[derive(Debug, Error)]
pub enum InstanceAuditRepositoryError {
    /// 审计仓储锁被污染。
    #[error("instance audit store lock is poisoned")]
    StorePoisoned,
    /// 审计数据目录创建失败。
    #[error("failed to create instance audit data directory {path}")]
    CreateDirectory {
        /// 创建失败的数据目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的目录创建错误。
        source: io::Error,
    },
    /// 读取已有审计文件失败。
    #[error("failed to read instance audit store {path}")]
    Read {
        /// 读取失败的审计文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的读取错误。
        source: io::Error,
    },
    /// 已有审计文件不是合法的记录数组。
    #[error("instance audit store {path} contains invalid JSON")]
    Decode {
        /// 包含非法 JSON 的审计文件路径。
        path: PathBuf,
        #[source]
        /// JSON 解码错误。
        source: serde_json::Error,
    },
    /// 创建审计临时文件失败。
    #[error("failed to create temporary instance audit store in {path}")]
    CreateTemporary {
        /// 创建临时文件的目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的创建错误。
        source: io::Error,
    },
    /// 序列化审计记录失败。
    #[error("failed to encode instance audit store {path}")]
    Encode {
        /// 无法写入的审计文件路径。
        path: PathBuf,
        #[source]
        /// JSON 编码错误。
        source: serde_json::Error,
    },
    /// 写入或同步临时审计文件失败。
    #[error("failed to write instance audit store {path}")]
    Write {
        /// 写入失败的目标审计文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的写入错误。
        source: io::Error,
    },
    /// 用临时文件替换正式审计文件失败。
    #[error("failed to atomically replace instance audit store {path}")]
    Replace {
        /// 替换失败的正式审计文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的替换错误。
        source: io::Error,
    },
}

impl InstanceAuditRepository {
    /// 打开数据目录中的审计仓储，并加载已有记录。
    ///
    /// 文件不存在时表示首次启动，仓储从空队列开始；文件存在但损坏时直接
    /// 返回错误，避免把不完整的审计历史静默当成可信数据继续运行。
    pub(crate) fn open(data_directory: &Path) -> Result<Self, InstanceAuditRepositoryError> {
        fs::create_dir_all(data_directory).map_err(|source| {
            InstanceAuditRepositoryError::CreateDirectory {
                path: data_directory.to_path_buf(),
                source,
            }
        })?;
        let path = data_directory.join(STORE_FILE_NAME);
        let records = load_records(&path)?;

        Ok(Self {
            records: Arc::new(Mutex::new(records)),
            path: Arc::new(path),
        })
    }

    /// 追加一条审计记录，并按容量淘汰最早记录。
    pub(crate) fn append(
        &self,
        record: InstanceAuditRecord,
    ) -> Result<(), InstanceAuditRepositoryError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| InstanceAuditRepositoryError::StorePoisoned)?;
        let previous = records.clone();
        records.push_back(record);
        trim_records(&mut records);
        if let Err(error) = persist_records(&self.path, &records) {
            *records = previous;
            return Err(error);
        }

        Ok(())
    }

    /// 按实例读取最新的审计记录。
    pub(crate) fn list(
        &self,
        instance_id: &InstanceId,
        limit: usize,
    ) -> Result<InstanceAuditPage, InstanceAuditRepositoryError> {
        let records = self
            .records
            .lock()
            .map_err(|_| InstanceAuditRepositoryError::StorePoisoned)?;
        let items = records
            .iter()
            .rev()
            .filter(|record| record.instance_id() == instance_id)
            .take(limit)
            .cloned()
            .collect();

        Ok(InstanceAuditPage::new(items, None))
    }
}

/// 从磁盘加载审计记录；缺失文件等价于首次启动。
fn load_records(
    path: &Path,
) -> Result<VecDeque<InstanceAuditRecord>, InstanceAuditRepositoryError> {
    let content = match fs::read(path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(VecDeque::new()),
        Err(source) => {
            return Err(InstanceAuditRepositoryError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let mut records =
        from_slice(&content).map_err(|source| InstanceAuditRepositoryError::Decode {
            path: path.to_path_buf(),
            source,
        })?;
    trim_records(&mut records);
    Ok(records)
}

/// 将队列限制在持久化保留上限内，始终淘汰最早的记录。
fn trim_records(records: &mut VecDeque<InstanceAuditRecord>) {
    while records.len() > MAXIMUM_RECORDS {
        records.pop_front();
    }
}

/// 将完整快照写入同目录临时文件并原子替换正式文件。
///
/// 临时文件与目标文件必须处于同一目录，才能保证替换动作不跨文件系统；
/// `sync_all` 则让返回成功时的内容已经交给操作系统持久化路径。
fn persist_records(
    path: &Path,
    records: &VecDeque<InstanceAuditRecord>,
) -> Result<(), InstanceAuditRepositoryError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temporary = NamedTempFile::new_in(parent).map_err(|source| {
        InstanceAuditRepositoryError::CreateTemporary {
            path: parent.to_path_buf(),
            source,
        }
    })?;
    to_writer_pretty(temporary.as_file_mut(), records).map_err(|source| {
        InstanceAuditRepositoryError::Encode {
            path: path.to_path_buf(),
            source,
        }
    })?;
    temporary
        .write_all(b"\n")
        .and_then(|_| temporary.flush())
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|source| InstanceAuditRepositoryError::Write {
            path: path.to_path_buf(),
            source,
        })?;
    temporary
        .persist(path)
        .map_err(|error| InstanceAuditRepositoryError::Replace {
            path: path.to_path_buf(),
            source: error.error,
        })
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use nexus_domain::InstanceAuditAction;
    use nexus_domain::InstanceAuditOutcome;
    use nexus_domain::InstanceId;
    use nexus_domain::RuntimeMode;
    use nexus_domain::SupervisorMode;
    use tempfile::tempdir;

    use super::InstanceAuditRecord;
    use super::InstanceAuditRepository;
    use super::InstanceAuditRepositoryError;
    use super::STORE_FILE_NAME;

    #[test]
    fn lists_latest_records_first_and_filters_by_instance() {
        let directory = tempdir().expect("temporary audit directory is created");
        let repository =
            InstanceAuditRepository::open(directory.path()).expect("audit repository opens");
        let instance_id = InstanceId::new("survival".to_owned()).expect("instance ID is valid");
        let other_instance =
            InstanceId::new("creative".to_owned()).expect("other instance ID is valid");
        repository
            .append(record(instance_id.clone(), "2026-08-05T00:00:00Z"))
            .expect("first record is appended");
        repository
            .append(record(other_instance, "2026-08-05T00:00:01Z"))
            .expect("unrelated record is appended");
        repository
            .append(record(instance_id.clone(), "2026-08-05T00:00:02Z"))
            .expect("second record is appended");

        let page = repository
            .list(&instance_id, 10)
            .expect("audit page is read");

        assert_eq!(page.items().len(), 2);
        assert_eq!(page.items()[0].occurred_at(), "2026-08-05T00:00:02Z");
        assert_eq!(page.items()[1].occurred_at(), "2026-08-05T00:00:00Z");
    }

    #[test]
    fn reloads_records_after_repository_recreation() {
        let directory = tempdir().expect("temporary audit directory is created");
        let instance_id = InstanceId::new("survival".to_owned()).expect("instance ID is valid");
        let expected = record(instance_id.clone(), "2026-08-05T00:00:00Z");
        let repository =
            InstanceAuditRepository::open(directory.path()).expect("audit repository opens");
        repository
            .append(expected.clone())
            .expect("audit record is persisted");
        drop(repository);

        let reloaded =
            InstanceAuditRepository::open(directory.path()).expect("audit repository reloads");
        let page = reloaded
            .list(&instance_id, 10)
            .expect("persisted audit page is read");

        assert_eq!(page.items(), &[expected]);
    }

    #[test]
    fn refuses_to_start_with_corrupted_store() {
        let directory = tempdir().expect("temporary audit directory is created");
        fs::write(directory.path().join(STORE_FILE_NAME), b"not-json")
            .expect("corrupted audit store is written");

        assert!(matches!(
            InstanceAuditRepository::open(directory.path()),
            Err(InstanceAuditRepositoryError::Decode { .. })
        ));
    }

    fn record(instance_id: InstanceId, occurred_at: &str) -> InstanceAuditRecord {
        InstanceAuditRecord::new(
            instance_id,
            None,
            InstanceAuditAction::Start,
            InstanceAuditOutcome::Succeeded,
            RuntimeMode::Host,
            SupervisorMode::Direct,
            None,
            occurred_at.to_owned(),
        )
    }
}
