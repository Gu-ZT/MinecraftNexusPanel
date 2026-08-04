use nexus_domain::ProvisionPlan;
use serde::Deserialize;

/// 执行已解析的一键搭建计划请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProvisionExecuteRequest {
    resolved_plan: ProvisionPlan,
    plan_hash: String,
}

impl ProvisionExecuteRequest {
    /// 返回请求中的解析计划。
    #[must_use]
    pub(crate) fn resolved_plan(&self) -> &ProvisionPlan {
        &self.resolved_plan
    }

    /// 返回解析阶段生成的稳定计划哈希。
    #[must_use]
    pub(crate) fn plan_hash(&self) -> &str {
        &self.plan_hash
    }
}
