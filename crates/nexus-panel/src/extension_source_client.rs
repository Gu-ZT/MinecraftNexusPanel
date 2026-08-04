//! 扩展来源的 HTTPS 元数据和工件客户端。
//!
//! 当前来源只接受 Modrinth API 与其允许的 CDN 域名；返回的工件在安装前还必须
//! 由调用方按大小和强哈希再次校验。

use std::collections::BTreeMap;
use std::time::Duration;

use nexus_domain::ExtensionArtifact;
use nexus_domain::ExtensionCompatibility;
use nexus_domain::ExtensionDependency;
use nexus_domain::ExtensionKind;
use nexus_domain::ExtensionPlanItem;
use nexus_domain::ExtensionPlanResolution;
use nexus_domain::ExtensionProject;
use nexus_domain::ExtensionSearchResult;
use nexus_domain::ExtensionVersion;
use nexus_domain::ExtensionVersionResult;
use reqwest::Client;
use reqwest::Response;
use reqwest::redirect::Policy;
use rustls::crypto::ring;
use serde_json::Map;
use serde_json::Value;
use serde_json::from_slice;
use serde_json::to_string;
use url::Url;

use crate::extension_source_error::ExtensionSourceError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const SEARCH_TIMEOUT: Duration = Duration::from_secs(30);
const ARTIFACT_TIMEOUT: Duration = Duration::from_secs(300);
const MAXIMUM_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const MAXIMUM_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;
const MAXIMUM_DEPENDENCY_NODES: usize = 64;
const MODRINTH_SEARCH_URL: &str = "https://api.modrinth.com/v2/search";
const MODRINTH_PROJECT_VERSION_URL_PREFIX: &str = "https://api.modrinth.com/v2/project/";
const MODRINTH_SOURCE: &str = "modrinth";

/// 查询扩展项目、版本、依赖和下载工件的内部客户端。
#[derive(Clone)]
pub(crate) struct ExtensionSourceClient {
    client: Client,
}

impl ExtensionSourceClient {
    /// 创建限制重定向、仅允许 HTTPS 的扩展来源客户端。
    pub(crate) fn new() -> Result<Self, ExtensionSourceError> {
        let _ = ring::default_provider().install_default();
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(SEARCH_TIMEOUT)
            .https_only(true)
            .redirect(Policy::none())
            .user_agent("MCNP extension catalog")
            .build()
            .map_err(ExtensionSourceError::Client)?;

        Ok(Self { client })
    }

