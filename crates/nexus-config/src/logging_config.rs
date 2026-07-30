use crate::ConfigError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoggingConfig {
    filter: String,
}

impl LoggingConfig {
    pub fn new(filter: String) -> Result<Self, ConfigError> {
        if filter.trim().is_empty() {
            return Err(ConfigError::EmptyLogFilter);
        }

        Ok(Self { filter })
    }

    #[must_use]
    pub fn filter(&self) -> &str {
        &self.filter
    }
}
