use nexus_domain::TaskId;
use tokio::sync::watch;

/// 可取消的下载任务控制句柄。
///
/// 句柄可克隆，任一副本调用 [`Self::cancel`] 都会通知实际下载循环；取消是幂等的。
#[derive(Clone)]
pub struct DownloadTask {
    id: TaskId,
    cancellation: watch::Sender<bool>,
}

impl DownloadTask {
    /// 创建一个未取消的新任务。
    #[must_use]
    pub fn new() -> Self {
        let (cancellation, _) = watch::channel(false);

        Self {
            id: TaskId::new(),
            cancellation,
        }
    }

    /// 返回任务标识。
    #[must_use]
    pub const fn id(&self) -> TaskId {
        self.id
    }

    /// 请求取消任务。
    pub fn cancel(&self) {
        self.cancellation.send_replace(true);
    }

    /// 判断任务是否已收到取消请求。
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