    /// 按扩展种类、Minecraft 版本和加载器搜索项目。
    pub(crate) async fn search(
        &self,
        query: &str,
        kind: ExtensionKind,
        minecraft_version: Option<&str>,
        loader: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<ExtensionSearchResult, ExtensionSourceError> {
        let expected_project_type = project_type(kind);
        let mut facets = vec![vec![format!("project_type:{expected_project_type}")]];
        if let Some(version) = minecraft_version {
            facets.push(vec![format!("versions:{version}")]);
        }
        if let Some(loader) = loader {
            facets.push(vec![format!("categories:{loader}")]);
        }
        let facets = to_string(&facets).map_err(|_| ExtensionSourceError::InvalidResponse)?;
        let response = self
            .client
            .get(MODRINTH_SEARCH_URL)
            .query(&[("query", query), ("facets", facets.as_str())])
            .query(&[("limit", limit), ("offset", offset)])
            .send()
            .await
            .map_err(ExtensionSourceError::Request)?
            .error_for_status()
            .map_err(ExtensionSourceError::Request)?;
        let bytes = read_response(response).await?;
        let metadata: Value =
            from_slice(&bytes).map_err(|_| ExtensionSourceError::InvalidResponse)?;

        parse_modrinth_search(&metadata, kind, minecraft_version, loader, limit, offset)
    }

    /// 列出项目版本并按版本过滤条件解析来源数据。
    pub(crate) async fn list_versions(
        &self,
        project_id: &str,
        minecraft_version: Option<&str>,
        loader: Option<&str>,
    ) -> Result<ExtensionVersionResult, ExtensionSourceError> {
        if !is_valid_project_id(project_id) {
            return Err(ExtensionSourceError::InvalidRequest);
        }
        let url = format!("{MODRINTH_PROJECT_VERSION_URL_PREFIX}{project_id}/version");
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(ExtensionSourceError::Request)?
            .error_for_status()
            .map_err(ExtensionSourceError::Request)?;
        let bytes = read_response(response).await?;
        let metadata: Value =
            from_slice(&bytes).map_err(|_| ExtensionSourceError::InvalidResponse)?;

        parse_modrinth_versions(&metadata, project_id, minecraft_version, loader)
    }

    /// 解析必需依赖，构建插件或模组安装计划。
    ///
    /// 同一项目的冲突版本和超过节点上限的依赖图会被拒绝。
    pub(crate) async fn resolve_dependencies(
        &self,
        template_id: &str,
        kind: ExtensionKind,
        project_id: &str,
        version_id: &str,
        minecraft_version: &str,
        loader: Option<&str>,
    ) -> Result<ExtensionPlanResolution, ExtensionSourceError> {
        if !is_valid_project_id(project_id) || version_id.is_empty() {
            return Err(ExtensionSourceError::InvalidRequest);
        }

        let mut pending = vec![(project_id.to_owned(), Some(version_id.to_owned()))];
        let mut selected = BTreeMap::new();
        let mut items = Vec::new();
        while let Some((project_id, requested_version_id)) = pending.pop() {
            if let Some(existing_version_id) = selected.get(&project_id) {
                if requested_version_id
                    .as_deref()
                    .is_some_and(|version_id| version_id != existing_version_id)
                {
                    return Err(ExtensionSourceError::DependencyConflict { project_id });
                }
                continue;
            }
            if selected.len() >= MAXIMUM_DEPENDENCY_NODES {
                return Err(ExtensionSourceError::DependencyGraphTooLarge {
                    maximum_nodes: MAXIMUM_DEPENDENCY_NODES,
                });
            }

            let versions = self
                .list_versions(&project_id, Some(minecraft_version), loader)
                .await?;
            let version = select_version(&versions, &project_id, requested_version_id.as_deref())?;
            let artifact = version
                .artifacts()
                .iter()
                .find(|artifact| artifact.primary())
                .or_else(|| version.artifacts().first())
                .cloned()
                .ok_or_else(|| ExtensionSourceError::NoArtifact {
                    project_id: project_id.clone(),
                    version_id: version.id().to_owned(),
                })?;
            selected.insert(project_id.clone(), version.id().to_owned());
            for dependency in version.dependencies() {
                if dependency.dependency_type() != "required" {
                    continue;
                }
                let dependency_project_id = dependency.project_id().ok_or_else(|| {
                    ExtensionSourceError::MissingDependencyProject {
                        version_id: version.id().to_owned(),
                    }
                })?;
                pending.push((
                    dependency_project_id.to_owned(),
                    dependency.version_id().map(str::to_owned),
                ));
            }
            items.push(ExtensionPlanItem::new(
                MODRINTH_SOURCE.to_owned(),
                project_id,
                version.id().to_owned(),
                version.version_number().to_owned(),
                artifact,
                version.dependencies().to_vec(),
            ));
        }

        Ok(ExtensionPlanResolution::new(
            template_id.to_owned(),
            kind,
            minecraft_version.to_owned(),
            loader.map(str::to_owned),
            items,
        ))
    }

    /// 校验工件 HTTPS URL、允许域名和大小后返回响应流。
    pub(crate) async fn download_artifact(
        &self,
        artifact: &ExtensionArtifact,
    ) -> Result<Response, ExtensionSourceError> {
        let url = Url::parse(artifact.download_url())
            .map_err(|_| ExtensionSourceError::InvalidArtifactUrl)?;
        if url.scheme() != "https"
            || url
                .host_str()
                .is_none_or(|host| !is_allowed_artifact_host(host))
        {
            return Err(ExtensionSourceError::InvalidArtifactUrl);
        }
        if artifact.size() > MAXIMUM_ARTIFACT_BYTES {
            return Err(ExtensionSourceError::ArtifactTooLarge {
                maximum_bytes: MAXIMUM_ARTIFACT_BYTES,
            });
        }

        let response = self
            .client
            .get(url)
            .timeout(ARTIFACT_TIMEOUT)
            .send()
            .await
            .map_err(ExtensionSourceError::Request)?
            .error_for_status()
            .map_err(ExtensionSourceError::Request)?;
        if response
            .content_length()
            .is_some_and(|length| length > MAXIMUM_ARTIFACT_BYTES)
        {
            return Err(ExtensionSourceError::ArtifactTooLarge {
                maximum_bytes: MAXIMUM_ARTIFACT_BYTES,
            });
        }

        Ok(response)
    }
}

async fn read_response(response: Response) -> Result<Vec<u8>, ExtensionSourceError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAXIMUM_RESPONSE_BYTES as u64)
    {
        return Err(ExtensionSourceError::ResponseTooLarge {
            maximum_bytes: MAXIMUM_RESPONSE_BYTES,
        });
    }

    let mut response = response;
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(ExtensionSourceError::Request)?
    {
        let size =
            bytes
                .len()
                .checked_add(chunk.len())
                .ok_or(ExtensionSourceError::ResponseTooLarge {
                    maximum_bytes: MAXIMUM_RESPONSE_BYTES,
                })?;
        if size > MAXIMUM_RESPONSE_BYTES {
            return Err(ExtensionSourceError::ResponseTooLarge {
                maximum_bytes: MAXIMUM_RESPONSE_BYTES,
            });
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(bytes)
}

fn parse_modrinth_search(
    metadata: &Value,
    kind: ExtensionKind,
    minecraft_version: Option<&str>,
    loader: Option<&str>,
    limit: usize,
    offset: usize,
) -> Result<ExtensionSearchResult, ExtensionSourceError> {
    let object = metadata
        .as_object()
        .ok_or(ExtensionSourceError::InvalidResponse)?;
    let hits = object
        .get("hits")
        .and_then(Value::as_array)
        .ok_or(ExtensionSourceError::InvalidResponse)?;
    let total = object
        .get("total_hits")
        .and_then(Value::as_u64)
        .ok_or(ExtensionSourceError::InvalidResponse)?;
    let items = hits
        .iter()
        .map(|hit| parse_project(hit, kind, minecraft_version, loader))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ExtensionSearchResult::new(
        MODRINTH_SOURCE.to_owned(),
        items,
        total,
        limit,
        offset,
    ))
}

