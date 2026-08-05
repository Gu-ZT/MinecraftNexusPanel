use serde::Deserialize;
use serde::Serialize;

use crate::EventId;
use crate::InstanceAuditAction;
use crate::InstanceAuditOutcome;
use crate::InstanceId;
use crate::RuntimeMode;
use crate::SupervisorMode;
use crate::TaskId;

/// 记录一次实例生命周期或受管进程退出结果。
///
/// 审计记录只保存进程监督边界内可验证的事实。`DEGRADED` 结果为未来容器
/// 执行器或资源执行器保留明确语义；当前 Core 不会把未实现的容器运行时回退
/// 成宿主机进程。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceAuditRecord {
    audit_id: EventId,
    instance_id: InstanceId,
    task_id: Option<TaskId>,
    action: InstanceAuditAction,
    outcome: InstanceAuditOutcome,
    runtime_mode: RuntimeMode,
    supervisor_mode: SupervisorMode,
    reason: Option<String>,
    occurred_at: String,
}

impl InstanceAuditRecord {
    /// 创建一条实例审计记录。
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instance_id: InstanceId,
        task_id: Option<TaskId>,
        action: InstanceAuditAction,
        outcome: InstanceAuditOutcome,
        runtime_mode: RuntimeMode,
        supervisor_mode: SupervisorMode,
        reason: Option<String>,
        occurred_at: String,
    ) -> Self {
        Self {
            audit_id: EventId::new(),
            instance_id,
            task_id,
            action,
            outcome,
            runtime_mode,
            supervisor_mode,
            reason,
            occurred_at,
        }
    }

    /// 返回审计记录标识。
    #[must_use]
    pub const fn audit_id(&self) -> EventId {
        self.audit_id
    }

    /// 返回关联的实例标识。
    #[must_use]
    pub fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    /// 返回关联的异步任务标识。
    #[must_use]
    pub const fn task_id(&self) -> Option<TaskId> {
        self.task_id
    }

    /// 返回生命周期动作。
    #[must_use]
    pub const fn action(&self) -> InstanceAuditAction {
        self.action
    }

    /// 返回动作结果。
    #[must_use]
    pub const fn outcome(&self) -> InstanceAuditOutcome {
        self.outcome
    }

    /// 返回请求使用的运行时模式。
    #[must_use]
    pub const fn runtime_mode(&self) -> RuntimeMode {
        self.runtime_mode
    }

    /// 返回请求使用的监督模式。
    #[must_use]
    pub const fn supervisor_mode(&self) -> SupervisorMode {
        self.supervisor_mode
    }

    /// 返回失败、退出或降级原因。
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// 返回记录产生时间。
    #[must_use]
    pub fn occurred_at(&self) -> &str {
        &self.occurred_at
    }
}
