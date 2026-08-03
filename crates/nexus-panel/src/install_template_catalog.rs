use nexus_domain::ExtensionKind;
use nexus_domain::InstallRuntimeRequirement;
use nexus_domain::InstallTemplate;
use nexus_domain::InstallTemplateExtensionLayout;
use nexus_domain::InstallTemplateFamily;
use nexus_domain::InstanceKind;
use nexus_domain::ProxyTopology;
use nexus_domain::VersionMetadataProvider;

pub(crate) fn install_templates() -> Vec<InstallTemplate> {
    vec![
        template(
            "vanilla",
            "Vanilla",
            InstanceKind::Vanilla,
            InstallTemplateFamily::JavaServer,
            InstallRuntimeRequirement::Java,
            ProxyTopology::None,
            vec![provider(
                "mojang-version-manifest",
                "Mojang version manifest",
                "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json",
            )],
        ),
        template(
            "paper",
            "Paper",
            InstanceKind::Paper,
            InstallTemplateFamily::JavaServer,
            InstallRuntimeRequirement::Java,
            ProxyTopology::None,
            vec![provider(
                "paper-downloads-service",
                "Paper downloads service",
                "https://fill.papermc.io/v3/projects/paper",
            )],
        )
        .with_extension_layouts(plugin_layout()),
        template(
            "velocity",
            "Velocity",
            InstanceKind::Velocity,
            InstallTemplateFamily::JavaProxy,
            InstallRuntimeRequirement::Java,
            ProxyTopology::OneToMany,
            vec![provider(
                "velocity-downloads-service",
                "Velocity downloads service",
                "https://fill.papermc.io/v3/projects/velocity",
            )],
        )
        .with_extension_layouts(plugin_layout()),
        template(
            "fabric",
            "Fabric",
            InstanceKind::Fabric,
            InstallTemplateFamily::JavaServer,
            InstallRuntimeRequirement::Java,
            ProxyTopology::None,
            vec![
                provider(
                    "fabric-game-versions",
                    "Fabric game versions",
                    "https://meta.fabricmc.net/v2/versions/game",
                ),
                provider(
                    "fabric-loader-versions",
                    "Fabric loader versions",
                    "https://meta.fabricmc.net/v2/versions/loader",
                ),
            ],
        )
        .with_extension_layouts(mod_layout()),
        template(
            "neoforge",
            "NeoForge",
            InstanceKind::NeoForge,
            InstallTemplateFamily::JavaServer,
            InstallRuntimeRequirement::Java,
            ProxyTopology::None,
            vec![provider(
                "neoforge-maven-service",
                "NeoForge Maven service",
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml",
            )],
        )
        .with_extension_layouts(mod_layout()),
        template(
            "forge",
            "Forge",
            InstanceKind::Forge,
            InstallTemplateFamily::JavaServer,
            InstallRuntimeRequirement::Java,
            ProxyTopology::None,
            vec![provider(
                "forge-version-service",
                "Forge version service",
                "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json",
            )],
        )
        .with_extension_layouts(mod_layout()),
        java_server("bukkit", "Bukkit", InstanceKind::Bukkit, plugin_layout()),
        java_server("spigot", "Spigot", InstanceKind::Spigot, plugin_layout()),
        template(
            "purpur",
            "Purpur",
            InstanceKind::Purpur,
            InstallTemplateFamily::JavaServer,
            InstallRuntimeRequirement::Java,
            ProxyTopology::None,
            vec![provider(
                "purpur-version-service",
                "Purpur version service",
                "https://api.purpurmc.org/v2/purpur",
            )],
        )
        .with_extension_layouts(plugin_layout()),
        template(
            "pufferfish",
            "Pufferfish",
            InstanceKind::Pufferfish,
            InstallTemplateFamily::JavaServer,
            InstallRuntimeRequirement::Java,
            ProxyTopology::None,
            vec![
                provider(
                    "pufferfish-1.21-jenkins-service",
                    "Pufferfish 1.21 Jenkins service",
                    "https://ci.pufferfish.host/job/Pufferfish-1.21/api/json?tree=builds[number,url,result,artifacts[relativePath]]",
                ),
                provider(
                    "pufferfish-1.20-jenkins-service",
                    "Pufferfish 1.20 Jenkins service",
                    "https://ci.pufferfish.host/job/Pufferfish-1.20/api/json?tree=builds[number,url,result,artifacts[relativePath]]",
                ),
                provider(
                    "pufferfish-1.19-jenkins-service",
                    "Pufferfish 1.19 Jenkins service",
                    "https://ci.pufferfish.host/job/Pufferfish-1.19/api/json?tree=builds[number,url,result,artifacts[relativePath]]",
                ),
                provider(
                    "pufferfish-1.18-jenkins-service",
                    "Pufferfish 1.18 Jenkins service",
                    "https://ci.pufferfish.host/job/Pufferfish-1.18/api/json?tree=builds[number,url,result,artifacts[relativePath]]",
                ),
                provider(
                    "pufferfish-1.17-jenkins-service",
                    "Pufferfish 1.17 Jenkins service",
                    "https://ci.pufferfish.host/job/Pufferfish-1.17/api/json?tree=builds[number,url,result,artifacts[relativePath]]",
                ),
            ],
        )
        .with_extension_layouts(plugin_layout()),
        template(
            "folia",
            "Folia",
            InstanceKind::Folia,
            InstallTemplateFamily::JavaServer,
            InstallRuntimeRequirement::Java,
            ProxyTopology::None,
            vec![provider(
                "folia-downloads-service",
                "Folia downloads service",
                "https://fill.papermc.io/v3/projects/folia",
            )],
        )
        .with_extension_layouts(plugin_layout()),
        template(
            "leaf",
            "Leaf",
            InstanceKind::Leaf,
            InstallTemplateFamily::JavaServer,
            InstallRuntimeRequirement::Java,
            ProxyTopology::None,
            vec![provider(
                "leaf-github-releases",
                "Leaf GitHub releases",
                "https://api.github.com/repos/Winds-Studio/Leaf/releases?per_page=100",
            )],
        )
        .with_extension_layouts(plugin_layout()),
        java_server("mohist", "Mohist", InstanceKind::Mohist, hybrid_layout()),
        java_server("magma", "Magma", InstanceKind::Magma, hybrid_layout()),
        java_server("sponge", "Sponge", InstanceKind::Sponge, sponge_layout()),
        java_server(
            "arclight",
            "Arclight",
            InstanceKind::Arclight,
            hybrid_layout(),
        ),
        java_server("youer", "Youer", InstanceKind::Youer, hybrid_layout()),
        java_server(
            "async-youer",
            "AsyncYouer",
            InstanceKind::AsyncYouer,
            hybrid_layout(),
        ),
        java_server("silkard", "Silkard", InstanceKind::Silkard, hybrid_layout()),
        java_server(
            "catserver",
            "CatServer",
            InstanceKind::CatServer,
            hybrid_layout(),
        ),
        java_server("lingshu", "Lingshu", InstanceKind::Lingshu, hybrid_layout()),
        java_proxy(
            "waterfall",
            "Waterfall",
            InstanceKind::Waterfall,
            vec![provider(
                "waterfall-downloads-service",
                "Waterfall downloads service",
                "https://fill.papermc.io/v3/projects/waterfall",
            )],
        ),
        java_proxy(
            "bungeecord",
            "BungeeCord",
            InstanceKind::BungeeCord,
            vec![provider(
                "bungeecord-jenkins-service",
                "BungeeCord Jenkins service",
                "https://hub.spigotmc.org/jenkins/job/BungeeCord/api/json?tree=builds[number,url,result]",
            )],
        ),
        java_proxy(
            "lightfall",
            "Lightfall",
            InstanceKind::Lightfall,
            Vec::new(),
        ),
        template(
            "geyser",
            "Geyser",
            InstanceKind::Geyser,
            InstallTemplateFamily::BedrockProxy,
            InstallRuntimeRequirement::Java,
            ProxyTopology::OneToOne,
            vec![provider(
                "geyser-version-service",
                "Geyser version service",
                "https://download.geysermc.org/v2/projects/geyser",
            )],
        ),
        template(
            "bedrock-dedicated-server",
            "Bedrock Dedicated Server",
            InstanceKind::BedrockDedicatedServer,
            InstallTemplateFamily::BedrockServer,
            InstallRuntimeRequirement::Native,
            ProxyTopology::None,
            Vec::new(),
        ),
        template(
            "pocketmine-mp",
            "PocketMine-MP",
            InstanceKind::PocketMineMp,
            InstallTemplateFamily::BedrockServer,
            InstallRuntimeRequirement::Php,
            ProxyTopology::None,
            Vec::new(),
        )
        .with_extension_layouts(plugin_layout()),
        template(
            "nukkit",
            "Nukkit",
            InstanceKind::Nukkit,
            InstallTemplateFamily::BedrockServer,
            InstallRuntimeRequirement::Java,
            ProxyTopology::None,
            Vec::new(),
        )
        .with_extension_layouts(plugin_layout()),
        template(
            "cloudburst-nukkit",
            "Cloudburst Nukkit",
            InstanceKind::CloudburstNukkit,
            InstallTemplateFamily::BedrockServer,
            InstallRuntimeRequirement::Java,
            ProxyTopology::None,
            Vec::new(),
        )
        .with_extension_layouts(plugin_layout()),
    ]
}

