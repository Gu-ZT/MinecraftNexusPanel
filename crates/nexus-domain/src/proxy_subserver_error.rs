use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ProxySubserverError {
    #[error("proxy subserver ID is invalid")]
    InvalidId,
    #[error("proxy subserver name is invalid")]
    InvalidName,
    #[error("proxy subserver host is invalid")]
    InvalidHost,
    #[error("proxy subserver port is invalid")]
    InvalidPort,
}
