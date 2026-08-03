use std::collections::BTreeSet;

use nexus_domain::CoreId;
use nexus_domain::RequestId;

use crate::InstanceProcessManager;
use crate::InstanceRepository;
use crate::RuntimeDiscovery;

pub(crate) struct CoreRequestState {
    core_id: CoreId,
    event_subscription: Option<RequestId>,
    event_topics: BTreeSet<String>,
    instances: InstanceRepository,
    processes: InstanceProcessManager,
    runtimes: RuntimeDiscovery,
}

impl CoreRequestState {
    pub(crate) fn new(
        core_id: CoreId,
        instances: InstanceRepository,
        processes: InstanceProcessManager,
        runtimes: RuntimeDiscovery,
    ) -> Self {
        Self {
            core_id,
            event_subscription: None,
            event_topics: BTreeSet::new(),
            instances,
            processes,
            runtimes,
        }
    }

    pub(crate) const fn core_id(&self) -> CoreId {
        self.core_id
    }

    pub(crate) const fn event_subscription(&self) -> Option<RequestId> {
        self.event_subscription
    }

    pub(crate) const fn is_subscribed_to_events(&self) -> bool {
        self.event_subscription.is_some()
    }

    pub(crate) fn is_subscribed_to_topic(&self, topic: &str) -> bool {
        self.event_topics.contains(topic)
    }

    pub(crate) const fn instances(&self) -> &InstanceRepository {
        &self.instances
    }

    pub(crate) const fn processes(&self) -> &InstanceProcessManager {
        &self.processes
    }

    pub(crate) const fn runtimes(&self) -> &RuntimeDiscovery {
        &self.runtimes
    }

    pub(crate) fn subscribe_to_events(
        &mut self,
        subscription_id: RequestId,
        topics: BTreeSet<String>,
    ) {
        self.event_subscription = Some(subscription_id);
        self.event_topics = topics;
    }

    pub(crate) fn unsubscribe_from_events(&mut self) {
        self.event_subscription = None;
        self.event_topics.clear();
    }
}