pub(crate) fn install_template(id: &str) -> Option<InstallTemplate> {
    install_templates()
        .into_iter()
        .find(|template| template.id() == id)
}

fn java_server(
    id: &str,
    name: &str,
    kind: InstanceKind,
    extension_layouts: Vec<InstallTemplateExtensionLayout>,
) -> InstallTemplate {
    template(
        id,
        name,
        kind,
        InstallTemplateFamily::JavaServer,
        InstallRuntimeRequirement::Java,
        ProxyTopology::None,
        Vec::new(),
    )
    .with_extension_layouts(extension_layouts)
}

fn java_proxy(
    id: &str,
    name: &str,
    kind: InstanceKind,
    metadata_providers: Vec<VersionMetadataProvider>,
) -> InstallTemplate {
    template(
        id,
        name,
        kind,
        InstallTemplateFamily::JavaProxy,
        InstallRuntimeRequirement::Java,
        ProxyTopology::OneToMany,
        metadata_providers,
    )
    .with_extension_layouts(plugin_layout())
}

fn template(
    id: &str,
    name: &str,
    instance_kind: InstanceKind,
    family: InstallTemplateFamily,
    required_runtime: InstallRuntimeRequirement,
    proxy_topology: ProxyTopology,
    metadata_providers: Vec<VersionMetadataProvider>,
) -> InstallTemplate {
    InstallTemplate::new(
        id.to_owned(),
        name.to_owned(),
        instance_kind,
        family,
        required_runtime,
        proxy_topology,
        metadata_providers,
    )
}

