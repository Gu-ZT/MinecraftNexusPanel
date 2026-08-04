use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

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
        }
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
}
