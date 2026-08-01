use nexus_domain::CoreId;
use nexus_domain::RequestId;

use crate::InstanceProcessManager;
use crate::InstanceRepository;

pub(crate) struct CoreRequestState {
    core_id: CoreId,
    event_subscription: Option<RequestId>,
    instances: InstanceRepository,
    processes: InstanceProcessManager,
}

impl CoreRequestState {
    pub(crate) const fn new(
        core_id: CoreId,
        instances: InstanceRepository,
        processes: InstanceProcessManager,
    ) -> Self {
        Self {
            core_id,
            event_subscription: None,
            instances,
            processes,
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

    pub(crate) const fn instances(&self) -> &InstanceRepository {
        &self.instances
    }

    pub(crate) const fn processes(&self) -> &InstanceProcessManager {
        &self.processes
    }

    pub(crate) fn subscribe_to_events(&mut self, subscription_id: RequestId) {
        self.event_subscription = Some(subscription_id);
    }

    pub(crate) fn unsubscribe_from_events(&mut self) {
        self.event_subscription = None;
    }
}
