use serde::Deserialize;
use serde::Serialize;

/// 描述 Core 如何监督实例主进程。
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SupervisorMode {
    /// 直接启动实例可执行文件。
    #[default]
    Direct,
    /// 通过外部 MCDR 包装器启动实例。
    Mcdr,
}
