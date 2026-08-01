use std::time::Duration;

use nexus_domain::TaskId;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::InstanceProcessCommand;

#[derive(Clone)]
pub(crate) struct InstanceProcess {
    command_sender: mpsc::Sender<InstanceProcessCommand>,
    process_id: TaskId,
}

impl InstanceProcess {
    pub(crate) const fn new(
        process_id: TaskId,
        command_sender: mpsc::Sender<InstanceProcessCommand>,
    ) -> Self {
        Self {
            command_sender,
            process_id,
        }
    }

    pub(crate) const fn process_id(&self) -> TaskId {
        self.process_id
    }

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
