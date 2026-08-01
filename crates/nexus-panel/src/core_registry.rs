use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use nexus_domain::CoreId;
use nexus_domain::PRODUCT_NAME;
use nexus_protocol::PresharedKey;
use nexus_protocol::ProtocolVersion;
use nexus_protocol::SessionError;
use nexus_storage::NewCore;
use nexus_storage::SqliteStore;
use nexus_storage::StoredCore;
use serde_json::Value;
use serde_json::from_str;
use serde_json::json;
use serde_json::to_string;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::select;
use tokio::spawn;
use tokio::sync::RwLock;
use tokio::sync::watch::Receiver;
use tokio::sync::watch::Sender;
use tokio::sync::watch::channel;
use tokio::task::spawn_blocking;
use tokio::time::sleep;
use tokio::time::timeout;
use tracing::warn;

use crate::CoreConnection;
use crate::CoreConnectionError;
use crate::CoreCreate;
use crate::CoreEndpoint;
use crate::CoreRegistryError;
use crate::CoreRuntime;
use crate::CoreStatus;
use crate::ManagedCore;
use crate::SecretCipher;

const INITIAL_RECONNECT_DELAY_SECONDS: u64 = 1;
const MAXIMUM_RECONNECT_DELAY_SECONDS: u64 = 30;

#[derive(Clone)]
pub struct CoreRegistry {
    store: SqliteStore,
    cipher: SecretCipher,
    panel_id: String,
    entries: Arc<RwLock<HashMap<CoreId, Arc<ManagedCore>>>>,
    shutdown: Sender<()>,
}

impl CoreRegistry {
    pub fn new(
        store: SqliteStore,
        cipher: SecretCipher,
        panel_id: String,
    ) -> Result<Self, CoreRegistryError> {
        let mut entries = HashMap::new();
        for registration in store.list_cores()? {
            let core_id = CoreId::from_str(registration.id()).map_err(|_| {
                CoreRegistryError::InvalidStoredCore {
                    core_id: registration.id().to_owned(),
                }
            })?;
            let encoded_secret = cipher.decrypt(core_id, registration.secret_envelope())?;
            let encoded_secret = String::from_utf8(encoded_secret).map_err(|_| {
                CoreRegistryError::InvalidStoredCore {
                    core_id: registration.id().to_owned(),
                }
            })?;
            let pre_shared_key = PresharedKey::from_base64url(&encoded_secret).map_err(|_| {
                CoreRegistryError::InvalidStoredCore {
                    core_id: registration.id().to_owned(),
                }
            })?;
            from_str::<Vec<String>>(registration.tags_json())?;
            entries.insert(
                core_id,
                Arc::new(ManagedCore::new(
                    registration,
                    pre_shared_key,
                    None,
                    CoreRuntime::unknown(),
                )),
            );
        }
        let existing: Vec<_> = entries.values().cloned().collect();
        let (shutdown, _) = channel(());
        let registry = Self {
            store,
            cipher,
            panel_id,
            entries: Arc::new(RwLock::new(entries)),
            shutdown,
        };
        for core in existing {
            registry.spawn_connection_monitor(core);
        }

        Ok(registry)
    }

    pub async fn register(&self, request: &CoreCreate) -> Result<Value, CoreRegistryError> {
        if let Some(field) = request.invalid_field() {
            return Err(CoreRegistryError::InvalidRequest { field });
        }
        let pre_shared_key = PresharedKey::from_base64url(request.secret())
            .map_err(CoreRegistryError::InvalidSecret)?;
        let (connection, runtime) = establish_connection(
            request.address(),
            request.skip_certificate_verification(),
            request.connect_timeout_seconds(),
            &pre_shared_key,
            &self.panel_id,
        )
        .await?;
        let core_id = connection.core_id();
        let now = current_timestamp();
        let new_core = NewCore {
            id: core_id.to_string(),
            name: request.name().to_owned(),
            address: request.address().to_owned(),
            secret_envelope: self.cipher.encrypt(core_id, request.secret().as_bytes())?,
            secret_updated_at: now.clone(),
            connect_timeout_seconds: request.connect_timeout_seconds(),
            skip_certificate_verification: request.skip_certificate_verification(),
            tags_json: to_string(&request.normalized_tags())?,
            created_at: now,
        };
        let registration = StoredCore::from_new(&new_core);
        let store = self.store.clone();
        let inserted = spawn_blocking(move || store.insert_core(&new_core)).await??;
        if !inserted {
            return Err(CoreRegistryError::AlreadyExists { core_id });
        }
        let core = Arc::new(ManagedCore::new(
            registration,
            pre_shared_key,
            Some(connection),
            runtime,
        ));
        self.entries.write().await.insert(core_id, core.clone());
        self.spawn_connection_monitor(core.clone());

        core_json(&core).await
    }

