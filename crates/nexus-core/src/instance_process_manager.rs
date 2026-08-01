use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use nexus_domain::EventId;
use nexus_domain::Instance;
use nexus_domain::InstanceId;
use nexus_domain::InstanceLogPage;
use nexus_domain::InstanceLogStream;
use nexus_domain::InstanceMetricSample;
use nexus_domain::InstanceRuntime;
use nexus_domain::InstanceState;
use nexus_domain::TaskId;
use nexus_protocol::WireMessage;
use serde_json::json;
use sysinfo::Pid;
use sysinfo::ProcessesToUpdate;
use sysinfo::System;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::process::Command;
use tokio::spawn;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

use crate::InstanceLogStore;
use crate::InstanceProcess;
use crate::InstanceProcessError;
use crate::InstanceProcessSupervisor;
use crate::InstanceRepository;
use crate::InstanceRepositoryError;
use crate::spawn_output_reader;

const EVENT_CHANNEL_CAPACITY: usize = 256;
const MAXIMUM_COMMAND_BYTES: usize = 8 * 1024;
const PROCESS_COMMAND_CAPACITY: usize = 4;

#[derive(Clone)]
pub struct InstanceProcessManager {
    data_directory: Arc<PathBuf>,
    event_sender: broadcast::Sender<WireMessage>,
    instances: InstanceRepository,
    logs: InstanceLogStore,
    processes: Arc<Mutex<BTreeMap<InstanceId, InstanceProcess>>>,
    sequence: Arc<AtomicU64>,
    system: Arc<Mutex<System>>,
}

