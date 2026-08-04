use thiserror::Error;

/// 更新实例配置时的输入校验错误。
#[derive(Debug, Error, Eq, PartialEq)]
pub enum InstanceUpdateError {
    /// 更新没有设置、清空或修改任何字段。
    #[error("instance update must change at least one field")]
    Empty,
    /// 实例目录不是规范化的相对路径。
    #[error("instance directory must be a normalized relative path")]
    InvalidDirectory,
    /// 到期时间不是 RFC 3339 格式。
    #[error("instance expiration must use RFC 3339 format")]
    InvalidExpiration,
    /// 启动配置包含无效值。
    #[error("launch configuration is invalid")]
    InvalidLaunch,
    /// 名称为空、过长或包含禁止字符。
    #[error("instance name must contain between 1 and 128 characters")]
    InvalidName,
    /// 必填实例配置不能被显式清空。
    #[error("required instance settings cannot be cleared")]
    RequiredFieldCleared,
    /// 更新命令为空、过长或包含禁止字符。
    #[error("update command is invalid")]
    InvalidUpdateCommand,
}
