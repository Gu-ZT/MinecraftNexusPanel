use serde::Deserialize;
use serde::Serialize;

use crate::InstanceCreate;
use crate::InstanceId;
use crate::InstanceKind;
use crate::InstanceRuntime;
use crate::InstanceUpdate;
use crate::InstanceUpdateError;
use crate::LaunchConfig;
use crate::PatchField;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    id: InstanceId,
    name: String,
    kind: InstanceKind,
    directory: String,
    launch: LaunchConfig,
    #[serde(default)]
    update_command: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
    runtime: InstanceRuntime,
    revision: u64,
}

impl Instance {
    pub(crate) fn from_create(instance: InstanceCreate) -> Self {
        let (id, name, kind, directory, launch) = instance.into_parts();

        Self {
            id,
            name,
            kind,
            directory,
            launch,
            update_command: None,
            expires_at: None,
            runtime: InstanceRuntime::created(),
            revision: 1,
        }
    }

    #[must_use]
    pub fn id(&self) -> &InstanceId {
        &self.id
    }

    #[must_use]
    pub fn directory(&self) -> &str {
        &self.directory
    }

    #[must_use]
    pub fn expires_at(&self) -> Option<&str> {
        self.expires_at.as_deref()
    }

    #[must_use]
    pub fn launch(&self) -> &LaunchConfig {
        &self.launch
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn kind(&self) -> InstanceKind {
        self.kind
    }

    #[must_use]
    pub fn runtime(&self) -> &InstanceRuntime {
        &self.runtime
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn update_command(&self) -> Option<&str> {
        self.update_command.as_deref()
    }

    pub fn apply_update(&mut self, update: &InstanceUpdate) -> Result<(), InstanceUpdateError> {
        update.validate()?;
        let mut changed = false;

        if let PatchField::Set(name) = update.name()
            && &self.name != name
        {
            self.name.clone_from(name);
            changed = true;
        }
        if let PatchField::Set(kind) = update.kind()
            && self.kind != *kind
        {
            self.kind = *kind;
            changed = true;
        }
        if let PatchField::Set(directory) = update.directory()
            && &self.directory != directory
        {
            self.directory.clone_from(directory);
            changed = true;
        }
        if let PatchField::Set(launch) = update.launch()
            && &self.launch != launch
        {
            self.launch.clone_from(launch);
            changed = true;
        }
        changed |= apply_optional_patch(&mut self.update_command, update.update_command());
        changed |= apply_optional_patch(&mut self.expires_at, update.expires_at());

        if changed {
            self.revision = self.revision.saturating_add(1);
        }

        Ok(())
    }

    pub fn set_runtime(&mut self, runtime: InstanceRuntime) {
        self.runtime = runtime;
    }
}

fn apply_optional_patch(value: &mut Option<String>, patch: &PatchField<String>) -> bool {
    match patch {
        PatchField::Unchanged => false,
        PatchField::Set(next) if value.as_ref() == Some(next) => false,
        PatchField::Set(next) => {
            value.replace(next.clone());
            true
        }
        PatchField::Clear if value.is_none() => false,
        PatchField::Clear => {
            value.take();
            true
        }
    }
}
