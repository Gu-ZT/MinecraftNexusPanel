use serde::Deserialize;
use serde::Serialize;

/// 描述基岩端扩展版本兼容性应由哪类声明决定。
///
/// 这是模板画像的策略声明，不代表当前已经完成每个版本的兼容性解析。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BedrockExtensionCompatibilityPolicy {
    /// 该画像不提供可管理的插件或扩展种类。
    Unsupported,
    /// 使用插件自身的 manifest/API 声明与目标服务端版本匹配。
    PluginManifest,
}
