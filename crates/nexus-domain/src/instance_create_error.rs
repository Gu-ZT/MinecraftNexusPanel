use thiserror::Error;

/// 创建实例时的输入校验错误。
#[derive(Debug, Error, Eq, PartialEq)]
pub enum InstanceCreateError {
    /// 实例目录不是规范化的相对路径。
    #[error("instance directory must be a normalized relative path")]
    InvalidDirectory,
    /// 启动配置包含空值、越界值或受保护的环境变量。
    #[error("launch configuration is invalid")]
    InvalidLaunch,
    /// 名称为空、过长或包含禁止字符。
    #[error("instance name must contain between 1 and 128 characters")]
    InvalidName,
}
