use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use crate::McdrConfig;
use crate::RuntimeMode;
use crate::SupervisorMode;

/// 描述实例进程的启动与优雅停止参数。
///
/// `executable` 和参数由 Core 组合为进程命令；环境变量使用有序映射，
/// 便于序列化结果稳定并让审计日志容易比较。停止超时只表达策略，
/// 不代表进程一定会在该时间内退出。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchConfig {
    executable: String,
    args: Vec<String>,
    #[serde(default)]
    environment: BTreeMap<String, String>,
    stop_command: String,
    stop_timeout_seconds: u16,
    #[serde(default)]
    runtime_mode: RuntimeMode,
    #[serde(default)]
    supervisor_mode: SupervisorMode,
    #[serde(default)]
    mcdr: Option<McdrConfig>,
}

impl LaunchConfig {
    /// 创建启动配置。
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        executable: String,
        args: Vec<String>,
        environment: BTreeMap<String, String>,
        stop_command: String,
        stop_timeout_seconds: u16,
    ) -> Self {
        Self {
            executable,
            args,
            environment,
            stop_command,
            stop_timeout_seconds,
            runtime_mode: RuntimeMode::Host,
            supervisor_mode: SupervisorMode::Direct,
            mcdr: None,
        }
    }

    /// 设置运行模式、监督模式和可选的 MCDR 包装器配置。
    #[must_use]
    pub fn with_execution(
        mut self,
        runtime_mode: RuntimeMode,
        supervisor_mode: SupervisorMode,
        mcdr: Option<McdrConfig>,
    ) -> Self {
        self.runtime_mode = runtime_mode;
        self.supervisor_mode = supervisor_mode;
        self.mcdr = mcdr;
        self
    }

    /// 返回要执行的可执行文件路径或命令名。
    #[must_use]
    pub fn executable(&self) -> &str {
        &self.executable
    }

    /// 返回传递给可执行文件的参数，保持原有顺序。
    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// 返回启动进程使用的环境变量。
    #[must_use]
    pub fn environment(&self) -> &BTreeMap<String, String> {
        &self.environment
    }

    /// 返回请求优雅停止时发送给实例的命令。
    #[must_use]
    pub fn stop_command(&self) -> &str {
        &self.stop_command
    }

    /// 返回等待优雅停止的最长时间，单位为秒。
    #[must_use]
    pub const fn stop_timeout_seconds(&self) -> u16 {
        self.stop_timeout_seconds
    }

    /// 返回实例进程运行位置。
    #[must_use]
    pub const fn runtime_mode(&self) -> RuntimeMode {
        self.runtime_mode
    }

    /// 返回 Core 使用的进程监督模式。
    #[must_use]
    pub const fn supervisor_mode(&self) -> SupervisorMode {
        self.supervisor_mode
    }

    /// 返回可选的 MCDR 包装器配置。
    #[must_use]
    pub const fn mcdr(&self) -> Option<&McdrConfig> {
        self.mcdr.as_ref()
    }

    /// 将启动配置解析为 Core 应执行的可执行文件和参数。
    ///
    /// `None` 表示监督模式配置不完整。调用方应把它视为配置错误，不能回退为
    /// 直接执行实例命令，否则会绕过管理员明确选择的 MCDR 包装器。
    #[must_use]
    pub fn resolved_process_command(&self) -> Option<(String, Vec<String>)> {
        match self.supervisor_mode {
            SupervisorMode::Direct if self.mcdr.is_none() => {
                Some((self.executable.clone(), self.args.clone()))
            }
            SupervisorMode::Mcdr => self
                .mcdr
                .as_ref()?
                .wrap_command(&self.executable, &self.args),
            SupervisorMode::Direct => None,
        }
    }

    /// 校验运行模式、监督模式和 MCDR 包装器之间的组合关系。
    pub(crate) fn is_valid_execution(&self) -> bool {
        match self.supervisor_mode {
            SupervisorMode::Direct => self.mcdr.is_none(),
            SupervisorMode::Mcdr => self.mcdr.as_ref().is_some_and(McdrConfig::is_valid),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LaunchConfig;
    use crate::McdrConfig;
    use crate::RuntimeMode;
    use crate::SupervisorMode;

    #[test]
    fn defaults_to_a_direct_host_process() {
        let config = LaunchConfig::new(
            "java".to_owned(),
            vec!["-jar".to_owned(), "server.jar".to_owned()],
            Default::default(),
            "stop".to_owned(),
            30,
        );

        assert_eq!(config.runtime_mode(), RuntimeMode::Host);
        assert_eq!(config.supervisor_mode(), SupervisorMode::Direct);
        assert_eq!(
            config.resolved_process_command(),
            Some((
                "java".to_owned(),
                vec!["-jar".to_owned(), "server.jar".to_owned()]
            ))
        );
    }

    #[test]
    fn expands_the_explicit_mcdr_child_command_placeholders() {
        let config = LaunchConfig::new(
            "java".to_owned(),
            vec!["-jar".to_owned(), "server.jar".to_owned()],
            Default::default(),
            "stop".to_owned(),
            30,
        )
        .with_execution(
            RuntimeMode::Host,
            SupervisorMode::Mcdr,
            Some(McdrConfig::new(
                "mcdreforged".to_owned(),
                vec![
                    "--server".to_owned(),
                    "{server}".to_owned(),
                    "--".to_owned(),
                    "{serverArgs}".to_owned(),
                ],
            )),
        );

        assert_eq!(
            config.resolved_process_command(),
            Some((
                "mcdreforged".to_owned(),
                vec![
                    "--server".to_owned(),
                    "java".to_owned(),
                    "--".to_owned(),
                    "-jar".to_owned(),
                    "server.jar".to_owned(),
                ]
            ))
        );
    }
}
