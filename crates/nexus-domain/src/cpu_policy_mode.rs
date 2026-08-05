use serde::Deserialize;
use serde::Serialize;

/// 描述 CPU policy 选择候选 CPU 的方式。
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CpuPolicyMode {
    /// 在有已确认性能核时优先性能核，否则使用可调度 CPU。
    #[default]
    Auto,
    /// 只选择操作系统明确标记为性能类别的 CPU。
    Performance,
    /// 只选择操作系统明确标记为能效类别的 CPU。
    Efficiency,
    /// 只使用调用方明确提供的 CPU ID 集合。
    Custom,
}