    pub async fn list(&self) -> Result<Value, CoreRegistryError> {
        let mut cores: Vec<_> = self.entries.read().await.values().cloned().collect();
        cores.sort_by(|left, right| left.registration.id().cmp(right.registration.id()));
        let mut items = Vec::with_capacity(cores.len());
        for core in cores {
            items.push(core_json(&core).await?);
        }

        Ok(json!({
            "items": items,
            "nextCursor": null,
        }))
    }

    pub async fn get(&self, core_id: CoreId) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        core_json(&core).await
    }

    pub async fn test_connection(&self, core_id: CoreId) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        let started_at = Instant::now();
        let (connection, _) = establish_connection(
            core.registration.address(),
            core.registration.skip_certificate_verification(),
            core.registration.connect_timeout_seconds(),
            &core.pre_shared_key,
            &self.panel_id,
        )
        .await?;

        Ok(json!({
            "success": true,
            "latencyMs": elapsed_milliseconds(started_at),
            "protocolVersion": protocol_text(connection.protocol()),
        }))
    }

    pub async fn reconnect(&self, core_id: CoreId) -> Result<Value, CoreRegistryError> {
        let core = self.find(core_id).await?;
        core.connection.lock().await.take();
        core.runtime.write().await.mark_reconnecting();
        core.reconnect.notify_waiters();

        core_json(&core).await
    }

    async fn find(&self, core_id: CoreId) -> Result<Arc<ManagedCore>, CoreRegistryError> {
        self.entries
            .read()
            .await
            .get(&core_id)
            .cloned()
            .ok_or(CoreRegistryError::NotFound { core_id })
    }

    fn spawn_connection_monitor(&self, core: Arc<ManagedCore>) {
        let panel_id = self.panel_id.clone();
        let shutdown = self.shutdown.subscribe();
        spawn(async move {
            monitor_connection(core, panel_id, shutdown).await;
        });
    }
}

async fn monitor_connection(core: Arc<ManagedCore>, panel_id: String, mut shutdown: Receiver<()>) {
    let mut reconnect_delay_seconds = INITIAL_RECONNECT_DELAY_SECONDS;
    loop {
        if core.connection.lock().await.is_none() {
            match establish_connection(
                core.registration.address(),
                core.registration.skip_certificate_verification(),
                core.registration.connect_timeout_seconds(),
                &core.pre_shared_key,
                &panel_id,
            )
            .await
            {
                Ok((connection, runtime)) => {
                    *core.connection.lock().await = Some(connection);
                    *core.runtime.write().await = runtime;
                    reconnect_delay_seconds = INITIAL_RECONNECT_DELAY_SECONDS;
                }
                Err(error) => {
                    core.runtime
                        .write()
                        .await
                        .mark_failure(status_for_error(&error));
                    warn!(
                        core_id = core.registration.id(),
                        error = %error,
                        "Core connection attempt failed"
                    );
                    select! {
                        () = sleep(Duration::from_secs(reconnect_delay_seconds)) => {}
                        () = core.reconnect.notified() => {}
                        result = shutdown.changed() => {
                            if result.is_err() {
                                return;
                            }
                        }
                    }
                    reconnect_delay_seconds =
                        (reconnect_delay_seconds * 2).min(MAXIMUM_RECONNECT_DELAY_SECONDS);
                    continue;
                }
            }
        }

        let heartbeat_seconds = core.connection.lock().await.as_ref().map_or(
            INITIAL_RECONNECT_DELAY_SECONDS,
            CoreConnection::heartbeat_seconds,
        );
        select! {
            () = sleep(Duration::from_secs(heartbeat_seconds)) => {}
            () = core.reconnect.notified() => {
                core.connection.lock().await.take();
                core.runtime.write().await.mark_reconnecting();
                continue;
            }
            result = shutdown.changed() => {
                if result.is_err() {
                    return;
                }
            }
        }

        let started_at = Instant::now();
        let ping_result = {
            let mut connection = core.connection.lock().await;
            let result = match connection.as_mut() {
                Some(connection) => timeout(
                    Duration::from_secs(u64::from(core.registration.connect_timeout_seconds())),
                    connection.ping(),
                )
                .await
                .map_err(|_| CoreRegistryError::ConnectionTimeout)
                .and_then(|result| result.map_err(CoreRegistryError::from)),
                None => continue,
            };
            if result.is_err() {
                connection.take();
            }
            result
        };
        match ping_result {
            Ok(_) => core
                .runtime
                .write()
                .await
                .mark_ping(elapsed_milliseconds(started_at), current_timestamp()),
            Err(error) => {
                core.runtime
                    .write()
                    .await
                    .mark_failure(status_for_error(&error));
                warn!(
                    core_id = core.registration.id(),
                    error = %error,
                    "Core heartbeat failed"
                );
            }
        }
    }
}

