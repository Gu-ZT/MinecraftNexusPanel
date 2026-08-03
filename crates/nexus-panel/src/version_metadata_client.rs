use std::time::Duration;

use nexus_domain::InstallTemplate;
use nexus_domain::InstallTemplateVersion;
use nexus_domain::InstallTemplateVersionKind;
use nexus_domain::PRODUCT_NAME;
use nexus_domain::PRODUCT_VERSION;
use nexus_domain::VersionMetadataProvider;
use reqwest::Client;
use reqwest::redirect::Policy;
use rustls::crypto::ring;
use serde_json::Value;

use crate::VersionMetadataError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const METADATA_TIMEOUT: Duration = Duration::from_secs(30);
const MAXIMUM_METADATA_BYTES: usize = 2 * 1024 * 1024;
const MOJANG_PROVIDER_ID: &str = "mojang-version-manifest";
const PAPER_PROVIDER_ID: &str = "paper-downloads-service";
const VELOCITY_PROVIDER_ID: &str = "velocity-downloads-service";
const FABRIC_GAME_PROVIDER_ID: &str = "fabric-game-versions";
const FABRIC_LOADER_PROVIDER_ID: &str = "fabric-loader-versions";
const PAPERMC_CONTACT_URL: &str = "https://github.com/Gu-ZT/MinecraftNexusPanel";

#[derive(Clone)]
pub(crate) struct VersionMetadataClient {
    client: Client,
}

impl VersionMetadataClient {
    pub(crate) fn new() -> Result<Self, VersionMetadataError> {
        let _ = ring::default_provider().install_default();
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(METADATA_TIMEOUT)
            .https_only(true)
            .redirect(Policy::none())
            .user_agent(format!(
                "{PRODUCT_NAME}/{PRODUCT_VERSION} ({PAPERMC_CONTACT_URL})"
            ))
            .build()
            .map_err(VersionMetadataError::Client)?;

        Ok(Self { client })
    }

    pub(crate) async fn list_versions(
        &self,
        template: &InstallTemplate,
    ) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
        match template.id() {
            "vanilla" => {
                let provider = provider(template, MOJANG_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_mojang_versions(provider, &metadata)
            }
            "paper" => {
                let provider = provider(template, PAPER_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_paper_versions(provider, &metadata)
            }
            "velocity" => {
                let provider = provider(template, VELOCITY_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_velocity_versions(provider, &metadata)
            }
            "fabric" => {
                let game_provider = provider(template, FABRIC_GAME_PROVIDER_ID)?;
                let loader_provider = provider(template, FABRIC_LOADER_PROVIDER_ID)?;
                let game_metadata = self.fetch(game_provider).await?;
                let loader_metadata = self.fetch(loader_provider).await?;
                let mut versions = parse_fabric_versions(
                    game_provider,
                    &game_metadata,
                    InstallTemplateVersionKind::Game,
                )?;
                versions.extend(parse_fabric_versions(
                    loader_provider,
                    &loader_metadata,
                    InstallTemplateVersionKind::Loader,
                )?);

                Ok(versions)
            }
            _ => Err(VersionMetadataError::UnsupportedTemplate {
                template_id: template.id().to_owned(),
            }),
        }
    }

    async fn fetch(
        &self,
        provider: &VersionMetadataProvider,
    ) -> Result<Value, VersionMetadataError> {
        let provider_id = provider.id().to_owned();
        let mut response = self
            .client
            .get(provider.url())
            .send()
            .await
            .map_err(|source| VersionMetadataError::Request {
                provider_id: provider_id.clone(),
                source,
            })?
            .error_for_status()
            .map_err(|source| VersionMetadataError::Request {
                provider_id: provider_id.clone(),
                source,
            })?;
        if response
            .content_length()
            .is_some_and(|length| length > MAXIMUM_METADATA_BYTES as u64)
        {
            return Err(VersionMetadataError::ResponseTooLarge {
                provider_id,
                maximum_bytes: MAXIMUM_METADATA_BYTES,
            });
        }

        let mut bytes = Vec::new();
        while let Some(chunk) =
            response
                .chunk()
                .await
                .map_err(|source| VersionMetadataError::Request {
                    provider_id: provider_id.clone(),
                    source,
                })?
        {
            let size = bytes.len().checked_add(chunk.len()).ok_or_else(|| {
                VersionMetadataError::ResponseTooLarge {
                    provider_id: provider_id.clone(),
                    maximum_bytes: MAXIMUM_METADATA_BYTES,
                }
            })?;
            if size > MAXIMUM_METADATA_BYTES {
                return Err(VersionMetadataError::ResponseTooLarge {
                    provider_id,
                    maximum_bytes: MAXIMUM_METADATA_BYTES,
                });
            }
            bytes.extend_from_slice(&chunk);
        }

        serde_json::from_slice(&bytes)
            .map_err(|_| VersionMetadataError::InvalidResponse { provider_id })
    }
}

fn provider<'a>(
    template: &'a InstallTemplate,
    provider_id: &str,
) -> Result<&'a VersionMetadataProvider, VersionMetadataError> {
    template
        .metadata_providers()
        .iter()
        .find(|provider| provider.id() == provider_id)
        .ok_or_else(|| VersionMetadataError::ProviderMissing {
            template_id: template.id().to_owned(),
            provider_id: provider_id.to_owned(),
        })
}

