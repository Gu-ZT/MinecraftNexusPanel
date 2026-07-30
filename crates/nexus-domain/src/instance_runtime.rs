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
}
