use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ProtocolVersionError {
    #[error("protocol major version mismatch: local {local}, remote {remote}")]
    MajorMismatch { local: u16, remote: u16 },
}
