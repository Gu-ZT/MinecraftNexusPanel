use serde::Deserialize;
use serde::Serialize;

use crate::CpuPolicy;
use crate::InstanceCreate;
use crate::InstanceId;
use crate::InstanceKind;
use crate::InstanceRuntime;
use crate::InstanceUpdate;
use crate::InstanceUpdateError;
use crate::LaunchConfig;
use crate::PatchField;

/// Core 持久化的 Minecraft 实例配置与运行时状态。
///
/// 配置字段由创建和更新校验保护，`revision` 用于让调用方识别配置是否发生变化；
/// `runtime` 是 Core 对进程的最新观察结果，不应被当作用户配置写回。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    id: InstanceId,
    name: String,
    kind: InstanceKind,
    directory: String,
    launch: LaunchConfig,
    /// 实例请求的 CPU 选择策略；缺少该字段的旧存档使用默认 AUTO/SHARED 策略。
    #[serde(default)]
    cpu_policy: CpuPolicy,
    #[serde(default)]
    update_command: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
    runtime: InstanceRuntime,
    revision: u64,
}

impl Instance {
    pub(crate) fn from_create(instance: InstanceCreate) -> Self {
        let (id, name, kind, directory, launch, cpu_policy) = instance.into_parts();

        Self {
            id,
            name,
            kind,
            directory,
            launch,
            cpu_policy,
            update_command: None,
            expires_at: None,
            runtime: InstanceRuntime::created(),
            revision: 1,
        }
    }

    /// 返回实例标识。
    #[must_use]
    pub fn id(&self) -> &InstanceId {
        &self.id
    }

    /// 返回实例工作目录的规范化相对路径。
    #[must_use]
    pub fn directory(&self) -> &str {
        &self.directory
    }

    /// 返回实例到期时间；未设置时为 `None`。
    #[must_use]
    pub fn expires_at(&self) -> Option<&str> {
        self.expires_at.as_deref()
    }

    /// 返回进程启动配置。
    #[must_use]
    pub fn launch(&self) -> &LaunchConfig {
        &self.launch
    }

    /// 返回持久化的实例 CPU policy 请求。
    #[must_use]
    pub const fn cpu_policy(&self) -> &CpuPolicy {
        &self.cpu_policy
    }

    /// 返回实例显示名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回实例的权威服务端类型。
    #[must_use]
    pub const fn kind(&self) -> InstanceKind {
        self.kind
    }

    /// 返回 Core 观察到的进程运行时快照。
    #[must_use]
    pub fn runtime(&self) -> &InstanceRuntime {
        &self.runtime
    }

    /// 返回配置修订号。
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// 返回更新命令；未配置时为 `None`。
    #[must_use]
    pub fn update_command(&self) -> Option<&str> {
        self.update_command.as_deref()
    }

    /// 应用一个经过领域校验的部分更新，并在配置实际变化时递增修订号。
    ///
    /// 运行时快照不会被部分更新修改；调用方应使用 [`Self::set_runtime`] 更新它。
    pub fn apply_update(&mut self, update: &InstanceUpdate) -> Result<(), InstanceUpdateError> {
        update.validate()?;
        let mut changed = false;

        if let PatchField::Set(name) = update.name()
            && &self.name != name
        {
            self.name.clone_from(name);
            changed = true;
        }
        if let PatchField::Set(kind) = update.kind()
            && self.kind != *kind
        {
            self.kind = *kind;
            changed = true;
        }
        if let PatchField::Set(directory) = update.directory()
            && &self.directory != directory
        {
            self.directory.clone_from(directory);
            changed = true;
        }
        if let PatchField::Set(launch) = update.launch()
            && &self.launch != launch
        {
            self.launch.clone_from(launch);
            changed = true;
        }
        if let PatchField::Set(cpu_policy) = update.cpu_policy()
            && &self.cpu_policy != cpu_policy
        {
            self.cpu_policy.clone_from(cpu_policy);
            changed = true;
        }
        changed |= apply_optional_patch(&mut self.update_command, update.update_command());
        changed |= apply_optional_patch(&mut self.expires_at, update.expires_at());

        if changed {
            self.revision = self.revision.saturating_add(1);
        }

        Ok(())
    }

    /// 替换 Core 保存的运行时快照，不改变配置修订号。
    pub fn set_runtime(&mut self, runtime: InstanceRuntime) {
        self.runtime = runtime;
    }
}

fn apply_optional_patch(value: &mut Option<String>, patch: &PatchField<String>) -> bool {
    match patch {
        PatchField::Unchanged => false,
        PatchField::Set(next) if value.as_ref() == Some(next) => false,
        PatchField::Set(next) => {
            value.replace(next.clone());
            true
        }
        PatchField::Clear if value.is_none() => false,
        PatchField::Clear => {
            value.take();
            true
        }
    }
}
