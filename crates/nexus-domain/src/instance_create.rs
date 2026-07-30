use crate::Instance;
use crate::InstanceCreateError;
use crate::InstanceId;
use crate::InstanceKind;
use crate::LaunchConfig;
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
        if self.name.trim().is_empty()
            || self.name.chars().count() > 128
            || self.name.contains('\0')
        {
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

fn is_valid_directory(directory: &str) -> bool {
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

fn is_valid_launch(launch: &LaunchConfig) -> bool {
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
                && !value.contains('\0')
        })
        && !launch.stop_command().trim().is_empty()
        && launch.stop_command().len() <= 8192
        && !launch.stop_command().contains('\0')
        && (1..=300).contains(&launch.stop_timeout_seconds())
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
}