fn parse_modrinth_versions(
    metadata: &Value,
    project_id: &str,
    minecraft_version: Option<&str>,
    loader: Option<&str>,
) -> Result<ExtensionVersionResult, ExtensionSourceError> {
    let versions = metadata
        .as_array()
        .ok_or(ExtensionSourceError::InvalidResponse)?;
    let items = versions
        .iter()
        .map(|version| parse_version(version, project_id, minecraft_version, loader))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ExtensionVersionResult::new(
        MODRINTH_SOURCE.to_owned(),
        project_id.to_owned(),
        items,
    ))
}

fn select_version<'a>(
    versions: &'a ExtensionVersionResult,
    project_id: &str,
    requested_version_id: Option<&str>,
) -> Result<&'a ExtensionVersion, ExtensionSourceError> {
    let version = match requested_version_id {
        Some(version_id) => versions
            .items()
            .iter()
            .find(|version| version.id() == version_id)
            .ok_or_else(|| ExtensionSourceError::VersionNotFound {
                project_id: project_id.to_owned(),
                version_id: version_id.to_owned(),
            })?,
        None => versions
            .items()
            .iter()
            .find(|version| version.compatibility() == ExtensionCompatibility::Compatible)
            .ok_or_else(|| ExtensionSourceError::NoCompatibleVersion {
                project_id: project_id.to_owned(),
            })?,
    };
    if version.compatibility() != ExtensionCompatibility::Compatible {
        return Err(ExtensionSourceError::NoCompatibleVersion {
            project_id: project_id.to_owned(),
        });
    }

    Ok(version)
}

