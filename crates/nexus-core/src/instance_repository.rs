use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use nexus_domain::Instance;
use nexus_domain::InstanceCreate;
use nexus_domain::InstanceId;
use nexus_domain::InstanceRuntime;
use nexus_domain::InstanceState;
use nexus_domain::InstanceUpdate;

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

    pub fn set_runtime(
        &self,
        instance_id: &InstanceId,
        runtime: InstanceRuntime,
    ) -> Result<Instance, InstanceRepositoryError> {
        let mut instances = self.lock_instances()?;
        let instance =
            instances
                .get_mut(instance_id)
                .ok_or_else(|| InstanceRepositoryError::NotFound {
                    instance_id: instance_id.clone(),
                })?;
        instance.set_runtime(runtime);

        Ok(instance.clone())
    }

    pub fn update(
        &self,
        instance_id: &InstanceId,
        expected_revision: u64,
        update: &InstanceUpdate,
    ) -> Result<Instance, InstanceRepositoryError> {
        let mut instances = self.lock_instances()?;
        let instance =
            instances
                .get_mut(instance_id)
                .ok_or_else(|| InstanceRepositoryError::NotFound {
                    instance_id: instance_id.clone(),
                })?;
        if instance.revision() != expected_revision {
            return Err(InstanceRepositoryError::RevisionMismatch {
                expected_revision,
                actual_revision: instance.revision(),
            });
        }
        let state = instance.runtime().state();
        if !matches!(
            state,
            InstanceState::Created | InstanceState::Stopped | InstanceState::Failed
        ) {
            return Err(InstanceRepositoryError::StateConflict {
                instance_id: instance_id.clone(),
                state,
            });
        }
        instance.apply_update(update)?;

        Ok(instance.clone())
    }

    pub fn transition_runtime(
        &self,
        instance_id: &InstanceId,
        allowed_states: &[InstanceState],
        runtime: InstanceRuntime,
    ) -> Result<Instance, InstanceRepositoryError> {
        let mut instances = self.lock_instances()?;
        let instance =
            instances
                .get_mut(instance_id)
                .ok_or_else(|| InstanceRepositoryError::NotFound {
                    instance_id: instance_id.clone(),
                })?;
        let state = instance.runtime().state();
        if !allowed_states.contains(&state) {
            return Err(InstanceRepositoryError::StateConflict {
                instance_id: instance_id.clone(),
                state,
            });
        }
        instance.set_runtime(runtime);

        Ok(instance.clone())
    }

    fn lock_instances(
        &self,
    ) -> Result<MutexGuard<'_, BTreeMap<InstanceId, Instance>>, InstanceRepositoryError> {
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
    use nexus_domain::InstanceRuntime;
    use nexus_domain::InstanceState;
    use nexus_domain::InstanceUpdate;
    use nexus_domain::LaunchConfig;
    use serde_json::from_value;
    use serde_json::json;

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

    #[test]
    fn updates_stopped_instances_with_revision_checks() {
        let repository = InstanceRepository::new();
        let instance = instance_create("survival");
        repository
            .create(instance)
            .expect("instance is created for settings updates");
        let instance_id = InstanceId::new("survival".to_owned()).expect("test identifier is valid");
        let update: InstanceUpdate = from_value(json!({
            "name": "Configured Survival",
            "directory": "instances/configured-survival",
            "updateCommand": "./update.sh",
            "expiresAt": "2030-01-01T00:00:00Z",
        }))
        .expect("update payload is valid");

        let updated = repository
            .update(&instance_id, 1, &update)
            .expect("stopped instance settings are updated");

        assert_eq!(updated.name(), "Configured Survival");
        assert_eq!(updated.directory(), "instances/configured-survival");
        assert_eq!(updated.update_command(), Some("./update.sh"));
        assert_eq!(updated.expires_at(), Some("2030-01-01T00:00:00Z"));
        assert_eq!(updated.revision(), 2);
        assert!(matches!(
            repository.update(&instance_id, 1, &update),
            Err(InstanceRepositoryError::RevisionMismatch {
                expected_revision: 1,
                actual_revision: 2,
            })
        ));

        repository
            .set_runtime(
                &instance_id,
                InstanceRuntime::running(42, "2030-01-01T00:00:00Z".to_owned()),
            )
            .expect("instance is marked running");
        assert!(matches!(
            repository.update(&instance_id, 2, &update),
            Err(InstanceRepositoryError::StateConflict {
                state: InstanceState::Running,
                ..
            })
        ));
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
