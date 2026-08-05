use thiserror::Error;

/// CPU policy 领域输入校验错误。
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CpuPolicyError {
    /// 自定义模式没有提供 CPU ID。
    #[error("custom CPU policy requires at least one requested CPU ID")]
    CustomCpuIdsRequired,
    /// policy 包含重复的 CPU ID。
    #[error("CPU policy contains duplicate CPU IDs")]
    DuplicateCpuIds,
    /// policy 同时提供了不适用的 requested CPU 集合。
    #[error("requested CPU IDs are only valid for custom CPU policy")]
    RequestedCpuIdsOnlyForCustom,
    /// 最小 CPU 数量为零或超出允许范围。
    #[error("minimum CPU count must be between 1 and 1000000")]
    InvalidMinimumCpuCount,
    /// 最大 CPU 数量小于最小 CPU 数量或超出允许范围。
    #[error("maximum CPU count must be between minimum CPU count and 1000000")]
    InvalidMaximumCpuCount,
}
