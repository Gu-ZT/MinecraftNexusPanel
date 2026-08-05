//! Core 启动时读取宿主机 CPU 拓扑的保守探测器。
//!
//! 平台专用数据只在操作系统明确提供时才进入领域快照。尤其是性能类别、NUMA
//! 和隔离状态，缺失时必须保持未知，不能根据 CPU 编号、排列顺序或线程数量推断。

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::num::NonZeroUsize;
use std::path::Path;
use std::thread::available_parallelism;

use nexus_domain::CpuAvailability;
use nexus_domain::CpuLogicalProcessor;
use nexus_domain::CpuPerformanceClass;
use nexus_domain::CpuTopology;
use nexus_domain::CpuTopologyDetection;
use sysinfo::System;

const MAX_CPU_RANGE_SPAN: u32 = 65_536;
const MAX_CPU_LIST_LENGTH: usize = 1_000_000;

/// 探测 Core 宿主机当前可见的逻辑 CPU 和物理核心数量。
pub(crate) fn detect_cpu_topology() -> CpuTopology {
    if let Some(topology) = detect_linux_cpu_topology(
        Path::new("/sys/devices/system/cpu"),
        Path::new("/proc/self/status"),
    ) {
        return topology;
    }

    detect_system_cpu_topology()
}

/// 使用 Linux sysfs 和进程 cpuset 探测 CPU 拓扑。
///
/// 路径参数独立传入，使解析逻辑可以在所有构建平台上使用临时目录测试，
/// 也避免单元测试依赖运行测试的宿主机 CPU 型号。`None` 表示 sysfs 没有
/// 提供足够的逻辑 CPU 集合，调用方应退回标准库和 sysinfo 的基础结果。
fn detect_linux_cpu_topology(
    sysfs_cpu_root: &Path,
    proc_status_path: &Path,
) -> Option<CpuTopology> {
    let online_path = sysfs_cpu_root.join("online");
    let online_ids = read_cpu_list(&online_path);
    let possible_ids = read_cpu_list(&sysfs_cpu_root.join("possible"))
        .or_else(|| online_ids.clone())
        .filter(|ids| !ids.is_empty())?;
    let allowed_ids = read_allowed_cpu_ids(proc_status_path);
    let cpu_ids = visible_cpu_ids(&possible_ids, allowed_ids.as_deref());
    if cpu_ids.is_empty() {
        return None;
    }

    let isolated_ids = read_cpu_list(&sysfs_cpu_root.join("isolated"));
    let (performance_classes, performance_source) =
        linux_performance_classes(sysfs_cpu_root, &cpu_ids);
    let logical_cpus = cpu_ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let isolated = isolated_ids
                .as_ref()
                .map(|ids| ids.binary_search(id).is_ok());
            CpuLogicalProcessor::new(
                *id,
                read_physical_core_id(sysfs_cpu_root, *id),
                performance_classes[index],
                read_online_state(sysfs_cpu_root, *id, online_ids.as_deref()),
                isolated,
                read_numa_node(sysfs_cpu_root, *id),
            )
        })
        .collect::<Vec<_>>();

    let physical_core_ids = logical_cpus
        .iter()
        .filter_map(|cpu| cpu.physical_core_id())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let physical_core_count = if physical_core_ids.is_empty() {
        System::new_all().physical_core_count()
    } else {
        Some(physical_core_ids.len())
    };
    let detection_confidence = if performance_source != "LINUX_SYSFS_TOPOLOGY"
        && logical_cpus
            .iter()
            .all(|cpu| cpu.physical_core_id().is_some())
    {
        "HIGH"
    } else {
        "MEDIUM"
    };

    Some(CpuTopology::new(
        env::consts::ARCH.to_owned(),
        logical_cpus.clone(),
        physical_core_count,
        availability(&logical_cpus),
        CpuTopologyDetection::new(
            performance_source.to_owned(),
            detection_confidence.to_owned(),
        ),
    ))
}

