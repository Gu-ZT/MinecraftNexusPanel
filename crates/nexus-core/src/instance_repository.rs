use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use nexus_domain::Instance;
use nexus_domain::InstanceCreate;
use nexus_domain::InstanceId;

use crate::InstanceRepositoryError;

#[derive(Clone, Default)]
pub struct InstanceRepository {
    instances: Arc<Mutex<BTreeMap<InstanceId, Instance>>>,
}

impl InstanceRepository {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&self, definition: InstanceCreate) -> Result<Instance, InstanceRepositoryError> {
        let instance = definition.into_instance()?;
        let instance_id = instance.id().clone();
        let mut instances = self.lock_instances()?;

        if instances.contains_key(&instance_id) {
            return Err(InstanceRepositoryError::AlreadyExists { instance_id });
        }

        instances.insert(instance_id, instance.clone());

        Ok(instance)
    }

    pub fn get(
        &self,
        instance_id: &InstanceId,
    ) -> Result<Option<Instance>, InstanceRepositoryError> {
        let instances = self.lock_instances()?;

        Ok(instances.get(instance_id).cloned())
    }

    pub fn list(&self) -> Result<Vec<Instance>, InstanceRepositoryError> {
        let instances = self.lock_instances()?;

        Ok(instances.values().cloned().collect())
    }

    fn lock_instances(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, BTreeMap<InstanceId, Instance>>, InstanceRepositoryError>
    {
        self.instances
            .lock()
            .map_err(|_| InstanceRepositoryError::LockPoisoned)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::InstanceRepository;
    use crate::InstanceRepositoryError;
    use nexus_domain::InstanceCreate;
    use nexus_domain::InstanceId;
    use nexus_domain::InstanceKind;
    use nexus_domain::LaunchConfig;

    #[test]
    fn creates_instances_once_and_lists_them_by_identifier() {
        let repository = InstanceRepository::new();
        let survival = instance_create("survival");
        let creative = instance_create("creative");

        repository
            .create(survival.clone())
            .expect("first instance is created");
        repository
            .create(creative)
            .expect("second instance is created");

        assert!(matches!(
            repository.create(survival),
            Err(InstanceRepositoryError::AlreadyExists { .. })
        ));

        let identifiers = repository
            .list()
            .expect("instances are listed")
            .into_iter()
            .map(|instance| instance.id().to_string())
            .collect::<Vec<_>>();

        assert_eq!(identifiers, ["creative", "survival"]);
    }

    fn instance_create(identifier: &str) -> InstanceCreate {
        InstanceCreate::new(
            InstanceId::new(identifier.to_owned()).expect("test identifier is valid"),
            identifier.to_owned(),
            InstanceKind::Paper,
            format!("instances/{identifier}"),
            LaunchConfig::new(
                "java".to_owned(),
                Vec::new(),
                BTreeMap::new(),
                "stop".to_owned(),
                30,
            ),
        )
        .expect("test instance is valid")
    }
}
