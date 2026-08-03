use nexus_domain::InstanceId;
use nexus_domain::InstanceKind;
use nexus_domain::ProxySubserverError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxySubserverRepositoryError {
    #[error("proxy instance {instance_id} with kind {kind:?} does not support subservers")]
    UnsupportedProxy {
        instance_id: InstanceId,
        kind: InstanceKind,
    },
    #[error("proxy subserver {subserver_id} does not exist on {proxy_instance_id}")]
    NotFound {
        proxy_instance_id: InstanceId,
        subserver_id: String,
    },
    #[error("proxy instance {instance_id} cannot have another subserver")]
    TopologyLimit { instance_id: InstanceId },
    #[error(transparent)]
    Invalid(#[from] ProxySubserverError),
    #[error("proxy subserver repository lock is poisoned")]
    LockPoisoned,
}