/// 使用跨平台标准 API 生成基础拓扑，作为专用探测器不可用时的安全回退。
fn detect_system_cpu_topology() -> CpuTopology {
    let logical_cpu_count = available_parallelism().map(NonZeroUsize::get).unwrap_or(1);
    let logical_cpus = (0..logical_cpu_count)
        .filter_map(|id| u32::try_from(id).ok())
        .map(|id| {
            CpuLogicalProcessor::new(id, None, CpuPerformanceClass::Unknown, true, None, None)
        })
        .collect::<Vec<_>>();
    let physical_core_count = System::new_all().physical_core_count();

    CpuTopology::new(
        env::consts::ARCH.to_owned(),
        logical_cpus,
        physical_core_count,
        CpuAvailability::default(),
        CpuTopologyDetection::new("SYSTEM_API".to_owned(), "LOW".to_owned()),
    )
}

/// 将可能的物理 CPU 集合限制到当前进程被允许使用的 cpuset。
fn visible_cpu_ids(possible_ids: &[u32], allowed_ids: Option<&[u32]>) -> Vec<u32> {
    let allowed = allowed_ids.map(|ids| ids.iter().copied().collect::<BTreeSet<_>>());
    possible_ids
        .iter()
        .copied()
        .filter(|id| {
            allowed
                .as_ref()
                .is_none_or(|allowed_ids| allowed_ids.contains(id))
        })
        .collect()
}

/// 解析 Linux 的 `Cpus_allowed_list`，失败时返回 `None` 并让调用方保留全局集合。
fn read_allowed_cpu_ids(path: &Path) -> Option<Vec<u32>> {
    let contents = fs::read_to_string(path).ok()?;
    let value = contents.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "Cpus_allowed_list").then_some(value.trim())
    })?;
    parse_cpu_list(value).ok()
}

/// 读取 Linux 的 CPU 列表格式，例如 `0-3,8,10-11`。
fn read_cpu_list(path: &Path) -> Option<Vec<u32>> {
    let contents = fs::read_to_string(path).ok()?;
    parse_cpu_list(contents.trim()).ok()
}

fn parse_cpu_list(value: &str) -> Result<Vec<u32>, &'static str> {
    if value.is_empty() {
        return Ok(Vec::new());
    }

    let mut ids = BTreeSet::new();
    for item in value.split(',').map(str::trim) {
        if item.is_empty() {
            return Err("empty CPU list item");
        }
        if let Some((start, end)) = item.split_once('-') {
            let start = start
                .parse::<u32>()
                .map_err(|_| "invalid CPU range start")?;
            let end = end.parse::<u32>().map_err(|_| "invalid CPU range end")?;
            let Some(span) = end.checked_sub(start) else {
                return Err("reversed CPU range");
            };
            if span > MAX_CPU_RANGE_SPAN {
                return Err("CPU range is too large");
            }
            ids.extend(start..=end);
        } else {
            ids.insert(item.parse::<u32>().map_err(|_| "invalid CPU id")?);
        }
        if ids.len() > MAX_CPU_LIST_LENGTH {
            return Err("CPU list is too large");
        }
    }

    Ok(ids.into_iter().collect())
}

/// 读取单个 CPU 的在线状态；CPU 全局在线列表优先于 per-CPU 文件。
fn read_online_state(root: &Path, id: u32, online_ids: Option<&[u32]>) -> bool {
    if let Some(online_ids) = online_ids {
        return online_ids.binary_search(&id).is_ok();
    }

    read_trimmed(&root.join(format!("cpu{id}/online")))
        .map(|value| value == "1")
        .unwrap_or(true)
}

/// 组合 package ID 和 core ID，避免多路 CPU 中不同插槽的 core ID 重叠。
fn read_physical_core_id(root: &Path, id: u32) -> Option<String> {
    let topology = root.join(format!("cpu{id}/topology"));
    let core_id = read_trimmed(&topology.join("core_id"))?;
    let package_id = read_trimmed(&topology.join("physical_package_id"));
    Some(match package_id {
        Some(package_id) => format!("{package_id}:{core_id}"),
        None => core_id,
    })
}

/// 从 CPU 目录下的 `nodeN` 入口读取 NUMA 节点。
fn read_numa_node(root: &Path, id: u32) -> Option<u32> {
    fs::read_dir(root.join(format!("cpu{id}")))
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?.strip_prefix("node")?;
            name.parse::<u32>().ok()
        })
        .min()
}

