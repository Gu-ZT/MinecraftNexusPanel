use serde::Deserialize;
use serde::Serialize;

use crate::DownloadManifest;
use crate::InstallRuntimeRequirement;
use crate::InstanceId;
use crate::InstanceKind;
use crate::RuntimeArchiveFormat;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionPlan {
    template_id: String,
    minecraft_version: String,
    build: String,
    instance_id: InstanceId,
    instance_name: String,
    instance_kind: InstanceKind,
    instance_directory: String,
    #[serde(default)]
    update_command: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
    required_runtime: InstallRuntimeRequirement,
    #[serde(default)]
    runtime_id: Option<String>,
    archive: DownloadManifest,
    archive_format: RuntimeArchiveFormat,
    executable_path: String,
    #[serde(default)]
    launch_arguments: Vec<String>,
    stop_command: String,
    stop_timeout_seconds: u16,
}

impl ProvisionPlan {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        template_id: String,
        minecraft_version: String,
        build: String,
        instance_id: InstanceId,
        instance_name: String,
        instance_kind: InstanceKind,
        instance_directory: String,
        update_command: Option<String>,
        expires_at: Option<String>,
        required_runtime: InstallRuntimeRequirement,
        runtime_id: Option<String>,
        archive: DownloadManifest,
        archive_format: RuntimeArchiveFormat,
        executable_path: String,
        launch_arguments: Vec<String>,
        stop_command: String,
        stop_timeout_seconds: u16,
    ) -> Self {
        Self {
            template_id,
            minecraft_version,
            build,
            instance_id,
            instance_name,
            instance_kind,
            instance_directory,
            update_command,
            expires_at,
            required_runtime,
            runtime_id,
            archive,
            archive_format,
            executable_path,
            launch_arguments,
            stop_command,
            stop_timeout_seconds,
        }
    }

    #[must_use]
    pub fn template_id(&self) -> &str {
        &self.template_id
    }

    #[must_use]
    pub fn minecraft_version(&self) -> &str {
        &self.minecraft_version
    }

    #[must_use]
    pub fn build(&self) -> &str {
        &self.build
    }

    #[must_use]
    pub fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    #[must_use]
    pub fn instance_name(&self) -> &str {
        &self.instance_name
    }

    #[must_use]
    pub const fn instance_kind(&self) -> InstanceKind {
        self.instance_kind
    }

    #[must_use]
    pub fn instance_directory(&self) -> &str {
        &self.instance_directory
    }

    #[must_use]
    pub fn update_command(&self) -> Option<&str> {
        self.update_command.as_deref()
    }

    #[must_use]
    pub fn expires_at(&self) -> Option<&str> {
        self.expires_at.as_deref()
    }

    #[must_use]
    pub const fn required_runtime(&self) -> InstallRuntimeRequirement {
        self.required_runtime
    }

    #[must_use]
    pub fn runtime_id(&self) -> Option<&str> {
        self.runtime_id.as_deref()
    }

    #[must_use]
    pub const fn archive(&self) -> &DownloadManifest {
        &self.archive
    }

    #[must_use]
    pub const fn archive_format(&self) -> RuntimeArchiveFormat {
        self.archive_format
    }

    #[must_use]
    pub fn executable_path(&self) -> &str {
        &self.executable_path
    }

    #[must_use]
    pub fn launch_arguments(&self) -> &[String] {
        &self.launch_arguments
    }

    #[must_use]
    pub fn stop_command(&self) -> &str {
        &self.stop_command
    }

    #[must_use]
    pub const fn stop_timeout_seconds(&self) -> u16 {
        self.stop_timeout_seconds
    }
}
