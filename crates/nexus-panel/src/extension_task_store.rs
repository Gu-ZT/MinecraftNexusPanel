use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use nexus_domain::CoreId;
use nexus_domain::ExtensionInstall;
use nexus_domain::ExtensionKind;
use nexus_domain::InstanceId;
use nexus_domain::TaskId;
use serde_json::Value;
use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const MAXIMUM_EXTENSION_TASKS: usize = 512;

#[derive(Clone, Default)]
pub(crate) struct ExtensionTaskStore {
    tasks: Arc<Mutex<HashMap<TaskId, Value>>>,
}

impl ExtensionTaskStore {
    pub(crate) fn start(
        &self,
        core_id: CoreId,
        instance_id: &InstanceId,
        kind: ExtensionKind,
        total: usize,
        task_kind: &str,
        idempotency_key: &str,
    ) -> Result<(TaskId, bool), ()> {
        let mut tasks = self.tasks.lock().map_err(|_| ())?;
        let core_id = core_id.to_string();
        let instance_id = instance_id.to_string();
        if let Some(task_id) = tasks.iter().find_map(|(task_id, task)| {
            (task.get("_idempotencyKey").and_then(Value::as_str) == Some(idempotency_key)
                && task.get("coreId").and_then(Value::as_str) == Some(core_id.as_str())
                && task.get("instanceId").and_then(Value::as_str) == Some(instance_id.as_str())
                && task.get("kind").and_then(Value::as_str) == Some(task_kind)
                && task.get("extensionKind") == Some(&json!(kind)))
            .then_some(*task_id)
        }) {
            return Ok((task_id, false));
        }

        let task_id = TaskId::new();
        if tasks.len() >= MAXIMUM_EXTENSION_TASKS {
            let completed_task = tasks.iter().find_map(|(task_id, task)| {
                matches!(
                    task.get("state").and_then(Value::as_str),
                    Some("SUCCEEDED") | Some("FAILED")
                )
                .then_some(*task_id)
            });
            if let Some(completed_task) = completed_task {
                tasks.remove(&completed_task);
            } else {
                return Err(());
            }
        }
        tasks.insert(
            task_id,
            json!({
                "taskId": task_id,
                "coreId": core_id,
                "instanceId": instance_id,
                "kind": task_kind,
                "extensionKind": kind,
                "state": "RUNNING",
                "rollbackState": "NOT_STARTED",
                "progress": { "completed": 0, "total": total },
                "installations": [],
                "acceptedAt": current_timestamp(),
                "_idempotencyKey": idempotency_key,
            }),
        );
        Ok((task_id, true))
    }

    pub(crate) fn get(&self, task_id: TaskId) -> Result<Option<Value>, ()> {
        let task = self.tasks.lock().map_err(|_| ())?.get(&task_id).cloned();
        Ok(task.map(|mut task| {
            if let Some(object) = task.as_object_mut() {
                object.remove("_idempotencyKey");
            }
            task
        }))
    }

    pub(crate) fn update_progress(
        &self,
        task_id: TaskId,
        completed: usize,
        total: usize,
    ) -> Result<(), ()> {
        let mut tasks = self.tasks.lock().map_err(|_| ())?;
        let Some(task) = tasks.get_mut(&task_id) else {
            return Err(());
        };
        task["progress"] = json!({ "completed": completed, "total": total });
        Ok(())
    }

    pub(crate) fn complete(
        &self,
        task_id: TaskId,
        installations: &[ExtensionInstall],
    ) -> Result<(), ()> {
        let mut tasks = self.tasks.lock().map_err(|_| ())?;
        let Some(task) = tasks.get_mut(&task_id) else {
            return Err(());
        };
        let total = task["progress"]["total"].as_u64().unwrap_or_default();
        task["state"] = json!("SUCCEEDED");
        task["rollbackState"] = json!("NOT_NEEDED");
        task["progress"] = json!({ "completed": total, "total": total });
        task["installations"] = json!(installations);
        Ok(())
    }

