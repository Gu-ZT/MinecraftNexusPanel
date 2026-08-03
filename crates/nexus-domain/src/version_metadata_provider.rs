use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionMetadataProvider {
    id: String,
    name: String,
    url: String,
}

impl VersionMetadataProvider {
    #[must_use]
    pub fn new(id: String, name: String, url: String) -> Self {
        Self { id, name, url }
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
    pub fn url(&self) -> &str {
        &self.url
    }
}
