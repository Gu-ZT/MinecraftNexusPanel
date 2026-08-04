use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtensionSourceError {
    #[error("failed to create the extension source HTTP client")]
    Client(#[source] reqwest::Error),
    #[error("extension source response is invalid")]
    InvalidResponse,
    #[error("extension source request parameters are invalid")]
    InvalidRequest,
    #[error("extension source request failed")]
    Request(#[source] reqwest::Error),
    #[error("extension source response exceeds {maximum_bytes} bytes")]
    ResponseTooLarge { maximum_bytes: usize },
    #[error("extension version {version_id} was not found in project {project_id}")]
    VersionNotFound {
        project_id: String,
        version_id: String,
    },
    #[error("project {project_id} has no compatible version")]
    NoCompatibleVersion { project_id: String },
    #[error("project {project_id} version {version_id} has no verified artifact")]
    NoArtifact {
        project_id: String,
        version_id: String,
    },
    #[error("dependency project is missing for version {version_id}")]
    MissingDependencyProject { version_id: String },
    #[error("dependency project {project_id} resolves to conflicting versions")]
    DependencyConflict { project_id: String },
    #[error("dependency graph exceeds {maximum_nodes} projects")]
    DependencyGraphTooLarge { maximum_nodes: usize },
}
