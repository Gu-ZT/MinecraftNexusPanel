use serde::Deserialize;
use serde::Serialize;

/// 实例在某一采样时刻的资源使用快照。
///
/// 时间戳由 Core 生成并以字符串传输；CPU 是百分比数值，内存字段使用
/// 字节，运行时间使用秒。该值对象只描述采样结果，不承诺采样频率。
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetricSample {
    occurred_at: String,
    cpu_percent: f32,
    memory_bytes: u64,
    virtual_memory_bytes: u64,
    uptime_seconds: u64,
}

impl InstanceMetricSample {
    /// 创建资源指标采样。
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        occurred_at: String,
        cpu_percent: f32,
        memory_bytes: u64,
        virtual_memory_bytes: u64,
        uptime_seconds: u64,
    ) -> Self {
        Self {
            occurred_at,
            cpu_percent,
            memory_bytes,
            virtual_memory_bytes,
            uptime_seconds,
        }
    }

    /// 返回采样发生时间。
    #[must_use]
    pub fn occurred_at(&self) -> &str {
        &self.occurred_at
    }

    /// 返回 CPU 使用率百分比。
    #[must_use]
    pub const fn cpu_percent(&self) -> f32 {
        self.cpu_percent
    }

    /// 返回当前进程使用的物理内存，单位为字节。
    #[must_use]
    pub const fn memory_bytes(&self) -> u64 {
        self.memory_bytes
    }

    /// 返回实例可见的虚拟内存，单位为字节。
    #[must_use]
    pub const fn virtual_memory_bytes(&self) -> u64 {
        self.virtual_memory_bytes
    }

    /// 返回实例已运行时间，单位为秒。
    #[must_use]
    pub const fn uptime_seconds(&self) -> u64 {
        self.uptime_seconds
    }
}
