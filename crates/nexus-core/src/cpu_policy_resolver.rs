//! CPU policy 的只读解析器。
//!
//! 解析器只生成候选和建议集合，不修改进程 affinity，也不创建独占预留。
//! 这两个副作用必须由后续宿主机/容器资源管理器在确认请求后执行。

use std::collections::BTreeSet;

use nexus_domain::CpuLogicalProcessor;
use nexus_domain::CpuPerformanceClass;
use nexus_domain::CpuPolicy;
use nexus_domain::CpuPolicyError;
use nexus_domain::CpuPolicyMode;
use nexus_domain::CpuShareMode;
use nexus_domain::CpuTopology;
use serde_json::Value;
use serde_json::json;
use thiserror::Error;

/// CPU policy 只读解析失败原因。
#[derive(Debug, Error)]
pub(crate) enum CpuPolicyResolveError {
    /// 请求没有通过领域字段校验。
    #[error("CPU policy is invalid: {0}")]
    Invalid(#[from] CpuPolicyError),
    /// 宿主机无法提供严格请求所需的 CPU 容量信息。
    #[error("CPU policy capacity is unavailable: {0}")]
    CapacityUnavailable(&'static str),
}

/// 根据 Core 缓存的拓扑生成 CPU policy 候选和建议集合。
pub(crate) fn resolve_cpu_policy(
    policy: &CpuPolicy,
    topology: &CpuTopology,
) -> Result<Value, CpuPolicyResolveError> {
    policy.validate()?;

    let eligible = topology
        .logical_cpus()
        .iter()
        .filter(|cpu| cpu.online() && cpu.isolated() != Some(true))
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        return Err(CpuPolicyResolveError::CapacityUnavailable("NO_ONLINE_CPU"));
    }

    let mut degraded_reasons = Vec::new();
    let mut candidates = match policy.mode() {
        CpuPolicyMode::Auto => {
            let performance = eligible
                .iter()
                .copied()
                .filter(|cpu| cpu.performance_class() == CpuPerformanceClass::Performance)
                .collect::<Vec<_>>();
            if performance.is_empty() {
                degraded_reasons.push("PERFORMANCE_CLASS_UNKNOWN");
                eligible.clone()
            } else {
                performance
            }
        }
        CpuPolicyMode::Performance => class_candidates(
            &eligible,
            CpuPerformanceClass::Performance,
            policy.strict(),
            &mut degraded_reasons,
        )?,
        CpuPolicyMode::Efficiency => class_candidates(
            &eligible,
            CpuPerformanceClass::Efficiency,
            policy.strict(),
            &mut degraded_reasons,
        )?,
        CpuPolicyMode::Custom => {
            custom_candidates(policy, topology, policy.strict(), &mut degraded_reasons)?
        }
    };

    if let Some(numa_node) = policy.numa_node() {
        let has_unknown_numa = candidates.iter().any(|cpu| cpu.numa_node().is_none());
        if has_unknown_numa && policy.strict() {
            return Err(CpuPolicyResolveError::CapacityUnavailable(
                "NUMA_INFORMATION_UNKNOWN",
            ));
        }
        if has_unknown_numa {
            degraded_reasons.push("NUMA_INFORMATION_UNKNOWN");
        }
        candidates.retain(|cpu| cpu.numa_node() == Some(numa_node));
    }

    if candidates.len() < policy.min_cpus() {
        if policy.strict() {
            return Err(CpuPolicyResolveError::CapacityUnavailable(
                "MINIMUM_CPU_COUNT_UNAVAILABLE",
            ));
        }
        degraded_reasons.push("MINIMUM_CPU_COUNT_UNAVAILABLE");
    }

    if policy.share_mode() == CpuShareMode::Exclusive {
        if policy.strict() {
            return Err(CpuPolicyResolveError::CapacityUnavailable(
                "CPU_RESERVATION_UNAVAILABLE",
            ));
        }
        degraded_reasons.push("CPU_RESERVATION_NOT_IMPLEMENTED");
    }

    order_candidates(&mut candidates, policy.prefer_physical_cores());
    let selected_cpu_ids = candidates
        .iter()
        .take(policy.min_cpus())
        .map(|cpu| cpu.id())
        .collect::<Vec<_>>();
    let candidate_cpu_ids = candidates.iter().map(|cpu| cpu.id()).collect::<Vec<_>>();
    let performance_class = selected_performance_class(&candidates, policy.min_cpus());
    let degraded_reason = (!degraded_reasons.is_empty()).then(|| degraded_reasons.join(";"));

    Ok(json!({
        "requested": policy,
        "candidateCpuIds": candidate_cpu_ids,
        "selectedCpuIds": selected_cpu_ids,
        "performanceClass": performance_class,
        "conflicts": degraded_reasons,
        "degradedReason": degraded_reason,
        "reservationId": Value::Null,
    }))
}

fn class_candidates<'a>(
    eligible: &[&'a CpuLogicalProcessor],
    class: CpuPerformanceClass,
    strict: bool,
    degraded_reasons: &mut Vec<&'static str>,
) -> Result<Vec<&'a CpuLogicalProcessor>, CpuPolicyResolveError> {
    let candidates = eligible
        .iter()
        .copied()
        .filter(|cpu| cpu.performance_class() == class)
        .collect::<Vec<_>>();
    if !candidates.is_empty() {
        return Ok(candidates);
    }
    if strict {
        return Err(CpuPolicyResolveError::CapacityUnavailable(
            "PERFORMANCE_CLASS_UNAVAILABLE",
        ));
    }

