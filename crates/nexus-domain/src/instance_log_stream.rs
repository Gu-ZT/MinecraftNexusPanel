use serde::Deserialize;
use serde::Serialize;

/// 实例日志的来源流。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InstanceLogStream {
    /// 标准错误输出。
    Stderr,
    /// 标准输出。
    Stdout,
    /// Core 或运行时产生的系统事件输出。
    System,
}
