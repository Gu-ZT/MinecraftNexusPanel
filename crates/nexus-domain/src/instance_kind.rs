//! Minecraft 实例类型及其专用能力画像。

use serde::Deserialize;
use serde::Serialize;

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
    Vanilla,
    Paper,
    Velocity,
    Fabric,
    NeoForge,
    Forge,
    Bukkit,
    Spigot,
    Purpur,
    Pufferfish,
    Folia,
    Leaf,
    Mohist,
    Magma,
    Sponge,
    Arclight,
    Youer,
    AsyncYouer,
    Silkard,
    CatServer,
    Lingshu,
    Waterfall,
    BungeeCord,
    Lightfall,
    Geyser,
    BedrockDedicatedServer,
    PocketMineMp,
    Nukkit,
    CloudburstNukkit,
    Custom,
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
            .with_extension_directories(extension_directories),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::InstanceKind;
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
        assert_eq!(dedicated.extension_kind(), None);

        let pocketmine = InstanceKind::PocketMineMp
            .bedrock_management_profile()
            .expect("PocketMine-MP has a Bedrock profile");
        assert_eq!(pocketmine.extension_kind(), Some(ExtensionKind::Plugin));
        assert_eq!(pocketmine.extension_directories(), ["plugins"]);
        assert_eq!(pocketmine.default_port(), 19132);

        assert!(InstanceKind::Paper.bedrock_management_profile().is_none());
    }
}
