use std::fmt;

use crate::ConfigError;

/// 首次初始化空数据库时使用的管理员凭据。
///
/// 构造时会去除用户名首尾空白并检查长度；密码按字节长度检查，且 `Debug`
/// 实现永远不会输出密码正文。
#[derive(Clone, Eq, PartialEq)]
pub struct InitialAdminConfig {
    username: String,
    password: String,
}

impl InitialAdminConfig {
    /// 校验并创建初始管理员凭据。
    pub fn new(username: String, password: String) -> Result<Self, ConfigError> {
        let username = username.trim().to_owned();
        if username.is_empty() || username.chars().count() > 64 {
            return Err(ConfigError::InvalidInitialAdminUsername);
        }
        if !(12..=1024).contains(&password.len()) {
            return Err(ConfigError::WeakInitialAdminPassword);
        }

        Ok(Self { username, password })
    }

    /// 返回规范化后的管理员用户名。
    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    /// 返回初始管理员密码。
    ///
    /// 调用方只应在创建管理员时短暂使用，不应将返回值写入日志或持久化配置。
    #[must_use]
    pub fn password(&self) -> &str {
        &self.password
    }
}

impl fmt::Debug for InitialAdminConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InitialAdminConfig")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::InitialAdminConfig;

    #[test]
    fn redacts_the_password_from_debug_output() {
        let config = InitialAdminConfig::new(
            "admin".to_owned(),
            "correct horse battery staple".to_owned(),
        )
        .expect("initial administrator credentials are valid");

        let debug_output = format!("{config:?}");
        assert!(debug_output.contains("admin"));
        assert!(debug_output.contains("[REDACTED]"));
        assert!(!debug_output.contains("correct horse battery staple"));
    }
}
