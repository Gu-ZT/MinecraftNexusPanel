use serde::Deserialize;
use serde::Serialize;

/// 描述一次生命周期审计记录的处理结果。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstanceAuditOutcome {
    /// 请求已被 Core 接受，但最终结果尚未由异步进程任务确认。
    Accepted,
    /// 动作已经成功完成。
    Succeeded,
    /// 动作失败，实例未达到请求的目标状态。
    Failed,
    /// 动作完成但采用了明确记录的降级路径。
    Degraded,
}
