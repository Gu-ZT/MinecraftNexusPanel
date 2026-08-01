use getrandom::Error as RandomError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecretCipherError {
    #[error("Core secret envelope could not be authenticated")]
    Authentication,
    #[error("Core secret envelope has an unsupported or malformed format")]
    InvalidEnvelope,
    #[error("failed to generate a Core secret nonce")]
    Random(#[from] RandomError),
}
