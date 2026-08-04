use serde::Deserialize;
use serde::Serialize;

use crate::BedrockManagementKind;
use crate::BedrockManagementProfile;
use crate::BedrockTransport;
use crate::ExtensionKind;
use crate::ProxyTopology;

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
