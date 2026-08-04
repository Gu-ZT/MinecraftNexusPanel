use std::io;

use thiserror::Error;

use crate::FrameError;
use crate::MessageError;

/// Noise 应用会话建立或消息传输错误。
#[derive(Debug, Error)]
pub enum SessionError {
    /// 长度前缀帧格式不合法。
    #[error(transparent)]
    Frame(#[from] FrameError),
    /// 底层异步流读写失败。
    #[error("Core session I/O failed")]
    Io(#[from] io::Error),
    /// 应用消息 JSON 编解码失败。
    #[error(transparent)]
    Message(#[from] MessageError),
    /// Noise 参数、握手或传输状态失败。
    #[error("Noise session failed")]
    Noise(#[from] snow::Error),
    /// 握手阶段收到非空应用负载。
    #[error("Noise handshake payload must be empty")]
    UnexpectedHandshakePayload,
}
