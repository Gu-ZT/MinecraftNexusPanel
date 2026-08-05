use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

use nexus_domain::InstanceAuditPage;
use nexus_domain::InstanceAuditRecord;
use nexus_domain::InstanceId;
use thiserror::Error;

const MAXIMUM_RECORDS: usize = 2048;

/// Core 内存中的实例生命周期审计仓储。
///
/// 审计记录按产生顺序保留，并在查询时按最新记录优先返回。当前仓储随 Core
/// 进程生命周期存在；跨重启持久化需要后续统一审计存储和保留策略支持。
#[derive(Clone)]
pub(crate) struct InstanceAuditRepository {
    records: Arc<Mutex<VecDeque<InstanceAuditRecord>>>,
}

impl Default for InstanceAuditRepository {
    fn default() -> Self {
        Self {
            records: Arc::new(Mutex::new(VecDeque::with_capacity(MAXIMUM_RECORDS))),
        }
    }
}

/// 实例审计仓储操作错误。
#[derive(Debug, Error)]
pub(crate) enum InstanceAuditRepositoryError {
    /// 审计仓储锁被污染。
    #[error("instance audit store lock is poisoned")]
    StorePoisoned,
}

impl InstanceAuditRepository {
    /// 追加一条审计记录，并按容量淘汰最早记录。
    pub(crate) fn append(
        &self,
        record: InstanceAuditRecord,
    ) -> Result<(), InstanceAuditRepositoryError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| InstanceAuditRepositoryError::StorePoisoned)?;
        records.push_back(record);
        while records.len() > MAXIMUM_RECORDS {
            records.pop_front();
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

#[cfg(test)]
mod tests {
    use nexus_domain::InstanceAuditAction;
    use nexus_domain::InstanceAuditOutcome;
    use nexus_domain::InstanceId;
    use nexus_domain::RuntimeMode;
    use nexus_domain::SupervisorMode;

    use super::InstanceAuditRecord;
    use super::InstanceAuditRepository;

    #[test]
    fn lists_latest_records_first_and_filters_by_instance() {
        let repository = InstanceAuditRepository::default();
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
