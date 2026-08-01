use serde::Deserialize;
use serde::Serialize;

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

    #[must_use]
    pub fn occurred_at(&self) -> &str {
        &self.occurred_at
    }

    #[must_use]
    pub const fn cpu_percent(&self) -> f32 {
        self.cpu_percent
    }

    #[must_use]
    pub const fn memory_bytes(&self) -> u64 {
        self.memory_bytes
    }

    #[must_use]
    pub const fn virtual_memory_bytes(&self) -> u64 {
        self.virtual_memory_bytes
    }

    #[must_use]
    pub const fn uptime_seconds(&self) -> u64 {
        self.uptime_seconds
    }
}
