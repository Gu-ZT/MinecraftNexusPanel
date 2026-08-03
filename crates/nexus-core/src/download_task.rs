use nexus_domain::TaskId;
use tokio::sync::watch;

#[derive(Clone)]
pub struct DownloadTask {
    id: TaskId,
    cancellation: watch::Sender<bool>,
}

impl DownloadTask {
    #[must_use]
    pub fn new() -> Self {
        let (cancellation, _) = watch::channel(false);

        Self {
            id: TaskId::new(),
            cancellation,
        }
    }

    #[must_use]
    pub const fn id(&self) -> TaskId {
        self.id
    }

    pub fn cancel(&self) {
        self.cancellation.send_replace(true);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        *self.cancellation.borrow()
    }

    pub(crate) fn subscribe_cancellation(&self) -> watch::Receiver<bool> {
        self.cancellation.subscribe()
    }
}

impl Default for DownloadTask {
    fn default() -> Self {
        Self::new()
    }
}
