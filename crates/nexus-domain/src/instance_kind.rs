use serde::Deserialize;
use serde::Serialize;

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
