use serde::Deserialize;
use serde::Serialize;

use crate::CpuAvailability;
use crate::CpuLogicalProcessor;
use crate::CpuTopologyDetection;

/// Core 宿主机 CPU 拓扑的只读快照。
///
/// 可选字段表示当前操作系统没有提供可靠映射，而不是允许调用方猜测。
/// 只有 `CpuPerformanceClass::Performance` 或 `Efficiency` 的逻辑 CPU 才能
/// 被对应的严格 CPU policy 选中。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuTopology {
    architecture: String,
    logical_cpus: Vec<CpuLogicalProcessor>,
    physical_core_count: Option<usize>,
    available: CpuAvailability,
    detection: CpuTopologyDetection,
}

impl CpuTopology {
    /// 创建 CPU 拓扑快照。
    #[must_use]
    pub fn new(
        architecture: String,
        logical_cpus: Vec<CpuLogicalProcessor>,
        physical_core_count: Option<usize>,
        available: CpuAvailability,
        detection: CpuTopologyDetection,
    ) -> Self {
        Self {
            architecture,
            logical_cpus,
            physical_core_count,
            available,
            detection,
        }
    }

    /// 返回目标平台架构名称。
    #[must_use]
    pub fn architecture(&self) -> &str {
        &self.architecture
    }

    /// 返回逻辑 CPU 描述，顺序保持探测器提供的稳定顺序。
    #[must_use]
    pub fn logical_cpus(&self) -> &[CpuLogicalProcessor] {
        &self.logical_cpus
    }

    /// 返回逻辑 CPU 数量。
    #[must_use]
    pub fn logical_cpu_count(&self) -> usize {
        self.logical_cpus.len()
    }

    /// 返回物理核心数量；平台未提供时为 `None`。
    #[must_use]
    pub const fn physical_core_count(&self) -> Option<usize> {
        self.physical_core_count
    }

    /// 返回可用于性能/能效策略的 CPU 集合。
    #[must_use]
    pub const fn available(&self) -> &CpuAvailability {
        &self.available
    }

    /// 返回探测来源和置信度。
    #[must_use]
    pub const fn detection(&self) -> &CpuTopologyDetection {
        &self.detection
    }
}

#[cfg(test)]
mod tests {
    use super::CpuTopology;
    use crate::CpuAvailability;
    use crate::CpuLogicalProcessor;
    use crate::CpuPerformanceClass;
    use crate::CpuTopologyDetection;

    #[test]
    fn keeps_unknown_performance_class_explicit() {
        let topology = CpuTopology::new(
            "x86_64".to_owned(),
            vec![CpuLogicalProcessor::new(
                0,
                None,
                CpuPerformanceClass::Unknown,
                true,
                None,
                None,
            )],
            Some(1),
            CpuAvailability::default(),
            CpuTopologyDetection::new("SYSTEM_API".to_owned(), "LOW".to_owned()),
        );

        assert_eq!(topology.logical_cpu_count(), 1);
        assert_eq!(
            topology.logical_cpus()[0].performance_class(),
            CpuPerformanceClass::Unknown
        );
        assert!(topology.available().performance_cpu_ids().is_empty());
        assert!(topology.available().efficiency_cpu_ids().is_empty());
    }
}
