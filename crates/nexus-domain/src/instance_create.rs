use crate::CpuPolicy;
use crate::Instance;
use crate::InstanceCreateError;
use crate::InstanceId;
use crate::InstanceKind;
use crate::LaunchConfig;
use crate::instance_update::is_valid_directory;
use crate::instance_update::is_valid_launch;
use crate::instance_update::is_valid_name;
use serde::Deserialize;
use serde::Serialize;

/// 创建实例所需的最小配置。
///
/// 构造和 [`Self::validate`] 只检查领域边界，不负责下载服务端、创建目录或启动进程；
/// 这些副作用由 Core 的编排层在配置通过后执行。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceCreate {
    id: InstanceId,
    name: String,
    kind: InstanceKind,
    directory: String,
    launch: LaunchConfig,
    #[serde(default)]
    cpu_policy: CpuPolicy,
}

impl InstanceCreate {
    /// 创建并校验实例配置。
    pub fn new(
        id: InstanceId,
        name: String,
        kind: InstanceKind,
        directory: String,
        launch: LaunchConfig,
    ) -> Result<Self, InstanceCreateError> {
        let instance = Self {
            id,
            name,
            kind,
            directory,
            launch,
            cpu_policy: CpuPolicy::default(),
        };
        instance.validate()?;

        Ok(instance)
    }

    /// 返回实例标识。
    #[must_use]
    pub fn id(&self) -> &InstanceId {
        &self.id
    }

    /// 校验配置并转换为初始运行时状态为 `Created` 的实例。
    pub fn into_instance(self) -> Result<Instance, InstanceCreateError> {
        self.validate()?;

        Ok(Instance::from_create(self))
    }

    /// 校验名称、目录和启动配置是否满足实例领域约束。
    pub fn validate(&self) -> Result<(), InstanceCreateError> {
        if !is_valid_name(&self.name) {
            return Err(InstanceCreateError::InvalidName);
        }
        if !is_valid_directory(&self.directory) {
            return Err(InstanceCreateError::InvalidDirectory);
        }
        if !is_valid_launch(&self.launch) {
            return Err(InstanceCreateError::InvalidLaunch);
        }
        if self.cpu_policy.validate().is_err() {
            return Err(InstanceCreateError::InvalidCpuPolicy);
        }

        Ok(())
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        InstanceId,
        String,
        InstanceKind,
        String,
        LaunchConfig,
        CpuPolicy,
    ) {
        (
            self.id,
            self.name,
            self.kind,
            self.directory,
            self.launch,
            self.cpu_policy,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::InstanceCreate;
    use crate::CpuPolicyMode;
    use crate::InstanceId;
    use crate::InstanceKind;
    use crate::LaunchConfig;
    use serde_json::from_value;
    use serde_json::json;

    #[test]
    fn rejects_a_directory_that_escapes_the_instance_root() {
        let instance = InstanceCreate::new(
            InstanceId::new("survival".to_owned()).expect("test identifier is valid"),
            "Survival".to_owned(),
            InstanceKind::Paper,
            "instances/../outside".to_owned(),
            LaunchConfig::new(
                "java".to_owned(),
                Vec::new(),
                BTreeMap::new(),
                "stop".to_owned(),
                30,
            ),
        );

        assert!(instance.is_err());
    }

    #[test]
    fn rejects_reserved_core_environment_variables() {
        let mut environment = BTreeMap::new();
        environment.insert("MCNP_CORE_PSK".to_owned(), "secret".to_owned());
        let instance = InstanceCreate::new(
            InstanceId::new("survival".to_owned()).expect("test identifier is valid"),
            "Survival".to_owned(),
            InstanceKind::Paper,
            "instances/survival".to_owned(),
            LaunchConfig::new(
                "java".to_owned(),
                Vec::new(),
                environment,
                "stop".to_owned(),
                30,
            ),
        );

        assert!(instance.is_err());
    }

    #[test]
    fn accepts_legacy_payloads_without_a_cpu_policy() {
        let instance: InstanceCreate = from_value(json!({
            "id": "survival",
            "name": "Survival",
            "kind": "PAPER",
            "directory": "instances/survival",
            "launch": {
                "executable": "java",
                "args": [],
                "environment": {},
                "stopCommand": "stop",
                "stopTimeoutSeconds": 30
            }
        }))
        .expect("legacy instance payload is accepted");

        assert_eq!(
            instance
                .into_instance()
                .expect("legacy instance is created")
                .cpu_policy()
                .mode(),
            CpuPolicyMode::Auto
        );
    }
}
