use thiserror::Error;

/// JSON 消息编解码错误。
#[derive(Debug, Error)]
pub enum MessageError {
    /// 消息不是有效 JSON，或序列化时无法生成 JSON。
    #[error("message is not valid JSON: {0}")]
    InvalidJson(serde_json::Error),
    /// 明文 JSON 超过协议规定的最大长度。
    #[error("message length {actual} exceeds the maximum of {maximum} bytes")]
    MessageTooLarge {
        /// 输入 JSON 消息的实际字节数。
        actual: usize,
        /// 协议允许的单条 JSON 消息最大字节数。
        maximum: usize,
    },
}
