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
    isolated: bool,
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
        isolated: bool,
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

    /// 判断逻辑 CPU 是否被系统标记为隔离。
    #[must_use]
    pub const fn isolated(&self) -> bool {
        self.isolated
    }

    /// 返回 NUMA 节点标识；平台未提供映射时为 `None`。
    #[must_use]
    pub const fn numa_node(&self) -> Option<u32> {
        self.numa_node
    }
}
