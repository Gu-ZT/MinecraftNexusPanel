use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;

use crate::CpuPolicyError;
use crate::CpuPolicyMode;
use crate::CpuShareMode;

/// 描述实例请求的 CPU 选择、NUMA 和共享策略。
///
/// 该值只表达请求，不代表 affinity 或独占预留已经应用。Core 必须返回
/// requested 与 applied 的区别，并在无法确认性能类别、NUMA 或独占条件时
/// 根据 `strict` 返回失败或显式降级。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuPolicy {
    mode: CpuPolicyMode,
    requested_cpu_ids: Vec<u32>,
    min_cpus: usize,
    max_cpus: Option<usize>,
    prefer_physical_cores: bool,
    numa_node: Option<u32>,
    share_mode: CpuShareMode,
    strict: bool,
}

impl CpuPolicy {
    /// 创建 CPU policy 请求。
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        mode: CpuPolicyMode,
        requested_cpu_ids: Vec<u32>,
        min_cpus: usize,
        max_cpus: Option<usize>,
        prefer_physical_cores: bool,
        numa_node: Option<u32>,
        share_mode: CpuShareMode,
        strict: bool,
    ) -> Self {
        Self {
            mode,
            requested_cpu_ids,
            min_cpus,
            max_cpus,
            prefer_physical_cores,
            numa_node,
            share_mode,
            strict,
        }
    }

    /// 返回选择模式。
    #[must_use]
    pub const fn mode(&self) -> CpuPolicyMode {
        self.mode
    }

    /// 返回自定义模式请求的 CPU ID。
    #[must_use]
    pub fn requested_cpu_ids(&self) -> &[u32] {
        &self.requested_cpu_ids
    }

    /// 返回最少需要的逻辑 CPU 数量。
    #[must_use]
    pub const fn min_cpus(&self) -> usize {
        self.min_cpus
    }

    /// 返回最多允许的逻辑 CPU 数量。
    #[must_use]
    pub const fn max_cpus(&self) -> Option<usize> {
        self.max_cpus
    }

    /// 判断是否优先为不同物理核心选择一个逻辑 CPU。
    #[must_use]
    pub const fn prefer_physical_cores(&self) -> bool {
        self.prefer_physical_cores
    }

    /// 返回 NUMA 节点约束。
    #[must_use]
    pub const fn numa_node(&self) -> Option<u32> {
        self.numa_node
    }

    /// 返回 CPU 是否要求独占预留。
    #[must_use]
    pub const fn share_mode(&self) -> CpuShareMode {
        self.share_mode
    }

    /// 返回无法严格满足时是否应直接失败。
    #[must_use]
    pub const fn strict(&self) -> bool {
        self.strict
    }

    /// 校验字段组合，不访问宿主机 CPU 状态。
    pub fn validate(&self) -> Result<(), CpuPolicyError> {
        if !(1..=1_000_000).contains(&self.min_cpus) {
            return Err(CpuPolicyError::InvalidMinimumCpuCount);
        }
        if self
            .max_cpus
            .is_some_and(|max_cpus| !(self.min_cpus..=1_000_000).contains(&max_cpus))
        {
            return Err(CpuPolicyError::InvalidMaximumCpuCount);
        }

        let unique_cpu_ids = self
            .requested_cpu_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if unique_cpu_ids.len() != self.requested_cpu_ids.len() {
            return Err(CpuPolicyError::DuplicateCpuIds);
        }
        if self.mode == CpuPolicyMode::Custom && self.requested_cpu_ids.is_empty() {
            return Err(CpuPolicyError::CustomCpuIdsRequired);
        }
        if self.mode != CpuPolicyMode::Custom && !self.requested_cpu_ids.is_empty() {
            return Err(CpuPolicyError::RequestedCpuIdsOnlyForCustom);
        }

        Ok(())
    }
}

impl Default for CpuPolicy {
    fn default() -> Self {
        Self {
            mode: CpuPolicyMode::Auto,
            requested_cpu_ids: Vec::new(),
            min_cpus: 1,
            max_cpus: None,
            prefer_physical_cores: true,
            numa_node: None,
            share_mode: CpuShareMode::Shared,
            strict: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CpuPolicy;
    use crate::CpuPolicyError;
    use crate::CpuPolicyMode;

    #[test]
    fn validates_a_default_auto_policy() {
        assert!(CpuPolicy::default().validate().is_ok());
    }

    #[test]
    fn rejects_duplicate_custom_cpu_ids() {
        let policy = CpuPolicy::new(
            CpuPolicyMode::Custom,
            vec![2, 2],
            1,
            Some(2),
            true,
            None,
            Default::default(),
            true,
        );

        assert_eq!(policy.validate(), Err(CpuPolicyError::DuplicateCpuIds));
    }

    #[test]
    fn rejects_a_maximum_below_the_minimum() {
        let policy = CpuPolicy::new(
            CpuPolicyMode::Auto,
            Vec::new(),
            4,
            Some(2),
            true,
            None,
            Default::default(),
            true,
        );

        assert_eq!(
            policy.validate(),
            Err(CpuPolicyError::InvalidMaximumCpuCount)
        );
    }
}
