use std::io;

use thiserror::Error;

use crate::FrameError;
use crate::MessageError;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error("Core session I/O failed")]
    Io(#[from] io::Error),
    #[error(transparent)]
    Message(#[from] MessageError),
    #[error("Noise session failed")]
    Noise(#[from] snow::Error),
    #[error("Noise handshake payload must be empty")]
    UnexpectedHandshakePayload,
}
