use nexus_domain::InstanceId;
use serde::Deserialize;

/// 用户从安装模板创建实例时提交的版本和实例参数。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TemplateProvisionRequest {
    instance_id: InstanceId,
    instance_name: String,
    instance_directory: String,
    minecraft_version: String,
    loader_version: String,
    #[serde(default)]
    runtime_id: Option<String>,
    #[serde(default)]
    jvm_arguments: Vec<String>,
    #[serde(default = "default_stop_command")]
    stop_command: String,
    #[serde(default = "default_stop_timeout_seconds")]
    stop_timeout_seconds: u16,
}

impl TemplateProvisionRequest {
    /// 返回目标实例标识。
    #[must_use]
    pub(crate) const fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    /// 返回实例显示名称。
    #[must_use]
    pub(crate) fn instance_name(&self) -> &str {
        &self.instance_name
    }

    /// 返回 Core 数据目录内的实例相对目录。
    #[must_use]
    pub(crate) fn instance_directory(&self) -> &str {
        &self.instance_directory
    }

    /// 返回用户选择的 Minecraft 版本。
    #[must_use]
    pub(crate) fn minecraft_version(&self) -> &str {
        &self.minecraft_version
    }

    /// 返回用户选择的加载器版本。
    #[must_use]
    pub(crate) fn loader_version(&self) -> &str {
        &self.loader_version
    }

    /// 返回可选的受管运行时标识；缺省时使用匹配版本的系统运行时。
    #[must_use]
    pub(crate) fn runtime_id(&self) -> Option<&str> {
        self.runtime_id.as_deref()
    }

    /// 返回写入 `user_jvm_args.txt` 的 JVM 参数。
    #[must_use]
    pub(crate) fn jvm_arguments(&self) -> &[String] {
        &self.jvm_arguments
    }

    /// 返回优雅停止命令。
    #[must_use]
    pub(crate) fn stop_command(&self) -> &str {
        &self.stop_command
    }

    /// 返回优雅停止等待时长。
    #[must_use]
    pub(crate) const fn stop_timeout_seconds(&self) -> u16 {
        self.stop_timeout_seconds
    }
}

fn default_stop_command() -> String {
    "stop".to_owned()
}

const fn default_stop_timeout_seconds() -> u16 {
    30
}
