#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredExtensionInstall {
    pub(crate) id: String,
    pub(crate) core_id: String,
    pub(crate) instance_id: String,
    pub(crate) kind: String,
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) source: String,
    pub(crate) project_id: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) installed_at: String,
}

impl StoredExtensionInstall {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn core_id(&self) -> &str {
        &self.core_id
    }

    #[must_use]
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn project_id(&self) -> Option<&str> {
        self.project_id.as_deref()
    }

    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    #[must_use]
    pub fn installed_at(&self) -> &str {
        &self.installed_at
    }
}