fn parse_version(
    value: &Value,
    project_id: &str,
    minecraft_version: Option<&str>,
    loader: Option<&str>,
) -> Result<ExtensionVersion, ExtensionSourceError> {
    let object = value
        .as_object()
        .ok_or(ExtensionSourceError::InvalidResponse)?;
    let id = required_string(object, "id")?;
    let version_number = required_string(object, "version_number")?;
    let game_versions = required_array(object, "game_versions")?;
    let loaders = required_array(object, "loaders")?;
    let dependencies = dependency_array(object)?;
    let artifacts = artifact_array(object)?;
    let compatibility = compatibility(&game_versions, &loaders, minecraft_version, loader);

    Ok(ExtensionVersion::new(
        id,
        project_id.to_owned(),
        object
            .get("name")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or(&version_number)
            .to_owned(),
        version_number,
        game_versions,
        loaders,
        dependencies,
        artifacts,
        object
            .get("downloads")
            .and_then(Value::as_u64)
            .ok_or(ExtensionSourceError::InvalidResponse)?,
        compatibility,
    ))
}

fn required_array(
    object: &Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, ExtensionSourceError> {
    object
        .get(key)
        .ok_or(ExtensionSourceError::InvalidResponse)
        .and_then(string_array_value)
}

fn string_array_value(value: &Value) -> Result<Vec<String>, ExtensionSourceError> {
    value
        .as_array()
        .ok_or(ExtensionSourceError::InvalidResponse)?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or(ExtensionSourceError::InvalidResponse)
        })
        .collect()
}

fn dependency_array(
    object: &Map<String, Value>,
) -> Result<Vec<ExtensionDependency>, ExtensionSourceError> {
    let Some(value) = object.get("dependencies") else {
        return Ok(Vec::new());
    };
    value
        .as_array()
        .ok_or(ExtensionSourceError::InvalidResponse)?
        .iter()
        .map(|value| {
            let object = value
                .as_object()
                .ok_or(ExtensionSourceError::InvalidResponse)?;
            Ok(ExtensionDependency::new(
                optional_string(object, "project_id")?,
                optional_string(object, "version_id")?,
                optional_string(object, "file_name")?,
                required_string(object, "dependency_type")?,
            ))
        })
        .collect()
}

fn artifact_array(
    object: &Map<String, Value>,
) -> Result<Vec<ExtensionArtifact>, ExtensionSourceError> {
    let files = object
        .get("files")
        .and_then(Value::as_array)
        .ok_or(ExtensionSourceError::InvalidResponse)?;
    files.iter().map(parse_artifact).collect()
}

fn parse_artifact(value: &Value) -> Result<ExtensionArtifact, ExtensionSourceError> {
    let object = value
        .as_object()
        .ok_or(ExtensionSourceError::InvalidResponse)?;
    let file_name = required_string(object, "filename")?;
    let download_url = required_string(object, "url")?;
    let parsed_url =
        Url::parse(&download_url).map_err(|_| ExtensionSourceError::InvalidResponse)?;
    if parsed_url.scheme() != "https" || parsed_url.host_str().is_none() {
        return Err(ExtensionSourceError::InvalidResponse);
    }
    let hashes = object
        .get("hashes")
        .and_then(Value::as_object)
        .ok_or(ExtensionSourceError::InvalidResponse)?;
    let sha512 = hashes
        .get("sha512")
        .and_then(Value::as_str)
        .filter(|value| is_hex_digest(value, 128))
        .map(str::to_owned)
        .ok_or(ExtensionSourceError::InvalidResponse)?;
    let sha1 = hashes
        .get("sha1")
        .and_then(Value::as_str)
        .map(|value| {
            if is_hex_digest(value, 40) {
                Ok(value.to_owned())
            } else {
                Err(ExtensionSourceError::InvalidResponse)
            }
        })
        .transpose()?;

    Ok(ExtensionArtifact::new(
        file_name,
        download_url,
        object
            .get("size")
            .and_then(Value::as_u64)
            .ok_or(ExtensionSourceError::InvalidResponse)?,
        sha1,
        sha512,
        object
            .get("primary")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    ))
}

