use std::time::Duration;

use nexus_domain::TaskId;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::InstanceProcessCommand;

/// 单个实例子进程的内部控制句柄。
///
/// 所有操作都通过监督器命令通道执行，避免多个请求直接并发操作同一个子进程。
#[derive(Clone)]
pub(crate) struct InstanceProcess {
    command_sender: mpsc::Sender<InstanceProcessCommand>,
    process_id: TaskId,
}

impl InstanceProcess {
    /// 创建进程控制句柄。
    pub(crate) const fn new(
        process_id: TaskId,
        command_sender: mpsc::Sender<InstanceProcessCommand>,
    ) -> Self {
        Self {
            command_sender,
            process_id,
        }
    }

    /// 返回内部进程任务标识。
    pub(crate) const fn process_id(&self) -> TaskId {
        self.process_id
    }

    /// 请求立即终止子进程。
    pub(crate) async fn kill(&self) -> bool {
        let (acknowledged, receiver) = oneshot::channel();
        if self
            .command_sender
            .send(InstanceProcessCommand::Kill { acknowledged })
            .await
            .is_err()
        {
            return false;
        }

        receiver.await.unwrap_or(false)
    }

    /// 请求向子进程标准输入写入命令。
    pub(crate) async fn send_command(&self, command: String) -> bool {
        let (acknowledged, receiver) = oneshot::channel();
        if self
            .command_sender
            .send(InstanceProcessCommand::SendCommand {
                acknowledged,
                command,
            })
            .await
            .is_err()
        {
            return false;
        }

        receiver.await.unwrap_or(false)
    }

    /// 请求优雅停止并等待监督器确认。
    pub(crate) async fn stop(&self, timeout: Duration) -> bool {
        let (acknowledged, receiver) = oneshot::channel();
        if self
            .command_sender
            .send(InstanceProcessCommand::Stop {
                acknowledged,
                timeout,
            })
            .await
            .is_err()
        {
            return false;
        }

        receiver.await.unwrap_or(false)
    }
}