impl InstanceProcessManager {
    #[must_use]
    pub fn new(data_directory: PathBuf, instances: InstanceRepository) -> Self {
        let (event_sender, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let sequence = Arc::new(AtomicU64::new(1));

        Self {
            data_directory: Arc::new(data_directory),
            event_sender: event_sender.clone(),
            instances,
            logs: InstanceLogStore::new(event_sender, sequence.clone()),
            processes: Arc::new(Mutex::new(BTreeMap::new())),
            sequence,
            system: Arc::new(Mutex::new(System::new())),
        }
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<WireMessage> {
        self.event_sender.subscribe()
    }

    pub async fn start(&self, instance_id: &InstanceId) -> Result<TaskId, InstanceProcessError> {
        let instance = self.instances.transition_runtime(
            instance_id,
            &[
                InstanceState::Created,
                InstanceState::Failed,
                InstanceState::Stopped,
            ],
            InstanceRuntime::starting(),
        )?;
        self.publish_state(&instance);

        match self.spawn(instance).await {
            Ok(task_id) => Ok(task_id),
            Err(error) => {
                self.mark_failed(instance_id, None);
                Err(error)
            }
        }
    }

    pub async fn stop(
        &self,
        instance_id: &InstanceId,
        timeout_seconds: Option<u16>,
    ) -> Result<TaskId, InstanceProcessError> {
        let instance = self.require_state(instance_id, &[InstanceState::Running])?;
        let timeout_seconds =
            timeout_seconds.unwrap_or_else(|| instance.launch().stop_timeout_seconds());
        let process = self.process(instance_id)?;

        if !process
            .stop(Duration::from_secs(u64::from(timeout_seconds)))
            .await
        {
            return Err(InstanceProcessError::ProcessUnavailable {
                instance_id: instance_id.clone(),
            });
        }

        Ok(TaskId::new())
    }

    pub async fn kill(&self, instance_id: &InstanceId) -> Result<TaskId, InstanceProcessError> {
        self.require_state(
            instance_id,
            &[InstanceState::Running, InstanceState::Stopping],
        )?;
        let process = self.process(instance_id)?;

        if !process.kill().await {
            return Err(InstanceProcessError::ProcessUnavailable {
                instance_id: instance_id.clone(),
            });
        }

        Ok(TaskId::new())
    }

    pub async fn command(
        &self,
        instance_id: &InstanceId,
        command: &str,
    ) -> Result<String, InstanceProcessError> {
        let command = normalize_command(command)?;
        self.require_state(instance_id, &[InstanceState::Running])?;
        let process = self.process(instance_id)?;

        if !process.send_command(command).await {
            return Err(InstanceProcessError::ProcessUnavailable {
                instance_id: instance_id.clone(),
            });
        }

        Ok(current_timestamp())
    }

    pub fn logs(
        &self,
        instance_id: &InstanceId,
        after: Option<u64>,
        before: Option<u64>,
        limit: usize,
    ) -> Result<InstanceLogPage, InstanceProcessError> {
        self.require_instance(instance_id)?;

        self.logs
            .read(instance_id, after, before, limit)
            .map_err(Into::into)
    }

    pub fn metrics(
        &self,
        instance_id: &InstanceId,
    ) -> Result<InstanceMetricSample, InstanceProcessError> {
        let instance = self.require_state(
            instance_id,
            &[InstanceState::Running, InstanceState::Stopping],
        )?;
        let process_id =
            instance
                .runtime()
                .pid()
                .ok_or_else(|| InstanceProcessError::MetricsUnavailable {
                    instance_id: instance_id.clone(),
                })?;
        let process_id = Pid::from_u32(process_id);
        let mut system = self
            .system
            .lock()
            .map_err(|_| InstanceProcessError::SystemLockPoisoned)?;
        system.refresh_processes(ProcessesToUpdate::Some(&[process_id]), false);
        let process =
            system
                .process(process_id)
                .ok_or_else(|| InstanceProcessError::MetricsUnavailable {
                    instance_id: instance_id.clone(),
                })?;

        Ok(InstanceMetricSample::new(
            current_timestamp(),
            process.cpu_usage(),
            process.memory(),
            process.virtual_memory(),
            process.run_time(),
        ))
    }

    async fn spawn(&self, instance: Instance) -> Result<TaskId, InstanceProcessError> {
        let instance_id = instance.id().clone();
        let working_directory = self.prepare_working_directory(&instance)?;
        let mut command = Command::new(instance.launch().executable());
        command
            .args(instance.launch().args())
            .current_dir(working_directory)
            .envs(instance.launch().environment())
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (name, _) in env::vars_os().filter(|(name, _)| {
            name.to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("MCNP_")
        }) {
            command.env_remove(name);
        }
        let mut child = command
            .spawn()
            .map_err(|source| InstanceProcessError::Spawn {
                instance_id: instance_id.clone(),
                source,
            })?;
        let process_id = child
            .id()
            .ok_or_else(|| InstanceProcessError::UnknownProcessId {
                instance_id: instance_id.clone(),
            })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| InstanceProcessError::StdinUnavailable {
                instance_id: instance_id.clone(),
            })?;
        let stdout =
            child
                .stdout
                .take()
                .ok_or_else(|| InstanceProcessError::StdoutUnavailable {
                    instance_id: instance_id.clone(),
                })?;
        let stderr =
            child
                .stderr
                .take()
                .ok_or_else(|| InstanceProcessError::StderrUnavailable {
                    instance_id: instance_id.clone(),
                })?;
        let task_id = TaskId::new();
        let (command_sender, command_receiver) = mpsc::channel(PROCESS_COMMAND_CAPACITY);
        let process = InstanceProcess::new(task_id, command_sender);
        self.insert_process(instance_id.clone(), process)?;

        let started_at = current_timestamp();
        let running = InstanceRuntime::running(process_id, started_at);
        let running_instance = match self.instances.transition_runtime(
            &instance_id,
            &[InstanceState::Starting],
            running,
        ) {
            Ok(instance) => instance,
            Err(error) => {
                let _ = child.start_kill();
                self.remove_process(&instance_id, task_id);
                return Err(error.into());
            }
        };
        self.publish_state(&running_instance);
        spawn_output_reader(
            stdout,
            instance_id.clone(),
            InstanceLogStream::Stdout,
            self.logs.clone(),
        );
        spawn_output_reader(
            stderr,
            instance_id.clone(),
            InstanceLogStream::Stderr,
            self.logs.clone(),
        );

        let stop_command = instance.launch().stop_command().to_owned();
        let event_sender = self.event_sender.clone();
        let instances = self.instances.clone();
        let processes = Arc::downgrade(&self.processes);
        let sequence = self.sequence.clone();
        let supervisor = InstanceProcessSupervisor {
            child,
            command_receiver,
            event_sender,
            instance_id,
            instances,
            process_id: task_id,
            processes,
            sequence,
            stdin,
            stop_command,
        };
        spawn(async move {
            supervisor.run().await;
        });

        Ok(task_id)
    }

    fn prepare_working_directory(
        &self,
        instance: &Instance,
    ) -> Result<PathBuf, InstanceProcessError> {
        let working_directory = self.data_directory.join(instance.directory());
        fs::create_dir_all(&working_directory).map_err(|source| {
            InstanceProcessError::CreateWorkingDirectory {
                path: working_directory.clone(),
                source,
            }
        })?;
        let data_directory = fs::canonicalize(self.data_directory.as_ref()).map_err(|source| {
            InstanceProcessError::CanonicalizeDataDirectory {
                path: self.data_directory.as_ref().clone(),
                source,
            }
        })?;
        let working_directory = fs::canonicalize(&working_directory).map_err(|source| {
            InstanceProcessError::CanonicalizeWorkingDirectory {
                path: working_directory,
                source,
            }
        })?;
        if !working_directory.starts_with(&data_directory) {
            return Err(InstanceProcessError::WorkingDirectoryOutsideDataDirectory {
                path: working_directory,
            });
        }

        Ok(working_directory)
    }

    fn require_state(
        &self,
        instance_id: &InstanceId,
        allowed_states: &[InstanceState],
    ) -> Result<Instance, InstanceProcessError> {
        let instance = self.require_instance(instance_id)?;
        let state = instance.runtime().state();
        if !allowed_states.contains(&state) {
            return Err(InstanceRepositoryError::StateConflict {
                instance_id: instance_id.clone(),
                state,
            }
            .into());
        }

        Ok(instance)
    }

    fn require_instance(&self, instance_id: &InstanceId) -> Result<Instance, InstanceProcessError> {
        self.instances.get(instance_id)?.ok_or_else(|| {
            InstanceRepositoryError::NotFound {
                instance_id: instance_id.clone(),
            }
            .into()
        })
    }

    fn process(&self, instance_id: &InstanceId) -> Result<InstanceProcess, InstanceProcessError> {
        self.lock_processes()?
            .get(instance_id)
            .cloned()
            .ok_or_else(|| InstanceProcessError::ProcessUnavailable {
                instance_id: instance_id.clone(),
            })
    }

    fn insert_process(
        &self,
        instance_id: InstanceId,
        process: InstanceProcess,
    ) -> Result<(), InstanceProcessError> {
        let mut processes = self.lock_processes()?;
        if processes.contains_key(&instance_id) {
            return Err(InstanceProcessError::ProcessUnavailable { instance_id });
        }
        processes.insert(instance_id, process);

        Ok(())
    }

    fn remove_process(&self, instance_id: &InstanceId, process_id: TaskId) {
        let Ok(mut processes) = self.lock_processes() else {
            return;
        };
        let matches_process = processes
            .get(instance_id)
            .is_some_and(|process| process.process_id() == process_id);
        if matches_process {
            processes.remove(instance_id);
        }
    }

    fn mark_failed(&self, instance_id: &InstanceId, exit_code: Option<i32>) {
        mark_failed(
            &self.instances,
            &self.event_sender,
            &self.sequence,
            instance_id,
            exit_code,
        );
    }

    fn publish_state(&self, instance: &Instance) {
        publish_state(&self.event_sender, &self.sequence, instance);
    }

    fn lock_processes(
        &self,
    ) -> Result<MutexGuard<'_, BTreeMap<InstanceId, InstanceProcess>>, InstanceProcessError> {
        self.processes
            .lock()
            .map_err(|_| InstanceProcessError::ProcessRegistryLockPoisoned)
    }
}

