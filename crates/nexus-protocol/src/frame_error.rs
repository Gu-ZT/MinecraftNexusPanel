use thiserror::Error;

/// 长度前缀帧编解码错误。
#[derive(Debug, Error, Eq, PartialEq)]
pub enum FrameError {
    /// 帧长度为零，协议不允许空负载帧。
    #[error("a protocol frame cannot be empty")]
    EmptyFrame,
    /// 帧负载超过当前编解码器配置的上限。
    #[error("frame length {actual} exceeds the maximum of {maximum} bytes")]
    FrameTooLarge { actual: usize, maximum: usize },
    /// 输入包含完整帧后仍有额外字节，而调用方要求恰好一个帧。
    #[error("frame has {actual} trailing bytes")]
    TrailingBytes { actual: usize },
    /// 输入尚未包含完整的长度头或帧负载。
    #[error("frame is truncated: expected {expected} bytes but received {actual}")]
    Truncated { actual: usize, expected: usize },
}
