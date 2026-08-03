use std::collections::BTreeMap;
use std::io;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use nexus_domain::InstanceId;
use nexus_domain::InstanceState;
use nexus_domain::TaskId;
use nexus_protocol::WireMessage;
use tokio::io::AsyncWriteExt;
use tokio::process::Child;
use tokio::process::ChildStdin;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio::time::sleep;

use crate::InstanceProcess;
use crate::InstanceProcessCommand;
use crate::InstanceProcessError;
use crate::InstanceRepository;
use crate::InstanceRepositoryError;
use crate::instance_process_manager::mark_failed;
use crate::instance_process_manager::publish_state;

const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(25);

#[cfg(not(windows))]
const LINE_ENDING: &[u8] = b"\n";
#[cfg(windows)]
const LINE_ENDING: &[u8] = b"\r\n";

pub(crate) struct InstanceProcessSupervisor {
    pub(crate) child: Child,
    pub(crate) command_receiver: mpsc::Receiver<InstanceProcessCommand>,
    pub(crate) event_sender: broadcast::Sender<WireMessage>,
    pub(crate) instance_id: InstanceId,
    pub(crate) instances: InstanceRepository,
    pub(crate) process_id: TaskId,
    pub(crate) processes: Weak<Mutex<BTreeMap<InstanceId, InstanceProcess>>>,
    pub(crate) sequence: Arc<AtomicU64>,
    pub(crate) stdin: ChildStdin,
    pub(crate) stop_command: String,
}

impl InstanceProcessSupervisor {
    pub(crate) async fn run(mut self) {
        let mut kill_requested = false;
        let mut stop_deadline = None;
        let mut stopping = false;
        let mut timed_out = false;

        loop {
            select! {
                command = self.command_receiver.recv() => {
                    let Some(command) = command else {
                        let _ = self.child.start_kill();
                        return;
                    };
                    match command {
                        InstanceProcessCommand::Kill { acknowledged } => {
                            let accepted = self.child.start_kill().is_ok();
                            kill_requested = accepted;
                            let _ = acknowledged.send(accepted);
                        }
                        InstanceProcessCommand::SendCommand {
                            acknowledged,
                            command,
                        } => {
                            let accepted = !stopping
                                && !kill_requested
                                && self.write_command(&command).await.is_ok();
                            let _ = acknowledged.send(accepted);
                        }
                        InstanceProcessCommand::Stop { acknowledged, timeout } => {
                            let accepted = !stopping
                                && !kill_requested
                                && self.write_stop_command().await.is_ok()
                                && self.mark_stopping().is_ok();
                            if accepted {
                                stopping = true;
                                stop_deadline = Some(Instant::now() + timeout);
                            }
                            let _ = acknowledged.send(accepted);
                        }
                    }
                }
                () = sleep(PROCESS_POLL_INTERVAL) => {}
            }

            if stop_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                timed_out = true;
                kill_requested = self.child.start_kill().is_ok();
                stop_deadline = None;
            }

            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.remove_process();
                    let exit_code = status.code();
                    if timed_out || (!stopping && !kill_requested) {
                        mark_failed(
                            &self.instances,
                            &self.event_sender,
                            &self.sequence,
                            &self.instance_id,
                            exit_code,
                        );
                    } else {
                        self.mark_stopped(exit_code);
                    }
                    return;
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::error!(instance_id = %self.instance_id, %error, "Unable to query instance process state");
                    let _ = self.child.start_kill();
                    self.remove_process();
                    mark_failed(
                        &self.instances,
                        &self.event_sender,
                        &self.sequence,
                        &self.instance_id,
                        None,
                    );
                    return;
                }
            }
        }
    }

    async fn write_stop_command(&mut self) -> Result<(), io::Error> {
        let stop_command = self.stop_command.clone();
        self.write_command(&stop_command).await
    }

    async fn write_command(&mut self, command: &str) -> Result<(), io::Error> {
        let mut input = Vec::with_capacity(command.len() + LINE_ENDING.len());
        input.extend_from_slice(command.as_bytes());
        input.extend_from_slice(LINE_ENDING);
        self.stdin.write_all(&input).await?;
        self.stdin.flush().await
    }

    fn mark_stopping(&self) -> Result<(), InstanceProcessError> {
        let instance = self.instances.get(&self.instance_id)?.ok_or_else(|| {
            InstanceRepositoryError::NotFound {
                instance_id: self.instance_id.clone(),
            }
        })?;
        let instance = self.instances.transition_runtime(
            &self.instance_id,
            &[InstanceState::Running],
            instance.runtime().stopping(),
        )?;
        publish_state(&self.event_sender, &self.sequence, &instance);

        Ok(())
    }

    fn mark_stopped(&self, exit_code: Option<i32>) {
        let Ok(Some(instance)) = self.instances.get(&self.instance_id) else {
            return;
        };
        match self
            .instances
            .set_runtime(&self.instance_id, instance.runtime().stopped(exit_code))
        {
            Ok(instance) => publish_state(&self.event_sender, &self.sequence, &instance),
            Err(error) => {
                tracing::error!(instance_id = %self.instance_id, %error, "Unable to record stopped instance state")
            }
        }
    }

    fn remove_process(&self) {
        let Some(processes) = self.processes.upgrade() else {
            return;
        };
        let Ok(mut processes) = processes.lock() else {
            return;
        };
        let matches_process = processes
            .get(&self.instance_id)
            .is_some_and(|process| process.process_id() == self.process_id);
        if matches_process {
            processes.remove(&self.instance_id);
        }
    }
}
