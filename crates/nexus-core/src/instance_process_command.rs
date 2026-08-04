use std::time::Duration;

use tokio::sync::oneshot;

/// 发送给实例进程监督器的内部控制命令。
pub(crate) enum InstanceProcessCommand {
    /// 立即终止进程。
    Kill { acknowledged: oneshot::Sender<bool> },
    /// 向标准输入写入命令。
    SendCommand {
        acknowledged: oneshot::Sender<bool>,
        command: String,
    },
    /// 发送停止命令并设置超时。
    Stop {
        acknowledged: oneshot::Sender<bool>,
        timeout: Duration,
    },
}
