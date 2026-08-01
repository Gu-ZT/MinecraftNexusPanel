use serde::Deserialize;
use serde::Serialize;

use crate::InstanceState;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceRuntime {
    state: InstanceState,
    pid: Option<u32>,
    started_at: Option<String>,
    exit_code: Option<i32>,
}

impl InstanceRuntime {
    #[must_use]
    pub const fn created() -> Self {
        Self {
            state: InstanceState::Created,
            pid: None,
            started_at: None,
            exit_code: None,
        }
    }

    #[must_use]
    pub const fn state(&self) -> InstanceState {
        self.state
    }

    #[must_use]
    pub const fn starting() -> Self {
        Self {
            state: InstanceState::Starting,
            pid: None,
            started_at: None,
            exit_code: None,
        }
    }

    #[must_use]
    pub fn running(pid: u32, started_at: String) -> Self {
        Self {
            state: InstanceState::Running,
            pid: Some(pid),
            started_at: Some(started_at),
            exit_code: None,
        }
    }

    #[must_use]
    pub fn stopping(&self) -> Self {
        Self {
            state: InstanceState::Stopping,
            pid: self.pid,
            started_at: self.started_at.clone(),
            exit_code: None,
        }
    }

    #[must_use]
    pub fn stopped(&self, exit_code: Option<i32>) -> Self {
        Self {
            state: InstanceState::Stopped,
            pid: None,
            started_at: self.started_at.clone(),
            exit_code,
        }
    }

    #[must_use]
    pub fn failed(&self, exit_code: Option<i32>) -> Self {
        Self {
            state: InstanceState::Failed,
            pid: None,
            started_at: self.started_at.clone(),
            exit_code,
        }
    }

    #[must_use]
    pub const fn pid(&self) -> Option<u32> {
        self.pid
    }

    #[must_use]
    pub fn started_at(&self) -> Option<&str> {
        self.started_at.as_deref()
    }

    #[must_use]
    pub const fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }
}
