use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use nexus_domain::Instance;
use nexus_domain::InstanceId;
use nexus_domain::ProxySubserver;

use crate::ProxySubserverRepositoryError;

type SubserverMap = BTreeMap<InstanceId, BTreeMap<String, ProxySubserver>>;

#[derive(Clone, Default)]
pub struct ProxySubserverRepository {
    subservers: Arc<Mutex<SubserverMap>>,
}

impl ProxySubserverRepository {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(
        &self,
        proxy: &Instance,
    ) -> Result<Vec<ProxySubserver>, ProxySubserverRepositoryError> {
        ensure_supported_proxy(proxy)?;
        let subservers = self.lock_subservers()?;

        Ok(subservers
            .get(proxy.id())
            .map(|items| items.values().cloned().collect())
            .unwrap_or_default())
    }

    pub fn upsert(
        &self,
        proxy: &Instance,
        subserver: ProxySubserver,
    ) -> Result<ProxySubserver, ProxySubserverRepositoryError> {
        ensure_supported_proxy(proxy)?;
        subserver.validate()?;
        let mut subservers = self.lock_subservers()?;
        let items = subservers.entry(proxy.id().clone()).or_default();
        let is_new = !items.contains_key(subserver.id());
        if is_new
            && !proxy
                .kind()
                .proxy_topology()
                .allows_backend_count(items.len().saturating_add(1))
        {
            return Err(ProxySubserverRepositoryError::TopologyLimit {
                instance_id: proxy.id().clone(),
            });
        }

        let id = subserver.id().to_owned();
        items.insert(id, subserver.clone());

        Ok(subserver)
    }

    pub fn delete(
        &self,
        proxy: &Instance,
        subserver_id: &str,
    ) -> Result<(), ProxySubserverRepositoryError> {
        ensure_supported_proxy(proxy)?;
        let mut subservers = self.lock_subservers()?;
        let Some(items) = subservers.get_mut(proxy.id()) else {
            return Err(ProxySubserverRepositoryError::NotFound {
                proxy_instance_id: proxy.id().clone(),
                subserver_id: subserver_id.to_owned(),
            });
        };
        if items.remove(subserver_id).is_none() {
            return Err(ProxySubserverRepositoryError::NotFound {
                proxy_instance_id: proxy.id().clone(),
                subserver_id: subserver_id.to_owned(),
            });
        }
        if items.is_empty() {
            subservers.remove(proxy.id());
        }

        Ok(())
    }

    fn lock_subservers(
        &self,
    ) -> Result<MutexGuard<'_, SubserverMap>, ProxySubserverRepositoryError> {
        self.subservers
            .lock()
            .map_err(|_| ProxySubserverRepositoryError::LockPoisoned)
    }
}

fn ensure_supported_proxy(proxy: &Instance) -> Result<(), ProxySubserverRepositoryError> {
    if proxy.kind().proxy_topology().allows_backend_count(1) {
        return Ok(());
    }

    Err(ProxySubserverRepositoryError::UnsupportedProxy {
        instance_id: proxy.id().clone(),
        kind: proxy.kind(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::ProxySubserverRepository;
    use crate::ProxySubserverRepositoryError;
    use nexus_domain::InstanceCreate;
    use nexus_domain::InstanceId;
    use nexus_domain::InstanceKind;
    use nexus_domain::LaunchConfig;
    use nexus_domain::ProxySubserver;

    #[test]
    fn enforces_the_one_to_one_topology() {
        let repository = ProxySubserverRepository::new();
        let proxy = instance("geyser", InstanceKind::Geyser);
        let first = subserver("first", "target-one");
        let second = subserver("second", "target-two");

        repository
            .upsert(&proxy, first)
            .expect("first Geyser target is accepted");
        assert!(matches!(
            repository.upsert(&proxy, second),
            Err(ProxySubserverRepositoryError::TopologyLimit { .. })
        ));
    }

    #[test]
    fn permits_replacing_a_one_to_one_target() {
        let repository = ProxySubserverRepository::new();
        let proxy = instance("geyser", InstanceKind::Geyser);

        repository
            .upsert(&proxy, subserver("default", "target-one"))
            .expect("initial Geyser target is accepted");
        repository
            .upsert(&proxy, subserver("default", "target-two"))
            .expect("same Geyser target entry can be updated");

        let items = repository.list(&proxy).expect("Geyser targets are listed");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].target_instance_id().to_string(), "target-two");
    }

    fn instance(id: &str, kind: InstanceKind) -> nexus_domain::Instance {
        InstanceCreate::new(
            InstanceId::new(id.to_owned()).expect("instance ID is valid"),
            id.to_owned(),
            kind,
            format!("instances/{id}"),
            LaunchConfig::new(
                "java".to_owned(),
                Vec::new(),
                BTreeMap::new(),
                "stop".to_owned(),
                30,
            ),
        )
        .expect("instance definition is valid")
        .into_instance()
        .expect("instance is valid")
    }

    fn subserver(id: &str, target: &str) -> ProxySubserver {
        ProxySubserver::new(
            id.to_owned(),
            id.to_owned(),
            InstanceId::new(target.to_owned()).expect("target ID is valid"),
            "127.0.0.1".to_owned(),
            25565,
            true,
        )
        .expect("proxy subserver is valid")
    }
}
