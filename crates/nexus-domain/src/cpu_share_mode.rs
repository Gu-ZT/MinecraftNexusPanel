use serde::Deserialize;
use serde::Serialize;

/// 描述实例 CPU 集合是否允许与其他实例共享。
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CpuShareMode {
    /// 只应用 affinity，不创建独占预留。
    #[default]
    Shared,
    /// 要求 Core 创建不重叠的独占预留。
    Exclusive,
}
