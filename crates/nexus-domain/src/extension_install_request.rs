use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionPlanRequest;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionInstallRequest {
    #[serde(flatten)]
    plan: ExtensionPlanRequest,
    directory: Option<String>,
}

impl ExtensionInstallRequest {
    #[must_use]
    pub const fn plan(&self) -> &ExtensionPlanRequest {
        &self.plan
    }

    #[must_use]
    pub fn directory(&self) -> Option<&str> {
        self.directory.as_deref()
    }
}
