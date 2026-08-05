use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use nexus_domain::CpuReservation;
use nexus_domain::InstanceId;
use nexus_domain::TaskId;
use thiserror::Error;

/// Core 内存中的 CPU 独占预留仓储。
///
/// 仓储只负责同一 Core 进程内的原子冲突检查；重启持久化、跨 Core 调度和
/// 操作系统 affinity 应用由后续资源治理模块负责。
#[derive(Clone, Default)]
pub(crate) struct CpuReservationRepository {
    reservations: Arc<Mutex<HashMap<TaskId, CpuReservation>>>,
}

/// CPU 预留仓储操作错误。
#[derive(Debug, Error)]
pub(crate) enum CpuReservationRepositoryError {
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
}

impl CpuReservationRepository {
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
        reservations
            .remove(&reservation_id)
            .ok_or(CpuReservationRepositoryError::NotFound { reservation_id })
    }
}

#[cfg(test)]
mod tests {
    use super::CpuReservationRepository;
    use super::CpuReservationRepositoryError;
    use nexus_domain::InstanceId;

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
}
