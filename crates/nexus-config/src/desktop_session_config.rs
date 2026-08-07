use std::fmt;

use crate::ConfigError;

/// Desktop 本地会话引导使用的用户名和设备秘密。
///
/// 该配置只应由 Tauri 托管的 loopback sidecar 注入。设备秘密不会下发给 WebView，
/// Panel 仅用它为本机 Desktop 签发标准原生会话。
#[derive(Clone, Eq, PartialEq)]
pub struct DesktopSessionConfig {
    username: String,
    secret: String,
}

impl DesktopSessionConfig {
    /// 校验并创建 Desktop 会话引导配置。
    pub fn new(username: String, secret: String) -> Result<Self, ConfigError> {
        let username = username.trim().to_owned();
        if username.is_empty() || username.chars().count() > 64 {
            return Err(ConfigError::InvalidDesktopSessionUsername);
        }
        if !(32..=1024).contains(&secret.len()) {
            return Err(ConfigError::WeakDesktopSessionSecret);
        }

        Ok(Self { username, secret })
    }

    /// 返回需要建立原生会话的本地用户名。
    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    /// 返回仅供 loopback 引导请求校验的设备秘密。
    #[must_use]
    pub fn secret(&self) -> &str {
        &self.secret
    }
}

impl fmt::Debug for DesktopSessionConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DesktopSessionConfig")
            .field("username", &self.username)
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::DesktopSessionConfig;

    #[test]
    fn redacts_the_device_secret_from_debug_output() {
        let config = DesktopSessionConfig::new("admin".to_owned(), "x".repeat(32))
            .expect("Desktop session configuration is valid");

        let debug_output = format!("{config:?}");
        assert!(debug_output.contains("admin"));
        assert!(debug_output.contains("[REDACTED]"));
        assert!(!debug_output.contains("xxxxxxxx"));
    }
}
