//! Minecraft 实例类型及其专用能力画像。

use serde::Deserialize;
use serde::Serialize;

use crate::BedrockExtensionCompatibilityPolicy;
use crate::BedrockManagementKind;
use crate::BedrockManagementProfile;
use crate::BedrockTransport;
use crate::ExtensionKind;
use crate::ProxyTopology;

/// Minecraft 服务端、代理端和基岩端的稳定类型词汇。
///
/// 该枚举是 API 和模板目录之间的权威分类；某个类型进入目录不代表
/// 每个版本都已经具备可验证的归档结构和启动配方。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstanceKind {
    /// Java 原版服务端，不预设插件或模组加载器。
    Vanilla,
    /// 以 Bukkit/Spigot 插件生态为重点的高性能 Java 服务端。
    Paper,
    /// Java 一对多反向代理，可管理多个非代理后端实例。
    Velocity,
    /// 以 Fabric 加载器为基础的 Java 模组服务端。
    Fabric,
    /// 以 NeoForge 加载器为基础的 Java 模组服务端。
    NeoForge,
    /// 以 Forge 加载器为基础的 Java 模组服务端。
    Forge,
    /// Bukkit 插件 API 兼容的 Java 插件服务端基线。
    Bukkit,
    /// 以 Spigot 插件生态为重点的 Java 插件服务端。
    Spigot,
    /// 在 Paper 兼容基础上提供额外配置和功能的 Java 插件服务端。
    Purpur,
    /// 面向性能优化的 Java 插件服务端，兼容常见 Bukkit 插件接口。
    Pufferfish,
    /// 面向区域化调度的 Java 插件服务端，插件需遵守其并发约束。
    Folia,
    /// 面向性能优化的 Java 插件服务端分支，兼容 Bukkit 插件生态。
    Leaf,
    /// 同时支持插件和模组的 Java 混合端；两类扩展必须分别管理。
    Mohist,
    /// 同时支持插件和模组的 Java 混合端；安装目录由模板布局解析。
    Magma,
    /// 可承载 Sponge 扩展的 Java 服务端；扩展种类由模板能力声明。
    Sponge,
    /// 同时支持插件和模组的 Java 混合端；不能按物理目录合并记录。
    Arclight,
    /// 同时支持插件和模组的 Java 混合端；版本配方需单独验证。
    Youer,
    /// 同时支持插件和模组的 Java 混合端；启动和扩展布局可能异步化。
    AsyncYouer,
    /// 同时支持插件和模组的 Java 混合端；插件与模组保持独立管理空间。
    Silkard,
    /// 同时支持插件和模组的 Java 混合端；具体扩展目录由安装模板决定。
    CatServer,
    /// 同时支持插件和模组的 Java 混合端；仅表示目录分类，不代表已验证安装器。
    Lingshu,
    /// Java 一对多反向代理，可管理多个非代理后端实例。
    Waterfall,
    /// Java 一对多反向代理，可管理多个非代理后端实例。
    BungeeCord,
    /// Java 一对多反向代理，可管理多个非代理后端实例。
    Lightfall,
    /// 面向基岩版的一对一反向代理，只能关联一个 Java 后端实例。
    Geyser,
    /// Bedrock Dedicated Server 基岩版服务端，使用基岩专用运维画像。
    BedrockDedicatedServer,
    /// PocketMine-MP 基岩版服务端，支持独立的插件管理。
    PocketMineMp,
    /// Nukkit 基岩版服务端，支持独立的插件管理。
    Nukkit,
    /// Cloudburst Nukkit 基岩版服务端，支持独立的插件管理。
    CloudburstNukkit,
    /// 自定义服务端类型，必须由安装模板补充实际启动和管理能力。
    Custom,
    /// 尚未识别的服务端类型，仅用于兼容旧数据或未知输入。
    Unknown,
}

