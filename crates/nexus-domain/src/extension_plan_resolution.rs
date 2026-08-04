use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionKind;
use crate::ExtensionPlanItem;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionPlanResolution {
    template_id: String,
    kind: ExtensionKind,
    minecraft_version: String,
    loader: Option<String>,
    items: Vec<ExtensionPlanItem>,
}

impl ExtensionPlanResolution {
    #[must_use]
    pub fn new(
        template_id: String,
        kind: ExtensionKind,
        minecraft_version: String,
        loader: Option<String>,
        items: Vec<ExtensionPlanItem>,
    ) -> Self {
        Self {
            template_id,
            kind,
            minecraft_version,
            loader,
            items,
        }
    }

    #[must_use]
    pub fn template_id(&self) -> &str {
        &self.template_id
    }

    #[must_use]
    pub const fn kind(&self) -> ExtensionKind {
        self.kind
    }

    #[must_use]
    pub fn minecraft_version(&self) -> &str {
        &self.minecraft_version
    }

    #[must_use]
    pub fn loader(&self) -> Option<&str> {
        self.loader.as_deref()
    }

    #[must_use]
    pub fn items(&self) -> &[ExtensionPlanItem] {
        &self.items
    }
}