async fn establish_connection(
    address: &str,
    skip_certificate_verification: bool,
    connect_timeout_seconds: u32,
    pre_shared_key: &PresharedKey,
    panel_id: &str,
) -> Result<(CoreConnection, CoreRuntime), CoreRegistryError> {
    let endpoint = CoreEndpoint::parse(address, skip_certificate_verification)
        .map_err(CoreConnectionError::from)?;
    let started_at = Instant::now();
    let mut connection = timeout(
        Duration::from_secs(u64::from(connect_timeout_seconds)),
        CoreConnection::connect_endpoint(&endpoint, pre_shared_key, panel_id, PRODUCT_NAME),
    )
    .await
    .map_err(|_| CoreRegistryError::ConnectionTimeout)??;
    let system_info = timeout(
        Duration::from_secs(u64::from(connect_timeout_seconds)),
        connection.system_info(),
    )
    .await
    .map_err(|_| CoreRegistryError::ConnectionTimeout)??;
    let runtime = CoreRuntime {
        status: CoreStatus::Online,
        latency_milliseconds: Some(elapsed_milliseconds(started_at)),
        last_seen_at: Some(current_timestamp()),
        version: system_info
            .get("serverVersion")
            .and_then(Value::as_str)
            .map(str::to_owned),
        protocol_version: Some(protocol_text(connection.protocol())),
        capabilities: connection.capabilities().to_vec(),
        certificate_verified: Some(endpoint.verify_certificate()),
        tls_certificate_sha256: Some(connection.tls_certificate_sha256().to_owned()),
    };

    Ok((connection, runtime))
}

async fn core_json(core: &ManagedCore) -> Result<Value, CoreRegistryError> {
    let runtime = core.runtime.read().await.clone();
    let tags: Vec<String> = from_str(core.registration.tags_json())?;

    Ok(json!({
        "id": core.registration.id(),
        "name": core.registration.name(),
        "address": core.registration.address(),
        "status": runtime.status.as_str(),
        "latencyMs": runtime.latency_milliseconds,
        "lastSeenAt": runtime.last_seen_at,
        "version": runtime.version,
        "protocolVersion": runtime.protocol_version,
        "capabilities": runtime.capabilities,
        "secretConfigured": true,
        "secretUpdatedAt": core.registration.secret_updated_at(),
        "skipCertificateVerification": core.registration.skip_certificate_verification(),
        "certificateVerified": runtime.certificate_verified,
        "tlsCertificateSha256": runtime.tls_certificate_sha256,
        "tags": tags,
        "revision": core.registration.revision(),
    }))
}

fn status_for_error(error: &CoreRegistryError) -> CoreStatus {
    match error {
        CoreRegistryError::Connection(CoreConnectionError::ProtocolVersion(_)) => {
            CoreStatus::Incompatible
        }
        CoreRegistryError::Connection(
            CoreConnectionError::CertificateFingerprintMismatch
            | CoreConnectionError::Session(SessionError::Noise(_))
            | CoreConnectionError::Tls(_),
        ) => CoreStatus::AuthFailed,
        CoreRegistryError::Connection(_) | CoreRegistryError::ConnectionTimeout => {
            CoreStatus::Offline
        }
        _ => CoreStatus::Unknown,
    }
}

fn elapsed_milliseconds(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn protocol_text(protocol: ProtocolVersion) -> String {
    format!("{}.{}", protocol.major, protocol.minor)
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}