fn optional_string(
    object: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, ExtensionSourceError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .map(Some)
            .ok_or(ExtensionSourceError::InvalidResponse),
    }
}

fn is_hex_digest(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_valid_project_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn parse_project(
    hit: &Value,
    kind: ExtensionKind,
    minecraft_version: Option<&str>,
    loader: Option<&str>,
) -> Result<ExtensionProject, ExtensionSourceError> {
    let object = hit
        .as_object()
        .ok_or(ExtensionSourceError::InvalidResponse)?;
    let project_id = required_string(object, "project_id")?;
    let actual_project_type = required_string(object, "project_type")?;
    if actual_project_type != project_type(kind) {
        return Err(ExtensionSourceError::InvalidResponse);
    }
    let name = required_string(object, "title")?;
    let slug = object
        .get("slug")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| project_id.clone());
    let supported_minecraft_versions = string_array(object, "versions")?;
    let supported_loaders = string_array(object, "categories")?;
    let compatibility = compatibility(
        &supported_minecraft_versions,
        &supported_loaders,
        minecraft_version,
        loader,
    );

    Ok(ExtensionProject::new(
        project_id,
        MODRINTH_SOURCE.to_owned(),
        kind,
        name,
        object
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        format!("https://modrinth.com/{actual_project_type}/{slug}"),
        object
            .get("icon_url")
            .and_then(Value::as_str)
            .map(str::to_owned),
        object
            .get("downloads")
            .and_then(Value::as_u64)
            .ok_or(ExtensionSourceError::InvalidResponse)?,
        supported_minecraft_versions,
        supported_loaders,
        compatibility,
    ))
}

fn required_string(object: &Map<String, Value>, key: &str) -> Result<String, ExtensionSourceError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or(ExtensionSourceError::InvalidResponse)
}

fn string_array(
    object: &Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, ExtensionSourceError> {
    let Some(values) = object.get(key) else {
        return Ok(Vec::new());
    };
    values
        .as_array()
        .ok_or(ExtensionSourceError::InvalidResponse)?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or(ExtensionSourceError::InvalidResponse)
        })
        .collect()
}

fn compatibility(
    supported_minecraft_versions: &[String],
    supported_loaders: &[String],
    minecraft_version: Option<&str>,
    loader: Option<&str>,
) -> ExtensionCompatibility {
    let version_match = matches_value(supported_minecraft_versions, minecraft_version);
    let loader_match = matches_value(supported_loaders, loader);
    if version_match == Some(false) || loader_match == Some(false) {
        ExtensionCompatibility::Incompatible
    } else if version_match == Some(true) || loader_match == Some(true) {
        ExtensionCompatibility::Compatible
    } else {
        ExtensionCompatibility::Unknown
    }
}

fn matches_value(values: &[String], expected: Option<&str>) -> Option<bool> {
    expected.and_then(|expected| {
        (!values.is_empty()).then(|| values.iter().any(|value| value == expected))
    })
}

fn project_type(kind: ExtensionKind) -> &'static str {
    match kind {
        ExtensionKind::Plugin => "plugin",
        ExtensionKind::Mod => "mod",
    }
}

fn is_allowed_artifact_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("modrinth.com")
        || host.to_ascii_lowercase().ends_with(".modrinth.com")
}

#[cfg(test)]
mod tests {
    use super::is_allowed_artifact_host;
    use super::parse_modrinth_search;
    use nexus_domain::ExtensionCompatibility;
    use nexus_domain::ExtensionKind;
    use serde_json::json;

    #[test]
    fn parses_projects_and_reports_requested_compatibility() {
        let result = parse_modrinth_search(
            &json!({
                "total_hits": 1,
                "hits": [{
                    "project_id": "example",
                    "project_type": "mod",
                    "slug": "example-mod",
                    "title": "Example Mod",
                    "description": "A test mod",
                    "icon_url": "https://cdn.example.invalid/icon.png",
                    "downloads": 42,
                    "versions": ["1.21.1"],
                    "categories": ["fabric"]
                }]
            }),
            ExtensionKind::Mod,
            Some("1.21.1"),
            Some("fabric"),
            20,
            0,
        )
        .expect("Modrinth search response is valid");

        assert_eq!(result.source(), "modrinth");
        assert_eq!(result.items().len(), 1);
        assert_eq!(result.items()[0].project_id(), "example");
        assert_eq!(
            result.items()[0].project_url(),
            "https://modrinth.com/mod/example-mod"
        );
        assert_eq!(
            result.items()[0].compatibility(),
            ExtensionCompatibility::Compatible
        );
    }

