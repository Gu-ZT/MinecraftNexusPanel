use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

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

    #[must_use]
    pub fn executable(&self) -> &str {
        &self.executable
    }

    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    #[must_use]
    pub fn environment(&self) -> &BTreeMap<String, String> {
        &self.environment
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
