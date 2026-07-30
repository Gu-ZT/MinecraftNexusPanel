use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum FrameError {
    #[error("a protocol frame cannot be empty")]
    EmptyFrame,
    #[error("frame length {actual} exceeds the maximum of {maximum} bytes")]
    FrameTooLarge { actual: usize, maximum: usize },
    #[error("frame has {actual} trailing bytes")]
    TrailingBytes { actual: usize },
    #[error("frame is truncated: expected {expected} bytes but received {actual}")]
    Truncated { actual: usize, expected: usize },
}
