#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewExtensionInstall {
    pub id: String,
    pub core_id: String,
    pub instance_id: String,
    pub kind: String,
    pub path: String,
    pub sha256: String,
    pub source: String,
    pub project_id: Option<String>,
    pub version: Option<String>,
    pub installed_at: String,
}
