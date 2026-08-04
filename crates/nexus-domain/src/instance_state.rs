//! 实例进程生命周期状态。

use serde::Deserialize;
use serde::Serialize;

/// Core 观察到的实例生命周期状态。
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstanceState {
    #[default]
    /// 已创建配置但尚未运行。
    Created,
    /// Core 正在创建进程。
    Starting,
    /// 进程已启动并可接受命令。
    Running,
    /// Core 已发出优雅停止请求，等待进程退出。
    Stopping,
    /// 进程已退出且可再次启动。
    Stopped,
    /// 启动失败或进程异常退出。
    Failed,
    /// 状态来源不可确定，禁止假设可以执行启停操作。
    Unknown,
}
