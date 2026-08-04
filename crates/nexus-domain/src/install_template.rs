use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionKind;
use crate::InstallRuntimeRequirement;
use crate::InstallTemplateExtensionLayout;
use crate::InstallTemplateFamily;
use crate::InstanceKind;
use crate::ProxyTopology;
use crate::VersionMetadataProvider;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallTemplate {
    id: String,
    name: String,
    instance_kind: InstanceKind,
    family: InstallTemplateFamily,
    required_runtime: InstallRuntimeRequirement,
    proxy_topology: ProxyTopology,
    extension_layouts: Vec<InstallTemplateExtensionLayout>,
    metadata_providers: Vec<VersionMetadataProvider>,
}

impl InstallTemplate {
    #[must_use]
    pub fn new(
        id: String,
        name: String,
        instance_kind: InstanceKind,
        family: InstallTemplateFamily,
        required_runtime: InstallRuntimeRequirement,
        proxy_topology: ProxyTopology,
        metadata_providers: Vec<VersionMetadataProvider>,
    ) -> Self {
        Self {
            id,
            name,
            instance_kind,
            family,
            required_runtime,
            proxy_topology,
            extension_layouts: Vec::new(),
            metadata_providers,
        }
    }

    #[must_use]
    pub fn with_extension_layouts(
        mut self,
        extension_layouts: Vec<InstallTemplateExtensionLayout>,
    ) -> Self {
        self.extension_layouts = extension_layouts;
        self
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn instance_kind(&self) -> InstanceKind {
        self.instance_kind
    }

    #[must_use]
    pub const fn family(&self) -> InstallTemplateFamily {
        self.family
    }

    #[must_use]
    pub const fn required_runtime(&self) -> InstallRuntimeRequirement {
        self.required_runtime
    }

    #[must_use]
    pub const fn proxy_topology(&self) -> ProxyTopology {
        self.proxy_topology
    }

    #[must_use]
    pub fn extension_layouts(&self) -> &[InstallTemplateExtensionLayout] {
        &self.extension_layouts
    }

    #[must_use]
    pub fn extension_directories(&self, kind: ExtensionKind) -> Vec<&str> {
        self.extension_layouts
            .iter()
            .filter(|layout| layout.kind() == kind)
            .flat_map(|layout| layout.directories().iter().map(String::as_str))
            .collect()
    }

    #[must_use]
    pub fn metadata_providers(&self) -> &[VersionMetadataProvider] {
        &self.metadata_providers
    }
}

#[cfg(test)]
mod tests {
    use super::InstallTemplate;
    use crate::ExtensionKind;
    use crate::InstallRuntimeRequirement;
    use crate::InstallTemplateExtensionLayout;
    use crate::InstallTemplateFamily;
    use crate::InstanceKind;
    use crate::ProxyTopology;

    #[test]
    fn resolves_directories_by_extension_kind() {
        let template = InstallTemplate::new(
            "hybrid".to_owned(),
            "Hybrid".to_owned(),
            InstanceKind::Mohist,
            InstallTemplateFamily::JavaServer,
            InstallRuntimeRequirement::Java,
            ProxyTopology::None,
            Vec::new(),
        )
        .with_extension_layouts(vec![
            InstallTemplateExtensionLayout::new(
                ExtensionKind::Plugin,
                vec!["plugins".to_owned(), "extra-plugins".to_owned()],
            ),
            InstallTemplateExtensionLayout::new(ExtensionKind::Mod, vec!["mods".to_owned()]),
            InstallTemplateExtensionLayout::new(ExtensionKind::Plugin, vec!["mods".to_owned()]),
        ]);

        assert_eq!(
            template.extension_directories(ExtensionKind::Plugin),
            vec!["plugins", "extra-plugins", "mods"]
        );
        assert_eq!(
            template.extension_directories(ExtensionKind::Mod),
            vec!["mods"]
        );
    }
}
