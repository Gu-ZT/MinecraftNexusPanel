use nexus_domain::CpuPolicy;
use nexus_domain::InstanceId;
use serde::Deserialize;

/// Panel CPU 独占预留请求体。
///
/// 实例修订号用于避免把预留登记到已经变化的实例配置上；policy 的
/// `shareMode` 和宿主机容量由 Core 作为最终权威校验。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CpuReservationRequest {
    instance_id: InstanceId,
    revision: u64,
    policy: CpuPolicy,
}

impl CpuReservationRequest {
    /// 校验请求体自身的领域字段。
    pub(crate) fn validate(&self) -> Result<(), ()> {
        if self.revision == 0 {
            return Err(());
        }

        self.policy.validate().map_err(|_| ())
    }

    /// 返回要登记预留的实例。
    #[must_use]
    pub(crate) const fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    /// 返回请求方观察到的实例配置修订号。
    #[must_use]
    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    /// 返回 CPU 选择和独占策略。
    #[must_use]
    pub(crate) const fn policy(&self) -> &CpuPolicy {
        &self.policy
    }
}
