use serde::Deserialize;
use serde::Serialize;

/// 单个逻辑 CPU 的性能类别。
///
/// `Unknown` 是保守结果：在操作系统没有暴露可靠性能分类时，调度器不得
/// 根据 CPU 编号或枚举顺序把逻辑 CPU 猜成性能核或能效核。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CpuPerformanceClass {
    /// 操作系统明确报告为高性能类别。
    Performance,
    /// 操作系统明确报告为高能效类别。
    Efficiency,
    /// 当前平台没有提供可验证的性能类别。
    Unknown,
}
