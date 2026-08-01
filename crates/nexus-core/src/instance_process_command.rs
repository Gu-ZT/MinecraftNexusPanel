use std::time::Duration;

use tokio::sync::oneshot;

pub(crate) enum InstanceProcessCommand {
    Kill {
        acknowledged: oneshot::Sender<bool>,
    },
    SendCommand {
        acknowledged: oneshot::Sender<bool>,
        command: String,
    },
    Stop {
        acknowledged: oneshot::Sender<bool>,
        timeout: Duration,
    },
}