/// 根据 Linux `core_type` 或 ARM capacity 文件生成性能类别。
fn linux_performance_classes(root: &Path, ids: &[u32]) -> (Vec<CpuPerformanceClass>, &'static str) {
    let core_types = ids
        .iter()
        .map(|id| read_number(&root.join(format!("cpu{id}/topology/core_type"))))
        .collect::<Vec<_>>();
    if core_types.iter().any(|value| matches!(value, Some(1 | 2))) {
        return (
            core_types
                .into_iter()
                .map(|value| match value {
                    Some(1) => CpuPerformanceClass::Performance,
                    Some(2) => CpuPerformanceClass::Efficiency,
                    _ => CpuPerformanceClass::Unknown,
                })
                .collect(),
            "LINUX_SYSFS_CORE_TYPE",
        );
    }

    let capacities = ids
        .iter()
        .map(|id| {
            read_number(&root.join(format!("cpu{id}/cpu_capacity")))
                .or_else(|| read_number(&root.join(format!("cpu{id}/cpu_capacity_orig"))))
        })
        .collect::<Vec<_>>();
    if capacities.iter().any(Option::is_some) {
        return (classify_capacities(&capacities), "LINUX_SYSFS_CPU_CAPACITY");
    }

    (
        vec![CpuPerformanceClass::Unknown; ids.len()],
        "LINUX_SYSFS_TOPOLOGY",
    )
}

/// 只把 capacity 的最高和最低明确值映射为性能核和能效核。
///
/// 多于两个 capacity 等级时，中间等级保持未知，避免把不完整的硬件层级
/// 强行压缩为二元分类。缺失值也保持未知。
fn classify_capacities(capacities: &[Option<u64>]) -> Vec<CpuPerformanceClass> {
    let known = capacities
        .iter()
        .filter_map(|value| *value)
        .collect::<BTreeSet<_>>();
    let (Some(minimum), Some(maximum)) = (known.first().copied(), known.last().copied()) else {
        return vec![CpuPerformanceClass::Unknown; capacities.len()];
    };
    if minimum == maximum {
        return vec![CpuPerformanceClass::Unknown; capacities.len()];
    }

    capacities
        .iter()
        .map(|value| match value {
            Some(value) if *value == maximum => CpuPerformanceClass::Performance,
            Some(value) if *value == minimum => CpuPerformanceClass::Efficiency,
            _ => CpuPerformanceClass::Unknown,
        })
        .collect()
}

fn read_number(path: &Path) -> Option<u64> {
    read_trimmed(path)?.parse::<u64>().ok()
}