fn provider(id: &str, name: &str, url: &str) -> VersionMetadataProvider {
    VersionMetadataProvider::new(id.to_owned(), name.to_owned(), url.to_owned())
}

fn plugin_layout() -> Vec<InstallTemplateExtensionLayout> {
    vec![InstallTemplateExtensionLayout::new(
        ExtensionKind::Plugin,
        vec!["plugins".to_owned()],
    )]
}

fn mod_layout() -> Vec<InstallTemplateExtensionLayout> {
    vec![InstallTemplateExtensionLayout::new(
        ExtensionKind::Mod,
        vec!["mods".to_owned()],
    )]
}

fn hybrid_layout() -> Vec<InstallTemplateExtensionLayout> {
    let mut layouts = plugin_layout();
    layouts.extend(mod_layout());
    layouts
}

fn sponge_layout() -> Vec<InstallTemplateExtensionLayout> {
    vec![
        InstallTemplateExtensionLayout::new(ExtensionKind::Plugin, vec!["mods".to_owned()]),
        InstallTemplateExtensionLayout::new(ExtensionKind::Mod, vec!["mods".to_owned()]),
    ]
}

#[cfg(test)]
mod tests {
    use super::install_templates;
    use nexus_domain::ExtensionKind;
    use nexus_domain::InstallTemplateFamily;
    use nexus_domain::InstanceKind;
    use nexus_domain::ProxyTopology;

