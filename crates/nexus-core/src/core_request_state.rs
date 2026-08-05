use std::collections::BTreeSet;

use nexus_domain::CoreId;
use nexus_domain::CpuTopology;
use nexus_domain::RequestId;

use crate::FileManager;
use crate::InstanceProcessManager;
use crate::InstanceRepository;
use crate::ProvisionManager;
use crate::ProxySubserverRepository;
use crate::RuntimeManager;

/// 单个安全协议会话持有的 Core 资源视图和事件订阅状态。
///
/// 管理器本身通过内部共享句柄跨请求复用；该值只保存当前连接的订阅游标和主题过滤器。
pub(crate) struct CoreRequestState {
    core_id: CoreId,
    cpu_topology: CpuTopology,
    event_subscription: Option<RequestId>,
    event_topics: BTreeSet<String>,
    instances: InstanceRepository,
    processes: InstanceProcessManager,
    proxy_subservers: ProxySubserverRepository,
    provision: ProvisionManager,
    runtimes: RuntimeManager,
    files: FileManager,
}

impl CoreRequestState {
    /// 创建未订阅事件的请求状态。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        core_id: CoreId,
        cpu_topology: CpuTopology,
        instances: InstanceRepository,
        processes: InstanceProcessManager,
        proxy_subservers: ProxySubserverRepository,
        provision: ProvisionManager,
        runtimes: RuntimeManager,
        files: FileManager,
    ) -> Self {
        Self {
            core_id,
            cpu_topology,
            event_subscription: None,
            event_topics: BTreeSet::new(),
            instances,
            processes,
            proxy_subservers,
            provision,
            runtimes,
            files,
        }
    }

    /// 返回当前 Core 标识。
    pub(crate) const fn core_id(&self) -> CoreId {
        self.core_id
    }

    /// 返回 Core 启动时缓存的 CPU 拓扑快照。
    pub(crate) const fn cpu_topology(&self) -> &CpuTopology {
        &self.cpu_topology
    }

    /// 返回事件订阅请求标识。
    pub(crate) const fn event_subscription(&self) -> Option<RequestId> {
        self.event_subscription
    }

    /// 判断当前连接是否订阅了事件。
    pub(crate) const fn is_subscribed_to_events(&self) -> bool {
        self.event_subscription.is_some()
    }

    /// 判断当前连接是否订阅指定主题。
    pub(crate) fn is_subscribed_to_topic(&self, topic: &str) -> bool {
        self.event_topics.contains(topic)
    }

    /// 返回实例仓库。
    pub(crate) const fn instances(&self) -> &InstanceRepository {
        &self.instances
    }

    /// 返回进程管理器。
    pub(crate) const fn processes(&self) -> &InstanceProcessManager {
        &self.processes
    }

    /// 返回代理后端仓库。
    pub(crate) const fn proxy_subservers(&self) -> &ProxySubserverRepository {
        &self.proxy_subservers
    }

    /// 返回一键搭建管理器。
    pub(crate) const fn provision(&self) -> &ProvisionManager {
        &self.provision
    }

    /// 返回运行时管理器。
    pub(crate) const fn runtimes(&self) -> &RuntimeManager {
        &self.runtimes
    }

    /// 返回文件管理器。
    pub(crate) const fn files(&self) -> &FileManager {
        &self.files
    }

    /// 替换当前事件订阅请求和主题集合。
    pub(crate) fn subscribe_to_events(
        &mut self,
        subscription_id: RequestId,
        topics: BTreeSet<String>,
    ) {
        self.event_subscription = Some(subscription_id);
        self.event_topics = topics;
    }

    /// 清除当前连接的事件订阅。
    pub(crate) fn unsubscribe_from_events(&mut self) {
        self.event_subscription = None;
        self.event_topics.clear();
    }
}
