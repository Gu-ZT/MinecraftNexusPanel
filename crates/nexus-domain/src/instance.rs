use serde::Deserialize;
use serde::Serialize;

use crate::InstanceCreate;
use crate::InstanceId;
use crate::InstanceKind;
use crate::InstanceRuntime;
use crate::LaunchConfig;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    id: InstanceId,
    name: String,
    kind: InstanceKind,
    directory: String,
    launch: LaunchConfig,
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
    pub fn launch(&self) -> &LaunchConfig {
        &self.launch
    }

    #[must_use]
    pub fn runtime(&self) -> &InstanceRuntime {
        &self.runtime
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub fn set_runtime(&mut self, runtime: InstanceRuntime) {
        self.runtime = runtime;
    }
}