    #[test]
    fn reports_incompatible_when_a_requested_filter_is_not_supported() {
        let result = parse_modrinth_search(
            &json!({
                "total_hits": 1,
                "hits": [{
                    "project_id": "example",
                    "project_type": "plugin",
                    "title": "Example Plugin",
                    "downloads": 1,
                    "versions": ["1.20.4"],
                    "categories": ["paper"]
                }]
            }),
            ExtensionKind::Plugin,
            Some("1.21.1"),
            Some("paper"),
            20,
            0,
        )
        .expect("Modrinth search response is valid");

        assert_eq!(
            result.items()[0].compatibility(),
            ExtensionCompatibility::Incompatible
        );
    }

    #[test]
    fn rejects_a_project_with_the_wrong_extension_type() {
        let result = parse_modrinth_search(
            &json!({
                "total_hits": 1,
                "hits": [{
                    "project_id": "example",
                    "project_type": "plugin",
                    "title": "Example Plugin",
                    "downloads": 1,
                    "versions": [],
                    "categories": []
                }]
            }),
            ExtensionKind::Mod,
            None,
            None,
            20,
            0,
        );

        assert!(result.is_err());
    }

    #[test]
    fn parses_version_dependencies_and_hashed_artifacts() {
        let result = super::parse_modrinth_versions(
            &json!([{
                "id": "version-id",
                "project_id": "project-id",
                "name": "Release 1",
                "version_number": "1.0.0",
                "game_versions": ["1.21.1"],
                "loaders": ["fabric"],
                "downloads": 7,
                "dependencies": [{
                    "project_id": "dependency-project",
                    "version_id": "dependency-version",
                    "file_name": null,
                    "dependency_type": "required"
                }],
                "files": [{
                    "filename": "example.jar",
                    "url": "https://cdn.example.invalid/example.jar",
                    "size": 123,
                    "primary": true,
                    "hashes": {
                        "sha1": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                        "sha512": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    }
                }]
            }]),
            "project-id",
            Some("1.21.1"),
            Some("fabric"),
        )
        .expect("Modrinth version response is valid");

        assert_eq!(result.project_id(), "project-id");
        assert_eq!(result.items().len(), 1);
        assert_eq!(result.items()[0].dependencies().len(), 1);
        assert_eq!(
            result.items()[0].dependencies()[0].dependency_type(),
            "required"
        );
        assert_eq!(result.items()[0].artifacts()[0].file_name(), "example.jar");
        assert_eq!(result.items()[0].artifacts()[0].size(), 123);
        assert_eq!(result.items()[0].artifacts()[0].sha512().len(), 128);
        assert_eq!(
            result.items()[0].compatibility(),
            ExtensionCompatibility::Compatible
        );
    }

    #[test]
    fn rejects_an_artifact_without_a_secure_strong_hash() {
        let result = super::parse_modrinth_versions(
            &json!([{
                "id": "version-id",
                "version_number": "1.0.0",
                "game_versions": [],
                "loaders": [],
                "downloads": 1,
                "dependencies": [],
                "files": [{
                    "filename": "example.jar",
                    "url": "http://example.invalid/example.jar",
                    "size": 1,
                    "hashes": { "sha1": "bad" }
                }]
            }]),
            "project-id",
            None,
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn restricts_artifact_hosts_to_modrinth_domains() {
        assert!(is_allowed_artifact_host("cdn.modrinth.com"));
        assert!(is_allowed_artifact_host("MODRINTH.COM"));
        assert!(!is_allowed_artifact_host("cdn.example.invalid"));
        assert!(!is_allowed_artifact_host("modrinth.com.example.invalid"));
    }
}
