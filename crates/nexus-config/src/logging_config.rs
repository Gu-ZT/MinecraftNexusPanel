use crate::ConfigError;

/// 进程日志过滤器配置。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoggingConfig {
    filter: String,
}

impl LoggingConfig {
    /// 校验并创建日志过滤配置。
    pub fn new(filter: String) -> Result<Self, ConfigError> {
        if filter.trim().is_empty() {
            return Err(ConfigError::EmptyLogFilter);
        }

        Ok(Self { filter })
    }

    /// 返回 tracing 过滤器指令文本。
    #[must_use]
    pub fn filter(&self) -> &str {
        &self.filter
    }
}