fn parse_mojang_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    version_entries(provider, metadata, "versions")?
        .iter()
        .map(|entry| {
            let id = required_string(provider, entry, "id")?;
            let metadata_url = required_string(provider, entry, "url")?;
            let stable = entry.get("type").and_then(Value::as_str) == Some("release");

            Ok(InstallTemplateVersion::new(
                id,
                provider.id().to_owned(),
                InstallTemplateVersionKind::Game,
                stable,
                Some(metadata_url),
            ))
        })
        .collect()
}

fn parse_paper_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    parse_grouped_string_versions(
        provider,
        metadata,
        InstallTemplateVersionKind::Game,
        is_stable_version,
    )
}

fn parse_velocity_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    parse_grouped_string_versions(
        provider,
        metadata,
        InstallTemplateVersionKind::Server,
        is_stable_version,
    )
}

fn parse_fabric_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
    kind: InstallTemplateVersionKind,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    let entries = metadata
        .as_array()
        .ok_or_else(|| invalid_response(provider))?;

    entries
        .iter()
        .map(|entry| {
            let id = required_string(provider, entry, "version")?;
            let stable = entry
                .get("stable")
                .and_then(Value::as_bool)
                .ok_or_else(|| invalid_response(provider))?;

            Ok(InstallTemplateVersion::new(
                id,
                provider.id().to_owned(),
                kind,
                stable,
                None,
            ))
        })
        .collect()
}

fn parse_grouped_string_versions<F>(
    provider: &VersionMetadataProvider,
    metadata: &Value,
    kind: InstallTemplateVersionKind,
    stable: F,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError>
where
    F: Fn(&str) -> bool,
{
    let groups = metadata
        .get("versions")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response(provider))?;
    let mut versions = Vec::new();

    for entries in groups.values() {
        let entries = entries
            .as_array()
            .ok_or_else(|| invalid_response(provider))?;
        for entry in entries {
            let id = entry
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| invalid_response(provider))?;

            versions.push(InstallTemplateVersion::new(
                id.clone(),
                provider.id().to_owned(),
                kind,
                stable(&id),
                None,
            ));
        }
    }

    Ok(versions)
}

fn is_stable_version(version: &str) -> bool {
    !version.contains("SNAPSHOT") && !version.contains("-pre") && !version.contains("-rc")
}

fn version_entries<'a>(
    provider: &VersionMetadataProvider,
    metadata: &'a Value,
    field: &str,
) -> Result<&'a Vec<Value>, VersionMetadataError> {
    metadata
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_response(provider))
}

fn required_string(
    provider: &VersionMetadataProvider,
    value: &Value,
    field: &str,
) -> Result<String, VersionMetadataError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| invalid_response(provider))
}

fn invalid_response(provider: &VersionMetadataProvider) -> VersionMetadataError {
    VersionMetadataError::InvalidResponse {
        provider_id: provider.id().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use nexus_domain::InstallTemplateVersionKind;
    use nexus_domain::VersionMetadataProvider;
    use serde_json::json;

    use super::parse_fabric_versions;
    use super::parse_mojang_versions;
    use super::parse_paper_versions;
    use super::parse_velocity_versions;

    #[test]
    fn parses_mojang_game_versions() {
        let versions = parse_mojang_versions(
            &provider("mojang-version-manifest"),
            &json!({
                "versions": [
                    { "id": "1.21.8", "type": "release", "url": "https://example.invalid/1.21.8.json" },
                    { "id": "25w33a", "type": "snapshot", "url": "https://example.invalid/25w33a.json" }
                ]
            }),
        )
        .expect("Mojang metadata is valid");

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].kind(), InstallTemplateVersionKind::Game);
        assert!(versions[0].stable());
        assert_eq!(
            versions[0].metadata_url(),
            Some("https://example.invalid/1.21.8.json")
        );
        assert!(!versions[1].stable());
    }

    #[test]
    fn parses_paper_and_velocity_versions() {
        let paper = parse_paper_versions(
            &provider("paper-downloads-service"),
            &json!({ "versions": { "1.21": ["1.21.8", "1.21.8-rc-1"] } }),
        )
        .expect("Paper metadata is valid");
        let velocity = parse_velocity_versions(
            &provider("velocity-downloads-service"),
            &json!({ "versions": { "3.0.0": ["3.4.0-SNAPSHOT", "3.3.0"] } }),
        )
        .expect("Velocity metadata is valid");

        assert_eq!(paper[0].kind(), InstallTemplateVersionKind::Game);
        assert!(paper[0].stable());
        assert!(!paper[1].stable());
        assert_eq!(velocity[0].kind(), InstallTemplateVersionKind::Server);
        assert!(!velocity[0].stable());
        assert!(velocity[1].stable());
    }

    #[test]
    fn parses_fabric_game_and_loader_versions() {
        let game = parse_fabric_versions(
            &provider("fabric-game-versions"),
            &json!([{ "version": "1.21.8", "stable": true }]),
            InstallTemplateVersionKind::Game,
        )
        .expect("Fabric game metadata is valid");
        let loader = parse_fabric_versions(
            &provider("fabric-loader-versions"),
            &json!([{ "version": "0.16.14", "stable": true }]),
            InstallTemplateVersionKind::Loader,
        )
        .expect("Fabric loader metadata is valid");

        assert_eq!(game[0].kind(), InstallTemplateVersionKind::Game);
        assert_eq!(loader[0].kind(), InstallTemplateVersionKind::Loader);
        assert_eq!(loader[0].provider_id(), "fabric-loader-versions");
    }

    fn provider(id: &str) -> VersionMetadataProvider {
        VersionMetadataProvider::new(
            id.to_owned(),
            "Test provider".to_owned(),
            "https://example.invalid/metadata".to_owned(),
        )
    }
}