pub(crate) fn mark_failed(
    instances: &InstanceRepository,
    event_sender: &broadcast::Sender<WireMessage>,
    sequence: &AtomicU64,
    instance_id: &InstanceId,
    exit_code: Option<i32>,
) {
    let Ok(Some(instance)) = instances.get(instance_id) else {
        return;
    };
    match instances.set_runtime(instance_id, instance.runtime().failed(exit_code)) {
        Ok(instance) => publish_state(event_sender, sequence, &instance),
        Err(error) => {
            tracing::error!(%instance_id, %error, "Unable to record failed instance state")
        }
    }
}

pub(crate) fn publish_state(
    event_sender: &broadcast::Sender<WireMessage>,
    sequence: &AtomicU64,
    instance: &Instance,
) {
    let event = WireMessage::Event {
        event_id: EventId::new(),
        topic: "instance.state".to_owned(),
        sequence: sequence.fetch_add(1, Ordering::Relaxed),
        occurred_at: current_timestamp(),
        data: json!({
            "instanceId": instance.id(),
            "runtime": instance.runtime(),
        }),
    };
    let _ = event_sender.send(event);
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

fn normalize_command(command: &str) -> Result<String, InstanceProcessError> {
    let command = command.trim_end_matches(['\r', '\n']);
    if command.is_empty() {
        return Err(InstanceProcessError::CommandEmpty);
    }
    if command.as_bytes().contains(&0) {
        return Err(InstanceProcessError::CommandContainsNul);
    }
    if command.len() > MAXIMUM_COMMAND_BYTES {
        return Err(InstanceProcessError::CommandTooLong {
            maximum_bytes: MAXIMUM_COMMAND_BYTES,
        });
    }

    Ok(command.to_owned())
}
