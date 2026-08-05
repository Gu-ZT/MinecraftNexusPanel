use serde::Deserialize;
use serde::Serialize;

/// 当前可以用于性能核或能效核策略的逻辑 CPU 集合。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuAvailability {
    performance_cpu_ids: Vec<u32>,
    efficiency_cpu_ids: Vec<u32>,
}

impl CpuAvailability {
    /// 创建 CPU 可用集合。
    #[must_use]
    pub fn new(performance_cpu_ids: Vec<u32>, efficiency_cpu_ids: Vec<u32>) -> Self {
        Self {
            performance_cpu_ids,
            efficiency_cpu_ids,
        }
    }

    /// 返回已确认的性能 CPU 标识。
    #[must_use]
    pub fn performance_cpu_ids(&self) -> &[u32] {
        &self.performance_cpu_ids
    }

    /// 返回已确认的能效 CPU 标识。
    #[must_use]
    pub fn efficiency_cpu_ids(&self) -> &[u32] {
        &self.efficiency_cpu_ids
    }
}
