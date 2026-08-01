use std::time::Duration;

use tokio::sync::oneshot;

pub(crate) enum InstanceProcessCommand {
    Kill {
        acknowledged: oneshot::Sender<bool>,
    },
    Stop {
        acknowledged: oneshot::Sender<bool>,
        timeout: Duration,
    },
}
