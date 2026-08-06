use serde::Deserialize;
use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::CpuPolicy;
use crate::InstanceKind;
use crate::InstanceUpdateError;
use crate::LaunchConfig;
use crate::PatchField;

/// 实例配置的部分更新请求。
///
/// 每个字段使用 [`PatchField`] 表达保留、设置或清空。名称、类型、目录和启动配置
/// 是必填配置，不能被清空；更新命令和到期时间可以显式清除。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceUpdate {
    #[serde(default, skip_serializing_if = "PatchField::is_unchanged")]
    name: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_unchanged")]
    kind: PatchField<InstanceKind>,
    #[serde(default, skip_serializing_if = "PatchField::is_unchanged")]
    directory: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_unchanged")]
    launch: PatchField<LaunchConfig>,
    #[serde(default, skip_serializing_if = "PatchField::is_unchanged")]
    cpu_policy: PatchField<CpuPolicy>,
    #[serde(default, skip_serializing_if = "PatchField::is_unchanged")]
    update_command: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_unchanged")]
    expires_at: PatchField<String>,
}

impl InstanceUpdate {
    /// 返回名称更新操作。
    #[must_use]
    pub const fn name(&self) -> &PatchField<String> {
        &self.name
    }

    /// 返回服务端类型更新操作。
    #[must_use]
    pub const fn kind(&self) -> &PatchField<InstanceKind> {
        &self.kind
    }

    /// 返回实例目录更新操作。
    #[must_use]
    pub const fn directory(&self) -> &PatchField<String> {
        &self.directory
    }

    /// 返回启动配置更新操作。
    #[must_use]
    pub const fn launch(&self) -> &PatchField<LaunchConfig> {
        &self.launch
    }

    /// 返回 CPU policy 更新操作。
    #[must_use]
    pub const fn cpu_policy(&self) -> &PatchField<CpuPolicy> {
        &self.cpu_policy
    }

    /// 返回更新命令更新操作。
    #[must_use]
    pub const fn update_command(&self) -> &PatchField<String> {
        &self.update_command
    }

    /// 返回到期时间更新操作。
    #[must_use]
    pub const fn expires_at(&self) -> &PatchField<String> {
        &self.expires_at
    }

    /// 校验更新不是空操作，并检查各个被设置字段的领域约束。
    pub fn validate(&self) -> Result<(), InstanceUpdateError> {
        if self.name.is_unchanged()
            && self.kind.is_unchanged()
            && self.directory.is_unchanged()
            && self.launch.is_unchanged()
            && self.cpu_policy.is_unchanged()
            && self.update_command.is_unchanged()
            && self.expires_at.is_unchanged()
        {
            return Err(InstanceUpdateError::Empty);
        }
        if matches!(&self.name, PatchField::Clear)
            || matches!(&self.kind, PatchField::Clear)
            || matches!(&self.directory, PatchField::Clear)
            || matches!(&self.launch, PatchField::Clear)
            || matches!(&self.cpu_policy, PatchField::Clear)
        {
            return Err(InstanceUpdateError::RequiredFieldCleared);
        }
        if matches!(&self.name, PatchField::Set(name) if !is_valid_name(name)) {
            return Err(InstanceUpdateError::InvalidName);
        }
        if matches!(&self.directory, PatchField::Set(directory) if !is_valid_directory(directory)) {
            return Err(InstanceUpdateError::InvalidDirectory);
        }
        if matches!(&self.launch, PatchField::Set(launch) if !is_valid_launch(launch)) {
            return Err(InstanceUpdateError::InvalidLaunch);
        }
        if matches!(&self.cpu_policy, PatchField::Set(cpu_policy) if cpu_policy.validate().is_err())
        {
            return Err(InstanceUpdateError::InvalidCpuPolicy);
        }
        if matches!(&self.update_command, PatchField::Set(command) if !is_valid_command(command)) {
            return Err(InstanceUpdateError::InvalidUpdateCommand);
        }
        if matches!(&self.expires_at, PatchField::Set(expires_at) if OffsetDateTime::parse(expires_at, &Rfc3339).is_err())
        {
            return Err(InstanceUpdateError::InvalidExpiration);
        }

        Ok(())
    }
}

pub(crate) fn is_valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.chars().count() <= 128 && !name.contains('\0')
}

pub(crate) fn is_valid_directory(directory: &str) -> bool {
    !directory.is_empty()
        && directory.len() <= 1024
        && !directory.contains('\0')
        && !directory.contains('\\')
        && directory.split('/').all(|component| {
            !component.is_empty()
                && component != "."
                && component != ".."
                && !component.contains(':')
        })
}

pub(crate) fn is_valid_launch(launch: &LaunchConfig) -> bool {
    !launch.executable().trim().is_empty()
        && launch.executable().len() <= 4096
        && !launch.executable().contains('\0')
        && launch.args().len() <= 256
        && launch
            .args()
            .iter()
            .all(|argument| argument.len() <= 8192 && !argument.contains('\0'))
        && launch.environment().len() <= 128
        && launch.environment().iter().all(|(name, value)| {
            !name.trim().is_empty()
                && !name.contains('\0')
                && !name.contains('=')
                && !name.to_ascii_uppercase().starts_with("MCNP_")
                && !value.contains('\0')
        })
        && !launch.stop_command().trim().is_empty()
        && launch.stop_command().len() <= 8192
        && !launch.stop_command().contains('\0')
        && (1..=300).contains(&launch.stop_timeout_seconds())
        && launch.is_valid_execution()
}

fn is_valid_command(command: &str) -> bool {
    !command.trim().is_empty() && command.len() <= 8192 && !command.contains('\0')
}
