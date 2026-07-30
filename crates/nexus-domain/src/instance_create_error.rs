use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum InstanceCreateError {
    #[error("instance directory must be a normalized relative path")]
    InvalidDirectory,
    #[error("launch configuration is invalid")]
    InvalidLaunch,
    #[error("instance name must contain between 1 and 128 characters")]
    InvalidName,
}
