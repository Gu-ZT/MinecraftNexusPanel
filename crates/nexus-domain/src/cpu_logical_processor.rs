use serde::Deserialize;
use serde::Serialize;

use crate::CpuPerformanceClass;

/// 一个可供 Core 亲和策略引用的逻辑 CPU 描述。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuLogicalProcessor {
    id: u32,
    physical_core_id: Option<String>,
    performance_class: CpuPerformanceClass,
    online: bool,
    isolated: Option<bool>,
    numa_node: Option<u32>,
}

impl CpuLogicalProcessor {
    /// 创建逻辑 CPU 描述。
    #[must_use]
    pub fn new(
        id: u32,
        physical_core_id: Option<String>,
        performance_class: CpuPerformanceClass,
        online: bool,
        isolated: Option<bool>,
        numa_node: Option<u32>,
    ) -> Self {
        Self {
            id,
            physical_core_id,
            performance_class,
            online,
            isolated,
            numa_node,
        }
    }

    /// 返回操作系统逻辑 CPU 标识。
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// 返回物理核心标识；平台未提供映射时为 `None`。
    #[must_use]
    pub fn physical_core_id(&self) -> Option<&str> {
        self.physical_core_id.as_deref()
    }

    /// 返回操作系统报告的性能类别。
    #[must_use]
    pub const fn performance_class(&self) -> CpuPerformanceClass {
        self.performance_class
    }

    /// 判断逻辑 CPU 当前是否在线。
    #[must_use]
    pub const fn online(&self) -> bool {
        self.online
    }

    /// 返回逻辑 CPU 的隔离状态。
    ///
    /// `Some(true)` 表示操作系统明确将其列入隔离集合，`Some(false)` 表示
    /// 操作系统明确报告其不在隔离集合，`None` 表示当前平台没有提供可验证
    /// 的隔离信息。调用方不能把 `None` 当成未隔离。
    #[must_use]
    pub const fn isolated(&self) -> Option<bool> {
        self.isolated
    }

    /// 返回 NUMA 节点标识；平台未提供映射时为 `None`。
    #[must_use]
    pub const fn numa_node(&self) -> Option<u32> {
        self.numa_node
    }
}

#[cfg(test)]
mod tests {
    use super::CpuLogicalProcessor;
    use crate::CpuPerformanceClass;

    #[test]
    fn keeps_unreported_isolation_distinct_from_not_isolated() {
        let unknown =
            CpuLogicalProcessor::new(0, None, CpuPerformanceClass::Unknown, true, None, None);
        let confirmed_clear = CpuLogicalProcessor::new(
            1,
            None,
            CpuPerformanceClass::Unknown,
            true,
            Some(false),
            None,
        );

        assert_eq!(unknown.isolated(), None);
        assert_eq!(confirmed_clear.isolated(), Some(false));
    }
}
