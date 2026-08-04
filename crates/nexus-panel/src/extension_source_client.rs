use std::time::Duration;

use nexus_domain::ExtensionCompatibility;
use nexus_domain::ExtensionKind;
use nexus_domain::ExtensionProject;
use nexus_domain::ExtensionSearchResult;
use reqwest::Client;
use reqwest::Response;
use reqwest::redirect::Policy;
use rustls::crypto::ring;
use serde_json::Map;
use serde_json::Value;
use serde_json::from_slice;
use serde_json::to_string;

use crate::extension_source_error::ExtensionSourceError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const SEARCH_TIMEOUT: Duration = Duration::from_secs(30);
const MAXIMUM_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MODRINTH_SEARCH_URL: &str = "https://api.modrinth.com/v2/search";
const MODRINTH_SOURCE: &str = "modrinth";

#[derive(Clone)]
pub(crate) struct ExtensionSourceClient {
    client: Client,
}

impl ExtensionSourceClient {
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

#[cfg(test)]
mod tests {
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
}
