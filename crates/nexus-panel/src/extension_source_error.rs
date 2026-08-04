use thiserror::Error;

/// 扩展来源查询、依赖解析和工件下载错误。
#[derive(Debug, Error)]
pub enum ExtensionSourceError {
    /// 扩展来源 HTTP 客户端初始化失败。
    #[error("failed to create the extension source HTTP client")]
    Client(#[source] reqwest::Error),
    /// 来源响应不是预期的数据结构。
    #[error("extension source response is invalid")]
    InvalidResponse,
    /// 查询参数不符合来源 API 或领域约束。
    #[error("extension source request parameters are invalid")]
    InvalidRequest,
    /// 来源 HTTP 请求或状态码处理失败。
    #[error("extension source request failed")]
    Request(#[source] reqwest::Error),
    /// 元数据响应超过允许大小。
    #[error("extension source response exceeds {maximum_bytes} bytes")]
    ResponseTooLarge { maximum_bytes: usize },
    /// 工件不是允许的 HTTPS 来源 URL。
    #[error("extension artifact URL is not a valid HTTPS URL")]
    InvalidArtifactUrl,
    /// 工件超过允许下载大小。
    #[error("extension artifact exceeds {maximum_bytes} bytes")]
    ArtifactTooLarge { maximum_bytes: u64 },
    /// 项目中找不到请求版本。
    #[error("extension version {version_id} was not found in project {project_id}")]
    VersionNotFound {
        project_id: String,
        version_id: String,
    },
    /// 项目没有满足筛选条件的版本。
    #[error("project {project_id} has no compatible version")]
    NoCompatibleVersion { project_id: String },
    /// 版本没有可验证下载工件。
    #[error("project {project_id} version {version_id} has no verified artifact")]
    NoArtifact {
        project_id: String,
        version_id: String,
    },
    /// 依赖声明缺少项目标识。
    #[error("dependency project is missing for version {version_id}")]
    MissingDependencyProject { version_id: String },
    /// 同一依赖项目被解析为冲突版本。
    #[error("dependency project {project_id} resolves to conflicting versions")]
    DependencyConflict { project_id: String },
    /// 依赖图节点数超过上限。
    #[error("dependency graph exceeds {maximum_nodes} projects")]
    DependencyGraphTooLarge { maximum_nodes: usize },
}
