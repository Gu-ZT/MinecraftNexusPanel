//! 服务端一键搭建模板的领域描述。

use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionKind;
use crate::InstallRuntimeRequirement;
use crate::InstallTemplateExtensionLayout;
use crate::InstallTemplateFamily;
use crate::InstanceKind;
use crate::ProxyTopology;
use crate::VersionMetadataProvider;

/// 描述一个服务端类型的运行时、拓扑、扩展目录和版本来源。
///
/// 模板目录是安装能力的边界，不等同于“所有版本均已验证可安装”。
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
    /// 创建一个没有扩展目录的安装模板。
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

    /// 追加或替换该模板的扩展目录声明。
    #[must_use]
    pub fn with_extension_layouts(
        mut self,
        extension_layouts: Vec<InstallTemplateExtensionLayout>,
    ) -> Self {
        self.extension_layouts = extension_layouts;
        self
    }

    /// 返回模板 ID。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回面向用户显示的模板名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回模板对应的权威实例类型。
    #[must_use]
    pub const fn instance_kind(&self) -> InstanceKind {
        self.instance_kind
    }

    /// 返回 Java/基岩服务端或代理家族。
    #[must_use]
    pub const fn family(&self) -> InstallTemplateFamily {
        self.family
    }

    /// 返回安装和启动所需运行时。
    #[must_use]
    pub const fn required_runtime(&self) -> InstallRuntimeRequirement {
        self.required_runtime
    }

    /// 返回模板声明的代理拓扑。
    #[must_use]
    pub const fn proxy_topology(&self) -> ProxyTopology {
        self.proxy_topology
    }

    /// 返回完整扩展布局声明。
    #[must_use]
    pub fn extension_layouts(&self) -> &[InstallTemplateExtensionLayout] {
        &self.extension_layouts
    }

    /// 按扩展种类展开所有声明目录，保留声明顺序和重复目录。
    #[must_use]
    pub fn extension_directories(&self, kind: ExtensionKind) -> Vec<&str> {
        self.extension_layouts
            .iter()
            .filter(|layout| layout.kind() == kind)
            .flat_map(|layout| layout.directories().iter().map(String::as_str))
            .collect()
    }

    /// 返回该模板配置的版本元数据提供方。
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
