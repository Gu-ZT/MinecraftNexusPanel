use nexus_domain::InstallTemplate;
use nexus_domain::InstanceKind;
use nexus_domain::RuntimeKind;
use nexus_domain::VersionMetadataProvider;

pub(crate) fn install_templates() -> Vec<InstallTemplate> {
    vec![
        InstallTemplate::new(
            "vanilla".to_owned(),
            "Vanilla".to_owned(),
            InstanceKind::Vanilla,
            RuntimeKind::Java,
            vec![VersionMetadataProvider::new(
                "mojang-version-manifest".to_owned(),
                "Mojang version manifest".to_owned(),
                "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json".to_owned(),
            )],
        ),
        InstallTemplate::new(
            "paper".to_owned(),
            "Paper".to_owned(),
            InstanceKind::Paper,
            RuntimeKind::Java,
            vec![VersionMetadataProvider::new(
                "paper-project-api".to_owned(),
                "Paper project API".to_owned(),
                "https://api.papermc.io/v2/projects/paper".to_owned(),
            )],
        ),
        InstallTemplate::new(
            "velocity".to_owned(),
            "Velocity".to_owned(),
            InstanceKind::Velocity,
            RuntimeKind::Java,
            vec![VersionMetadataProvider::new(
                "velocity-project-api".to_owned(),
                "Velocity project API".to_owned(),
                "https://api.papermc.io/v2/projects/velocity".to_owned(),
            )],
        ),
        InstallTemplate::new(
            "fabric".to_owned(),
            "Fabric".to_owned(),
            InstanceKind::Fabric,
            RuntimeKind::Java,
            vec![
                VersionMetadataProvider::new(
                    "fabric-game-versions".to_owned(),
                    "Fabric game versions".to_owned(),
                    "https://meta.fabricmc.net/v2/versions/game".to_owned(),
                ),
                VersionMetadataProvider::new(
                    "fabric-loader-versions".to_owned(),
                    "Fabric loader versions".to_owned(),
                    "https://meta.fabricmc.net/v2/versions/loader".to_owned(),
                ),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::install_templates;
    use nexus_domain::InstanceKind;

    #[test]
    fn provides_the_initial_server_template_catalog() {
        let templates = install_templates();

        assert_eq!(templates.len(), 4);
        assert_eq!(templates[0].instance_kind(), InstanceKind::Vanilla);
        assert_eq!(templates[1].instance_kind(), InstanceKind::Paper);
        assert_eq!(templates[2].instance_kind(), InstanceKind::Velocity);
        assert_eq!(templates[3].instance_kind(), InstanceKind::Fabric);
        assert!(
            templates
                .iter()
                .all(|template| !template.metadata_providers().is_empty())
        );
    }
}