impl InstanceKind {
    /// 返回该类型允许的代理后端拓扑。
    ///
    /// Java 一对多代理和 Geyser 一对一代理在这里与普通服务端区分，
    /// Core 会据此拒绝不合法的子服务器关系。
    #[must_use]
    pub const fn proxy_topology(self) -> ProxyTopology {
        match self {
            Self::Velocity | Self::Waterfall | Self::BungeeCord | Self::Lightfall => {
                ProxyTopology::OneToMany
            }
            Self::Geyser => ProxyTopology::OneToOne,
            Self::Vanilla
            | Self::Paper
            | Self::Fabric
            | Self::NeoForge
            | Self::Forge
            | Self::Bukkit
            | Self::Spigot
            | Self::Purpur
            | Self::Pufferfish
            | Self::Folia
            | Self::Leaf
            | Self::Mohist
            | Self::Magma
            | Self::Sponge
            | Self::Arclight
            | Self::Youer
            | Self::AsyncYouer
            | Self::Silkard
            | Self::CatServer
            | Self::Lingshu
            | Self::BedrockDedicatedServer
            | Self::PocketMineMp
            | Self::Nukkit
            | Self::CloudburstNukkit
            | Self::Custom
            | Self::Unknown => ProxyTopology::None,
        }
    }

    /// 返回基岩端或 Geyser 的专用运维画像。
    ///
    /// Java 服务端返回 `None`，避免调用方把 `server.properties`、插件目录
    /// 或 Java TCP 探针错误地套用到基岩服务端。
    #[must_use]
    pub fn bedrock_management_profile(self) -> Option<BedrockManagementProfile> {
        let (management_kind, configuration_files, extension_kind, extension_directories) =
            match self {
                Self::BedrockDedicatedServer => (
                    BedrockManagementKind::DedicatedServer,
                    vec!["server.properties".to_owned()],
                    None,
                    Vec::new(),
                ),
                Self::PocketMineMp => (
                    BedrockManagementKind::PocketMine,
                    vec!["server.properties".to_owned()],
                    Some(ExtensionKind::Plugin),
                    vec!["plugins".to_owned()],
                ),
                Self::Nukkit | Self::CloudburstNukkit => (
                    BedrockManagementKind::Nukkit,
                    vec!["server.properties".to_owned()],
                    Some(ExtensionKind::Plugin),
                    vec!["plugins".to_owned()],
                ),
                Self::Geyser => (
                    BedrockManagementKind::Geyser,
                    vec!["config.yml".to_owned()],
                    None,
                    Vec::new(),
                ),
                _ => return None,
            };

        Some(
            BedrockManagementProfile::new(
                management_kind,
                BedrockTransport::RaknetUdp,
                19132,
                configuration_files,
                extension_kind,
            )
            .with_extension_compatibility_policy(if extension_kind.is_some() {
                BedrockExtensionCompatibilityPolicy::PluginManifest
            } else {
                BedrockExtensionCompatibilityPolicy::Unsupported
            })
            .with_extension_directories(extension_directories),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::InstanceKind;
    use crate::BedrockConfigurationFormat;
    use crate::BedrockExtensionCompatibilityPolicy;
    use crate::BedrockManagementKind;
    use crate::ExtensionKind;

    #[test]
    fn exposes_distinct_bedrock_management_profiles() {
        let dedicated = InstanceKind::BedrockDedicatedServer
            .bedrock_management_profile()
            .expect("BDS has a Bedrock profile");
        assert_eq!(
            dedicated.management_kind(),
            BedrockManagementKind::DedicatedServer
        );
        assert_eq!(dedicated.configuration_files(), ["server.properties"]);
        assert_eq!(
            dedicated.configuration_format(),
            BedrockConfigurationFormat::Properties
        );
        assert_eq!(dedicated.extension_kind(), None);
        assert_eq!(
            dedicated.extension_compatibility_policy(),
            BedrockExtensionCompatibilityPolicy::Unsupported
        );

        let pocketmine = InstanceKind::PocketMineMp
            .bedrock_management_profile()
            .expect("PocketMine-MP has a Bedrock profile");
        assert_eq!(pocketmine.extension_kind(), Some(ExtensionKind::Plugin));
        assert_eq!(pocketmine.extension_directories(), ["plugins"]);
        assert_eq!(
            pocketmine.extension_compatibility_policy(),
            BedrockExtensionCompatibilityPolicy::PluginManifest
        );
        assert_eq!(pocketmine.default_port(), 19132);

        let geyser = InstanceKind::Geyser
            .bedrock_management_profile()
            .expect("Geyser has a Bedrock profile");
        assert_eq!(
            geyser.configuration_format(),
            BedrockConfigurationFormat::Yaml
        );
        assert_eq!(
            geyser.extension_compatibility_policy(),
            BedrockExtensionCompatibilityPolicy::Unsupported
        );

        assert!(InstanceKind::Paper.bedrock_management_profile().is_none());
    }
}
