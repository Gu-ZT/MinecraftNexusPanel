use std::fmt;

use crate::ConfigError;

#[derive(Clone, Eq, PartialEq)]
pub struct InitialAdminConfig {
    username: String,
    password: String,
}

impl InitialAdminConfig {
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

    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

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
