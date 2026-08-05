use serde::Deserialize;
use serde::Serialize;

/// 基岩端管理配置的结构化文件格式。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BedrockConfigurationFormat {
    /// Java properties 风格配置，例如 `server.properties`。
    Properties,
    /// YAML 配置，例如 Geyser 的 `config.yml`。
    Yaml,
    /// 当前画像没有足够信息判断格式。
    Unknown,
}

impl BedrockConfigurationFormat {
    /// 从配置文件扩展名推导格式；未知扩展名保持未知。
    #[must_use]
    pub fn from_path(path: &str) -> Self {
        if path.ends_with(".properties") {
            Self::Properties
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            Self::Yaml
        } else {
            Self::Unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BedrockConfigurationFormat;

    #[test]
    fn infers_supported_bedrock_configuration_formats() {
        assert_eq!(
            BedrockConfigurationFormat::from_path("server.properties"),
            BedrockConfigurationFormat::Properties
        );
        assert_eq!(
            BedrockConfigurationFormat::from_path("config.yml"),
            BedrockConfigurationFormat::Yaml
        );
        assert_eq!(
            BedrockConfigurationFormat::from_path("config.json"),
            BedrockConfigurationFormat::Unknown
        );
    }
}
