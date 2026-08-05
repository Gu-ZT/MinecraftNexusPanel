use serde::Deserialize;
use serde::Serialize;

/// 标识一次实例进程审计所对应的生命周期动作。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstanceAuditAction {
    /// 请求启动实例。
    Start,
    /// 请求优雅停止实例。
    Stop,
    /// 请求强制终止实例。
    Kill,
    /// 受管进程在未请求停止时退出。
    ProcessExit,
}

#[cfg(test)]
mod tests {
    use serde_json::to_value;

    use super::InstanceAuditAction;

    #[test]
    fn serializes_actions_as_stable_protocol_values() {
        assert_eq!(
            to_value(InstanceAuditAction::ProcessExit).expect("audit action serializes"),
            "PROCESS_EXIT"
        );
    }
}
