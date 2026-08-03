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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceCreate {
    id: InstanceId,
    name: String,
    kind: InstanceKind,
    directory: String,
    launch: LaunchConfig,
}

impl InstanceCreate {
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
        };
        instance.validate()?;

        Ok(instance)
    }

    #[must_use]
    pub fn id(&self) -> &InstanceId {
        &self.id
    }

    pub fn into_instance(self) -> Result<Instance, InstanceCreateError> {
        self.validate()?;

        Ok(Instance::from_create(self))
    }

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

        Ok(())
    }

    pub(crate) fn into_parts(self) -> (InstanceId, String, InstanceKind, String, LaunchConfig) {
        (self.id, self.name, self.kind, self.directory, self.launch)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::InstanceCreate;
    use crate::InstanceId;
    use crate::InstanceKind;
    use crate::LaunchConfig;

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
}
