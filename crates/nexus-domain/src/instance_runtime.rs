//! 实例进程运行时快照。

use serde::Deserialize;
use serde::Serialize;

use crate::InstanceState;

/// 记录实例状态、PID、启动时间和退出码的运行时快照。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceRuntime {
    state: InstanceState,
    pid: Option<u32>,
    started_at: Option<String>,
    exit_code: Option<i32>,
}

impl InstanceRuntime {
    /// 创建一个尚未启动的运行时快照。
    #[must_use]
    pub const fn created() -> Self {
        Self {
            state: InstanceState::Created,
            pid: None,
            started_at: None,
            exit_code: None,
        }
    }

    /// 返回当前生命周期状态。
    #[must_use]
    pub const fn state(&self) -> InstanceState {
        self.state
    }

    /// 创建启动中的快照；此时通常还没有 PID。
    #[must_use]
    pub const fn starting() -> Self {
        Self {
            state: InstanceState::Starting,
            pid: None,
            started_at: None,
            exit_code: None,
        }
    }

    /// 创建已经获得操作系统 PID 的运行中快照。
    #[must_use]
    pub fn running(pid: u32, started_at: String) -> Self {
        Self {
            state: InstanceState::Running,
            pid: Some(pid),
            started_at: Some(started_at),
            exit_code: None,
        }
    }

    /// 将当前运行时转换为停止中的快照并保留 PID 信息。
    #[must_use]
    pub fn stopping(&self) -> Self {
        Self {
            state: InstanceState::Stopping,
            pid: self.pid,
            started_at: self.started_at.clone(),
            exit_code: None,
        }
    }

    /// 创建已停止快照并记录可选退出码。
    #[must_use]
    pub fn stopped(&self, exit_code: Option<i32>) -> Self {
        Self {
            state: InstanceState::Stopped,
            pid: None,
            started_at: self.started_at.clone(),
            exit_code,
        }
    }

    /// 创建失败快照并记录可选退出码。
    #[must_use]
    pub fn failed(&self, exit_code: Option<i32>) -> Self {
        Self {
            state: InstanceState::Failed,
            pid: None,
            started_at: self.started_at.clone(),
            exit_code,
        }
    }

    /// 返回操作系统 PID。
    #[must_use]
    pub const fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// 返回进程启动时间。
    #[must_use]
    pub fn started_at(&self) -> Option<&str> {
        self.started_at.as_deref()
    }

    /// 返回进程退出码。
    #[must_use]
    pub const fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }
}
