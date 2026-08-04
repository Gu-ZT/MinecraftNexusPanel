use nexus_domain::InstanceId;
use nexus_domain::InstanceKind;
use nexus_domain::ProxySubserverError;
use thiserror::Error;

/// 代理后端关系的代理类型、拓扑和输入校验错误。
#[derive(Debug, Error)]
pub enum ProxySubserverRepositoryError {
    /// 目标实例不是支持后端关系的代理类型。
    #[error("proxy instance {instance_id} with kind {kind:?} does not support subservers")]
    UnsupportedProxy {
        instance_id: InstanceId,
        kind: InstanceKind,
    },
    /// 指定后端关系不存在。
    #[error("proxy subserver {subserver_id} does not exist on {proxy_instance_id}")]
    NotFound {
        proxy_instance_id: InstanceId,
        subserver_id: String,
    },
    /// 新增关系会超过当前代理拓扑上限。
    #[error("proxy instance {instance_id} cannot have another subserver")]
    TopologyLimit { instance_id: InstanceId },
    /// 后端关系字段校验失败。
    #[error(transparent)]
    Invalid(#[from] ProxySubserverError),
    /// 后端关系仓库锁不可用。
    #[error("proxy subserver repository lock is poisoned")]
    LockPoisoned,
}