fn read_trimmed(path: &Path) -> Option<String> {
    let value = fs::read_to_string(path).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

/// 只把在线且未被明确隔离的已分类 CPU 暴露给类别策略。
fn availability(logical_cpus: &[CpuLogicalProcessor]) -> CpuAvailability {
    let performance_cpu_ids = logical_cpus
        .iter()
        .filter(|cpu| {
            cpu.online()
                && cpu.isolated() != Some(true)
                && cpu.performance_class() == CpuPerformanceClass::Performance
        })
        .map(CpuLogicalProcessor::id)
        .collect();
    let efficiency_cpu_ids = logical_cpus
        .iter()
        .filter(|cpu| {
            cpu.online()
                && cpu.isolated() != Some(true)
                && cpu.performance_class() == CpuPerformanceClass::Efficiency
        })
        .map(CpuLogicalProcessor::id)
        .collect();

    CpuAvailability::new(performance_cpu_ids, efficiency_cpu_ids)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use nexus_domain::CpuPerformanceClass;

    use super::detect_cpu_topology;
    use super::detect_linux_cpu_topology;
    use super::parse_cpu_list;
    use super::visible_cpu_ids;

    #[test]
    fn parses_and_sorts_linux_cpu_lists() {
        assert_eq!(parse_cpu_list("6-7,0-2,4"), Ok(vec![0, 1, 2, 4, 6, 7]));
        assert!(parse_cpu_list("4-2").is_err());
        assert!(parse_cpu_list("cpu").is_err());
    }

    #[test]
    fn restricts_possible_cpus_to_the_process_cpuset() {
        let possible = vec![0, 1, 2, 3];
        let allowed = vec![1, 3];

        assert_eq!(visible_cpu_ids(&possible, Some(&allowed)), vec![1, 3]);
        assert_eq!(visible_cpu_ids(&possible, None), possible);
    }

    #[test]
    fn reads_linux_core_type_numa_online_and_isolation() {
        let directory = tempfile::tempdir().expect("temporary sysfs root is created");
        let root = directory.path();
        write_value(&root.join("possible"), "0-3\n");
        write_value(&root.join("isolated"), "2\n");
        write_value(
            &root.join("status"),
            "Name:\ttest\nCpus_allowed_list:\t0-3\n",
        );
        for id in 0..4 {
            let cpu = root.join(format!("cpu{id}"));
            write_value(&cpu.join("topology/core_id"), &(id / 2).to_string());
            write_value(&cpu.join("topology/physical_package_id"), "0");
            write_value(
                &cpu.join("topology/core_type"),
                if id < 2 { "1" } else { "2" },
            );
            write_value(&cpu.join("online"), if id < 3 { "1" } else { "0" });
            fs::create_dir_all(cpu.join(format!("node{}", id / 2)))
                .expect("NUMA node entry is created");
        }

        let topology = detect_linux_cpu_topology(root, &root.join("status"))
            .expect("synthetic Linux topology is detected");

        assert_eq!(topology.physical_core_count(), Some(2));
        assert_eq!(topology.detection().source(), "LINUX_SYSFS_CORE_TYPE");
        assert_eq!(topology.detection().confidence(), "HIGH");
        assert_eq!(topology.logical_cpus()[0].physical_core_id(), Some("0:0"));
        assert_eq!(topology.logical_cpus()[2].numa_node(), Some(1));
        assert_eq!(topology.logical_cpus()[2].isolated(), Some(true));
        assert!(!topology.logical_cpus()[3].online());
        assert_eq!(
            topology.logical_cpus()[0].performance_class(),
            CpuPerformanceClass::Performance
        );
        assert_eq!(
            topology.logical_cpus()[2].performance_class(),
            CpuPerformanceClass::Efficiency
        );
        assert_eq!(topology.available().performance_cpu_ids(), &[0, 1]);
        assert!(topology.available().efficiency_cpu_ids().is_empty());
    }

    #[test]
    fn classifies_arm_capacity_without_guessing_middle_levels() {
        let directory = tempfile::tempdir().expect("temporary sysfs root is created");
        let root = directory.path();
        write_value(&root.join("possible"), "0-3\n");
        write_value(&root.join("online"), "0-3\n");
        write_value(&root.join("status"), "Cpus_allowed_list:\t0-3\n");
        for (id, capacity) in [(0, "1024"), (1, "1024"), (2, "512"), (3, "256")] {
            write_value(&root.join(format!("cpu{id}/cpu_capacity")), capacity);
        }

        let topology = detect_linux_cpu_topology(root, &root.join("status"))
            .expect("synthetic ARM capacity is detected");

        assert_eq!(topology.detection().source(), "LINUX_SYSFS_CPU_CAPACITY");
        assert_eq!(
            topology.logical_cpus()[0].performance_class(),
            CpuPerformanceClass::Performance
        );
        assert_eq!(
            topology.logical_cpus()[2].performance_class(),
            CpuPerformanceClass::Unknown
        );
        assert_eq!(
            topology.logical_cpus()[3].performance_class(),
            CpuPerformanceClass::Efficiency
        );
        assert!(
            topology
                .logical_cpus()
                .iter()
                .all(|cpu| cpu.isolated().is_none())
        );
    }

    #[test]
    fn returns_a_nonempty_snapshot_without_inventing_cpu_ids() {
        let topology = detect_cpu_topology();
        let mut ids = topology
            .logical_cpus()
            .iter()
            .map(|cpu| cpu.id())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();

        assert!(!ids.is_empty());
        assert_eq!(ids.len(), topology.logical_cpu_count());
    }

    fn write_value(path: &Path, value: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("synthetic sysfs parent is created");
        }
        fs::write(path, value).expect("synthetic sysfs value is written");
    }
}
