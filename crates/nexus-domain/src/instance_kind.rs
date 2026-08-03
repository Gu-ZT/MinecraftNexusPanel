use serde::Deserialize;
use serde::Serialize;

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
}