    degraded_reasons.push("PERFORMANCE_CLASS_UNAVAILABLE");
    Ok(eligible.to_vec())
}

fn custom_candidates<'a>(
    policy: &CpuPolicy,
    topology: &'a CpuTopology,
    strict: bool,
    degraded_reasons: &mut Vec<&'static str>,
) -> Result<Vec<&'a CpuLogicalProcessor>, CpuPolicyResolveError> {
    let mut candidates = Vec::new();
    let mut invalid = false;
    for requested_id in policy.requested_cpu_ids() {
        let Some(cpu) = topology
            .logical_cpus()
            .iter()
            .find(|cpu| cpu.id() == *requested_id)
        else {
            invalid = true;
            continue;
        };
        if !cpu.online() || cpu.isolated() == Some(true) {
            invalid = true;
            continue;
        }
        candidates.push(cpu);
    }
    if invalid && strict {
        return Err(CpuPolicyResolveError::CapacityUnavailable(
            "CUSTOM_CPU_UNAVAILABLE",
        ));
    }
    if invalid {
        degraded_reasons.push("CUSTOM_CPU_UNAVAILABLE");
    }
    Ok(candidates)
}

fn order_candidates(candidates: &mut Vec<&CpuLogicalProcessor>, prefer_physical_cores: bool) {
    if !prefer_physical_cores {
        candidates.sort_by_key(|cpu| cpu.id());
        return;
    }

    let mut ordered = Vec::with_capacity(candidates.len());
    let mut physical_cores = BTreeSet::new();
    for cpu in candidates.iter().copied() {
        let Some(physical_core_id) = cpu.physical_core_id() else {
            continue;
        };
        if physical_cores.insert(physical_core_id.to_owned()) {
            ordered.push(cpu);
        }
    }
    for cpu in candidates.iter().copied() {
        if !ordered.iter().any(|selected| selected.id() == cpu.id()) {
            ordered.push(cpu);
        }
    }
    *candidates = ordered;
}

fn selected_performance_class(
    candidates: &[&CpuLogicalProcessor],
    selected_count: usize,
) -> &'static str {
    let mut classes = candidates
        .iter()
        .take(selected_count)
        .map(|cpu| cpu.performance_class());
    let Some(first) = classes.next() else {
        return "UNKNOWN";
    };
    if classes.all(|class| class == first) {
        match first {
            CpuPerformanceClass::Performance => "PERFORMANCE",
            CpuPerformanceClass::Efficiency => "EFFICIENCY",
            CpuPerformanceClass::Unknown => "UNKNOWN",
        }
    } else {
        "UNKNOWN"
    }
}

#[cfg(test)]
mod tests {
    use nexus_domain::CpuAvailability;
    use nexus_domain::CpuLogicalProcessor;
    use nexus_domain::CpuPerformanceClass;
    use nexus_domain::CpuPolicy;
    use nexus_domain::CpuPolicyMode;
    use nexus_domain::CpuTopology;
    use nexus_domain::CpuTopologyDetection;

    use super::resolve_cpu_policy;

    #[test]
    fn resolves_performance_cores_before_siblings() {
        let policy = CpuPolicy::new(
            CpuPolicyMode::Performance,
            Vec::new(),
            2,
            Some(2),
            true,
            None,
            Default::default(),
            true,
        );
        let topology = topology(vec![
            processor(0, "0", CpuPerformanceClass::Performance),
            processor(1, "0", CpuPerformanceClass::Performance),
            processor(2, "1", CpuPerformanceClass::Performance),
        ]);

        let result = resolve_cpu_policy(&policy, &topology).expect("policy resolves");
        assert_eq!(result["candidateCpuIds"], serde_json::json!([0, 2, 1]));
        assert_eq!(result["selectedCpuIds"], serde_json::json!([0, 2]));
    }

    #[test]
    fn rejects_strict_performance_selection_when_class_is_unknown() {
        let policy = CpuPolicy::new(
            CpuPolicyMode::Performance,
            Vec::new(),
            1,
            None,
            true,
            None,
            Default::default(),
            true,
        );
        let topology = topology(vec![processor(0, "0", CpuPerformanceClass::Unknown)]);

        assert!(resolve_cpu_policy(&policy, &topology).is_err());
    }

    fn topology(logical_cpus: Vec<CpuLogicalProcessor>) -> CpuTopology {
        CpuTopology::new(
            "x86_64".to_owned(),
            logical_cpus,
            Some(2),
            CpuAvailability::default(),
            CpuTopologyDetection::new("TEST".to_owned(), "HIGH".to_owned()),
        )
    }

    fn processor(
        id: u32,
        physical_core_id: &str,
        class: CpuPerformanceClass,
    ) -> CpuLogicalProcessor {
        CpuLogicalProcessor::new(
            id,
            Some(physical_core_id.to_owned()),
            class,
            true,
            Some(false),
            Some(0),
        )
    }
}
