use serde::Deserialize;
use serde::Serialize;

use crate::DownloadManifest;
use crate::InstallRuntimeRequirement;
use crate::InstanceId;
use crate::InstanceKind;
use crate::ProvisionFile;
use crate::ProvisionInstallStrategy;
use crate::RuntimeArchiveFormat;

/// 一键搭建实例所需的完整执行计划。
///
/// 计划同时描述模板版本、实例配置、运行时安装清单和启动参数，但它只是经审计的
/// 输入快照，不表示下载、解压、配置写入或启动动作已经成功完成。
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
    #[serde(default)]
    install_strategy: ProvisionInstallStrategy,
    executable_path: String,
    #[serde(default)]
    required_runtime_version: Option<String>,
    #[serde(default)]
    launch_arguments: Vec<String>,
    #[serde(default)]
    files: Vec<ProvisionFile>,
    stop_command: String,
    stop_timeout_seconds: u16,
}

impl ProvisionPlan {
    /// 创建实例搭建计划。
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
            install_strategy: ProvisionInstallStrategy::ExtractArchive,
            executable_path,
            required_runtime_version: None,
            launch_arguments,
            files: Vec::new(),
            stop_command,
            stop_timeout_seconds,
        }
    }

    /// 设置下载产物的安装策略。
    #[must_use]
    pub fn with_install_strategy(mut self, install_strategy: ProvisionInstallStrategy) -> Self {
        self.install_strategy = install_strategy;
        self
    }

    /// 设置安装和启动所需的运行时主版本。
    #[must_use]
    pub fn with_required_runtime_version(mut self, version: String) -> Self {
        self.required_runtime_version = Some(version);
        self
    }

    /// 设置安装完成后写入实例目录的文本文件。
    #[must_use]
    pub fn with_files(mut self, files: Vec<ProvisionFile>) -> Self {
        self.files = files;
        self
    }

    /// 返回安装模板标识。
    #[must_use]
    pub fn template_id(&self) -> &str {
        &self.template_id
    }

    /// 返回 Minecraft 版本。
    #[must_use]
    pub fn minecraft_version(&self) -> &str {
        &self.minecraft_version
    }

    /// 返回模板选定的构建标识。
    #[must_use]
    pub fn build(&self) -> &str {
        &self.build
    }

    /// 返回待创建实例标识。
    #[must_use]
    pub fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    /// 返回待创建实例名称。
    #[must_use]
    pub fn instance_name(&self) -> &str {
        &self.instance_name
    }

    /// 返回待创建实例类型。
    #[must_use]
    pub const fn instance_kind(&self) -> InstanceKind {
        self.instance_kind
    }

    /// 返回待创建实例目录。
    #[must_use]
    pub fn instance_directory(&self) -> &str {
        &self.instance_directory
    }

    /// 返回可选的更新命令。
    #[must_use]
    pub fn update_command(&self) -> Option<&str> {
        self.update_command.as_deref()
    }

    /// 返回可选的实例到期时间。
    #[must_use]
    pub fn expires_at(&self) -> Option<&str> {
        self.expires_at.as_deref()
    }

    /// 返回启动所需运行时类别。
    #[must_use]
    pub const fn required_runtime(&self) -> InstallRuntimeRequirement {
        self.required_runtime
    }

    /// 返回受管运行时标识；使用系统运行时时为 `None`。
    #[must_use]
    pub fn runtime_id(&self) -> Option<&str> {
        self.runtime_id.as_deref()
    }

    /// 返回需要校验的运行时下载清单。
    #[must_use]
    pub const fn archive(&self) -> &DownloadManifest {
        &self.archive
    }

    /// 返回运行时归档格式。
    #[must_use]
    pub const fn archive_format(&self) -> RuntimeArchiveFormat {
        self.archive_format
    }

    /// 返回下载产物的安装策略。
    #[must_use]
    pub const fn install_strategy(&self) -> ProvisionInstallStrategy {
        self.install_strategy
    }

    /// 返回解压后的可执行文件相对路径。
    #[must_use]
    pub fn executable_path(&self) -> &str {
        &self.executable_path
    }

    /// 返回安装和启动要求的运行时主版本。
    #[must_use]
    pub fn required_runtime_version(&self) -> Option<&str> {
        self.required_runtime_version.as_deref()
    }

    /// 返回启动参数，保持模板声明顺序。
    #[must_use]
    pub fn launch_arguments(&self) -> &[String] {
        &self.launch_arguments
    }

    /// 返回安装完成后写入的实例文本文件。
    #[must_use]
    pub fn files(&self) -> &[ProvisionFile] {
        &self.files
    }

    /// 返回优雅停止命令。
    #[must_use]
    pub fn stop_command(&self) -> &str {
        &self.stop_command
    }

    /// 返回优雅停止等待时长，单位为秒。
    #[must_use]
    pub const fn stop_timeout_seconds(&self) -> u16 {
        self.stop_timeout_seconds
    }
}
