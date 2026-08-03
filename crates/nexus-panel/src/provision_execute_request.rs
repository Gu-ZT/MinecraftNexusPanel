use nexus_domain::ProvisionPlan;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProvisionExecuteRequest {
    resolved_plan: ProvisionPlan,
    plan_hash: String,
}

impl ProvisionExecuteRequest {
    #[must_use]
    pub(crate) fn resolved_plan(&self) -> &ProvisionPlan {
        &self.resolved_plan
    }

    #[must_use]
    pub(crate) fn plan_hash(&self) -> &str {
        &self.plan_hash
    }
}
