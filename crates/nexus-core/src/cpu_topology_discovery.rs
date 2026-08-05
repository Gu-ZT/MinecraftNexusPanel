//! Core 启动时读取宿主机 CPU 拓扑的保守探测器。
//!
//! 当前基础实现只采信标准库可用并行度和 sysinfo 物理核心数量；性能类别、NUMA
//! 映射、在线状态和隔离状态在操作系统专用探测器接入前保持未知或默认值。

use std::env;
use std::num::NonZeroUsize;
use std::thread::available_parallelism;

use nexus_domain::CpuAvailability;
use nexus_domain::CpuLogicalProcessor;
use nexus_domain::CpuPerformanceClass;
use nexus_domain::CpuTopology;
use nexus_domain::CpuTopologyDetection;
use sysinfo::System;

/// 探测 Core 宿主机当前可见的逻辑 CPU 和物理核心数量。
pub(crate) fn detect_cpu_topology() -> CpuTopology {
    let logical_cpu_count = available_parallelism().map(NonZeroUsize::get).unwrap_or(1);
    let logical_cpus = (0..logical_cpu_count)
        .filter_map(|id| u32::try_from(id).ok())
        .map(|id| {
            CpuLogicalProcessor::new(id, None, CpuPerformanceClass::Unknown, true, false, None)
        })
        .collect();
    let physical_core_count = System::new_all().physical_core_count();

    CpuTopology::new(
        env::consts::ARCH.to_owned(),
        logical_cpus,
        physical_core_count,
        CpuAvailability::default(),
        CpuTopologyDetection::new("SYSTEM_API".to_owned(), "LOW".to_owned()),
    )
}

#[cfg(test)]
mod tests {
    use super::detect_cpu_topology;
    use nexus_domain::CpuPerformanceClass;

    #[test]
    fn reports_visible_logical_cpus_without_guessing_performance_class() {
        let topology = detect_cpu_topology();

        assert!(!topology.logical_cpus().is_empty());
        assert!(
            topology
                .logical_cpus()
                .iter()
                .all(|cpu| cpu.performance_class() == CpuPerformanceClass::Unknown)
        );
        assert!(topology.available().performance_cpu_ids().is_empty());
        assert!(topology.available().efficiency_cpu_ids().is_empty());
    }
}
