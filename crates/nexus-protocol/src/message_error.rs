use thiserror::Error;

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("message is not valid JSON: {0}")]
    InvalidJson(serde_json::Error),
    #[error("message length {actual} exceeds the maximum of {maximum} bytes")]
    MessageTooLarge { actual: usize, maximum: usize },
}