    pub(crate) fn fail(
        &self,
        task_id: TaskId,
        completed: usize,
        installations: &[ExtensionInstall],
        error: &str,
        rollback_state: &str,
    ) -> Result<(), ()> {
        let mut tasks = self.tasks.lock().map_err(|_| ())?;
        let Some(task) = tasks.get_mut(&task_id) else {
            return Err(());
        };
        let total = task["progress"]["total"].as_u64().unwrap_or_default();
        task["state"] = json!("FAILED");
        task["rollbackState"] = json!(rollback_state);
        task["progress"] = json!({ "completed": completed, "total": total });
        task["installations"] = json!(installations);
        task["error"] = json!(error);
        Ok(())
    }
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(test)]
mod tests {
    use super::ExtensionTaskStore;
    use nexus_domain::CoreId;
    use nexus_domain::ExtensionKind;
    use nexus_domain::InstanceId;
    use serde_json::json;

    #[test]
    fn tracks_progress_and_terminal_states() {
        let store = ExtensionTaskStore::default();
        let instance_id = "survival"
            .parse::<InstanceId>()
            .expect("instance ID is valid");
        let core_id = CoreId::new();
        let task_id = store
            .start(
                core_id,
                &instance_id,
                ExtensionKind::Plugin,
                2,
                "EXTENSION_INSTALL",
                "request-1",
            )
            .expect("task is created");
        let (task_id, created) = task_id;
        assert!(created);

        let task = store
            .get(task_id)
            .expect("task lookup succeeds")
            .expect("task exists");
        assert_eq!(task["state"], "RUNNING");
        assert_eq!(task["rollbackState"], "NOT_STARTED");
        assert_eq!(task["progress"], json!({ "completed": 0, "total": 2 }));

        store
            .update_progress(task_id, 1, 2)
            .expect("progress update succeeds");
        store
            .complete(task_id, &[])
            .expect("task completion succeeds");
        let task = store
            .get(task_id)
            .expect("task lookup succeeds")
            .expect("task exists");
        assert_eq!(task["state"], "SUCCEEDED");
        assert_eq!(task["rollbackState"], "NOT_NEEDED");
        assert_eq!(task["progress"], json!({ "completed": 2, "total": 2 }));
    }

    #[test]
    fn records_a_failure_without_faking_full_progress() {
        let store = ExtensionTaskStore::default();
        let instance_id = "survival"
            .parse::<InstanceId>()
            .expect("instance ID is valid");
        let task_id = store
            .start(
                CoreId::new(),
                &instance_id,
                ExtensionKind::Mod,
                3,
                "EXTENSION_INSTALL",
                "request-2",
            )
            .expect("task is created");
        let (task_id, created) = task_id;
        assert!(created);

        store
            .fail(task_id, 1, &[], "artifact failed", "NOT_NEEDED")
            .expect("task failure is recorded");
        let task = store
            .get(task_id)
            .expect("task lookup succeeds")
            .expect("task exists");
        assert_eq!(task["state"], "FAILED");
        assert_eq!(task["rollbackState"], "NOT_NEEDED");
        assert_eq!(task["progress"], json!({ "completed": 1, "total": 3 }));
        assert_eq!(task["error"], "artifact failed");
    }

    #[test]
    fn reuses_a_task_for_the_same_scoped_idempotency_key() {
        let store = ExtensionTaskStore::default();
        let instance_id = "survival"
            .parse::<InstanceId>()
            .expect("instance ID is valid");
        let core_id = CoreId::new();

        let first = store
            .start(
                core_id,
                &instance_id,
                ExtensionKind::Plugin,
                1,
                "EXTENSION_INSTALL",
                "request-3",
            )
            .expect("first task is created");
        let second = store
            .start(
                core_id,
                &instance_id,
                ExtensionKind::Plugin,
                1,
                "EXTENSION_INSTALL",
                "request-3",
            )
            .expect("duplicate task is accepted");
        let update = store
            .start(
                core_id,
                &instance_id,
                ExtensionKind::Plugin,
                1,
                "EXTENSION_UPDATE",
                "request-3",
            )
            .expect("different operation can reuse a key");

        assert!(first.1);
        assert_eq!(second, (first.0, false));
        assert!(update.1);
        assert_ne!(update.0, first.0);
        let task = store
            .get(first.0)
            .expect("task lookup succeeds")
            .expect("task exists");
        assert!(task.get("_idempotencyKey").is_none());
    }
}
