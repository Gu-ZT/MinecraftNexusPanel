use serde::Deserialize;
use serde::Serialize;

use crate::InstanceId;
use crate::TaskId;

/// 记录一次 CPU 独占预留的领域快照。
///
/// 预留记录只表示 Core 已经为指定实例登记了不重叠的 CPU 集合；它不等于
/// 操作系统 affinity 已经成功应用。应用状态必须由后续资源执行器单独报告。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuReservation {
    reservation_id: TaskId,
    instance_id: InstanceId,
    cpu_ids: Vec<u32>,
    created_at: String,
}

impl CpuReservation {
    /// 创建 CPU 独占预留记录。
    #[must_use]
    pub fn new(
        reservation_id: TaskId,
        instance_id: InstanceId,
        cpu_ids: Vec<u32>,
        created_at: String,
    ) -> Self {
        Self {
            reservation_id,
            instance_id,
            cpu_ids,
            created_at,
        }
    }

    /// 返回预留标识。
    #[must_use]
    pub const fn reservation_id(&self) -> TaskId {
        self.reservation_id
    }

    /// 返回占用该集合的实例标识。
    #[must_use]
    pub fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    /// 返回已登记的 CPU ID 集合。
    #[must_use]
    pub fn cpu_ids(&self) -> &[u32] {
        &self.cpu_ids
    }

    /// 返回预留创建时间。
    #[must_use]
    pub fn created_at(&self) -> &str {
        &self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::CpuReservation;
    use crate::InstanceId;
    use crate::TaskId;

    #[test]
    fn preserves_the_reserved_instance_and_cpu_set() {
        let instance_id = InstanceId::new("survival".to_owned()).expect("instance ID is valid");
        let reservation = CpuReservation::new(
            TaskId::new(),
            instance_id.clone(),
            vec![2, 4],
            "2026-08-05T00:00:00Z".to_owned(),
        );

        assert_eq!(reservation.instance_id(), &instance_id);
        assert_eq!(reservation.cpu_ids(), &[2, 4]);
    }
}
