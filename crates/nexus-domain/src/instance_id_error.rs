//! 实例标识符校验错误。

use thiserror::Error;

/// 实例 ID 不符合资源路径安全约束。
#[derive(Debug, Error, Eq, PartialEq)]
pub enum InstanceIdError {
    #[error(
        "instance ID must start with an ASCII letter or digit and contain at most 64 ASCII letters, digits, dots, underscores, or hyphens"
    )]
    InvalidFormat,
}