    #[test]
    fn provides_the_supported_server_template_catalog() {
        let templates = install_templates();

        assert_eq!(templates.len(), 29);
        assert_eq!(templates[0].instance_kind(), InstanceKind::Vanilla);
        assert_eq!(templates[3].instance_kind(), InstanceKind::Fabric);
        assert_eq!(templates[0].family(), InstallTemplateFamily::JavaServer);
        assert_eq!(templates[2].proxy_topology(), ProxyTopology::OneToMany);
        let geyser = templates
            .iter()
            .find(|template| template.id() == "geyser")
            .expect("Geyser template exists");
        assert_eq!(geyser.instance_kind(), InstanceKind::Geyser);
        assert_eq!(geyser.family(), InstallTemplateFamily::BedrockProxy);
        assert_eq!(geyser.proxy_topology(), ProxyTopology::OneToOne);
        assert_eq!(templates[25].family(), InstallTemplateFamily::BedrockServer);

        let mohist = templates
            .iter()
            .find(|template| template.id() == "mohist")
            .expect("Mohist template exists");
        assert_eq!(mohist.extension_layouts().len(), 2);
        assert_eq!(mohist.extension_layouts()[0].kind(), ExtensionKind::Plugin);
        assert_eq!(mohist.extension_layouts()[0].directories()[0], "plugins");
        assert_eq!(mohist.extension_layouts()[1].kind(), ExtensionKind::Mod);
        assert_eq!(mohist.extension_layouts()[1].directories()[0], "mods");

        assert_eq!(
            templates[1].metadata_providers()[0].url(),
            "https://fill.papermc.io/v3/projects/paper"
        );
        assert_eq!(
            templates[2].metadata_providers()[0].url(),
            "https://fill.papermc.io/v3/projects/velocity"
        );
        assert_eq!(
            templates
                .iter()
                .find(|template| template.id() == "purpur")
                .expect("Purpur template exists")
                .metadata_providers()[0]
                .url(),
            "https://api.purpurmc.org/v2/purpur"
        );
        assert_eq!(
            templates
                .iter()
                .find(|template| template.id() == "folia")
                .expect("Folia template exists")
                .metadata_providers()[0]
                .url(),
            "https://fill.papermc.io/v3/projects/folia"
        );
        assert_eq!(
            templates
                .iter()
                .find(|template| template.id() == "waterfall")
                .expect("Waterfall template exists")
                .metadata_providers()[0]
                .url(),
            "https://fill.papermc.io/v3/projects/waterfall"
        );
        assert_eq!(
            templates
                .iter()
                .find(|template| template.id() == "forge")
                .expect("Forge template exists")
                .metadata_providers()[0]
                .url(),
            "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json"
        );
        assert_eq!(
            templates
                .iter()
                .find(|template| template.id() == "neoforge")
                .expect("NeoForge template exists")
                .metadata_providers()[0]
                .url(),
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml"
        );
        assert_eq!(
            templates
                .iter()
                .find(|template| template.id() == "pufferfish")
                .expect("Pufferfish template exists")
                .metadata_providers()[0]
                .url(),
            "https://ci.pufferfish.host/job/Pufferfish-1.21/api/json?tree=builds[number,url,result,artifacts[relativePath]]"
        );
        assert_eq!(
            templates
                .iter()
                .find(|template| template.id() == "leaf")
                .expect("Leaf template exists")
                .metadata_providers()[0]
                .url(),
            "https://api.github.com/repos/Winds-Studio/Leaf/releases?per_page=100"
        );
        assert_eq!(
            templates
                .iter()
                .find(|template| template.id() == "bungeecord")
                .expect("BungeeCord template exists")
                .metadata_providers()[0]
                .url(),
            "https://hub.spigotmc.org/jenkins/job/BungeeCord/api/json?tree=builds[number,url,result]"
        );
        assert_eq!(
            templates
                .iter()
                .find(|template| template.id() == "geyser")
                .expect("Geyser template exists")
                .metadata_providers()[0]
                .url(),
            "https://download.geysermc.org/v2/projects/geyser"
        );
        assert!(
            templates[..4]
                .iter()
                .all(|template| !template.metadata_providers().is_empty())
        );
    }
}
