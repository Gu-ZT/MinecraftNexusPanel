use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum InstanceIdError {
    #[error(
        "instance ID must start with an ASCII letter or digit and contain at most 64 ASCII letters, digits, dots, underscores, or hyphens"
    )]
    InvalidFormat,
}
