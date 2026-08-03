use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum InstanceUpdateError {
    #[error("instance update must change at least one field")]
    Empty,
    #[error("instance directory must be a normalized relative path")]
    InvalidDirectory,
    #[error("instance expiration must use RFC 3339 format")]
    InvalidExpiration,
    #[error("launch configuration is invalid")]
    InvalidLaunch,
    #[error("instance name must contain between 1 and 128 characters")]
    InvalidName,
    #[error("required instance settings cannot be cleared")]
    RequiredFieldCleared,
    #[error("update command is invalid")]
    InvalidUpdateCommand,
}
