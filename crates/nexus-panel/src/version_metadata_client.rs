//! 安装模板版本元数据的受限 HTTPS 客户端。
//!
//! 每个模板通过 `VersionMetadataProvider` 声明一个或多个来源；客户端只负责读取
//! 和规范化版本元数据，不把成功解析误认为归档布局、启动命令或升级流程已经验证。

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::str;
use std::time::Duration;

use nexus_domain::DownloadArchitecture;
use nexus_domain::DownloadManifest;
use nexus_domain::DownloadPlatform;
use nexus_domain::InstallTemplate;
use nexus_domain::InstallTemplateVersion;
use nexus_domain::InstallTemplateVersionKind;
use nexus_domain::PRODUCT_NAME;
use nexus_domain::PRODUCT_VERSION;
use nexus_domain::Sha256Digest;
use nexus_domain::VersionMetadataProvider;
use quick_xml::Reader;
use quick_xml::events::BytesStart;
use quick_xml::events::Event;
use reqwest::Client;
use reqwest::redirect::Policy;
use rustls::crypto::ring;
use serde_json::Map;
use serde_json::Value;
use url::Url;

use crate::VersionMetadataError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const METADATA_TIMEOUT: Duration = Duration::from_secs(30);
const MAXIMUM_METADATA_BYTES: usize = 2 * 1024 * 1024;
const MOJANG_PROVIDER_ID: &str = "mojang-version-manifest";
const PAPER_PROVIDER_ID: &str = "paper-downloads-service";
const NEOFORGE_PROVIDER_ID: &str = "neoforge-maven-service";
const FORGE_PROVIDER_ID: &str = "forge-version-service";
const BUKKIT_PROVIDER_ID: &str = "bukkit-jenkins-rss";
const SPIGOT_PROVIDER_ID: &str = "spigot-jenkins-rss";
const PURPUR_PROVIDER_ID: &str = "purpur-version-service";
const PUFFERFISH_PROVIDER_IDS: [&str; 5] = [
    "pufferfish-1.21-jenkins-service",
    "pufferfish-1.20-jenkins-service",
    "pufferfish-1.19-jenkins-service",
    "pufferfish-1.18-jenkins-service",
    "pufferfish-1.17-jenkins-service",
];
const MAGMA_PROVIDER_ID: &str = "magma-github-releases";
const SPONGE_PROVIDER_ID: &str = "sponge-github-releases";
const ARCLIGHT_PROVIDER_ID: &str = "arclight-github-releases";
const CATSERVER_PROVIDER_ID: &str = "catserver-github-releases";
const VELOCITY_PROVIDER_ID: &str = "velocity-downloads-service";
const FOLIA_PROVIDER_ID: &str = "folia-downloads-service";
const WATERFALL_PROVIDER_ID: &str = "waterfall-downloads-service";
const BUNGEECORD_PROVIDER_ID: &str = "bungeecord-jenkins-service";
const LIGHTFALL_PROVIDER_ID: &str = "lightfall-github-releases";
const GEYSER_PROVIDER_ID: &str = "geyser-version-service";
const BEDROCK_DEDICATED_SERVER_PROVIDER_ID: &str = "bedrock-dedicated-server-links";
const LEAF_PROVIDER_ID: &str = "leaf-github-releases";
const POCKETMINE_PROVIDER_ID: &str = "pocketmine-github-releases";
const NUKKIT_PROVIDER_ID: &str = "nukkit-opencollab-maven-service";
const CLOUDBURST_NUKKIT_PROVIDER_ID: &str = "cloudburst-nukkit-opencollab-maven-service";
const FABRIC_GAME_PROVIDER_ID: &str = "fabric-game-versions";
const FABRIC_LOADER_PROVIDER_ID: &str = "fabric-loader-versions";
const MOHIST_PROJECT_PROVIDER_ID: &str = "mohist-project-api";
const YOUER_PROJECT_PROVIDER_ID: &str = "youer-project-api";
const SILKARD_BRANCH_PROVIDER_ID: &str = "silkard-github-branches";
const PAPERMC_CONTACT_URL: &str = "https://github.com/Gu-ZT/MinecraftNexusPanel";
const NEOFORGE_MAVEN_BASE_URL: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const MAXIMUM_CHECKSUM_BYTES: usize = 1024;
const MAXIMUM_INSTALLER_BYTES: u64 = 512 * 1024 * 1024;

/// 从模板声明的供应商读取并规范化安装版本的内部客户端。
#[derive(Clone)]
pub(crate) struct VersionMetadataClient {
    client: Client,
}

impl VersionMetadataClient {
    /// 创建仅允许 HTTPS、禁止重定向的元数据客户端。
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

    /// 按模板标识读取其全部版本元数据。
    ///
    /// 该方法支持 Java 服务端、代理、混合端和基岩端 provider；Fabric 需要分别
    /// 读取游戏与加载器版本，Pufferfish 需要合并多个 Minecraft 分支来源。
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
            "neoforge" => {
                let provider = provider(template, NEOFORGE_PROVIDER_ID)?;
                let metadata = self.fetch_bytes(provider).await?;

                parse_neoforge_versions(provider, &metadata)
            }
            "forge" => {
                let provider = provider(template, FORGE_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_forge_versions(provider, &metadata)
            }
            "bukkit" => {
                let provider = provider(template, BUKKIT_PROVIDER_ID)?;
                let metadata = self.fetch_bytes(provider).await?;

                parse_jenkins_rss_versions(provider, &metadata)
            }
            "spigot" => {
                let provider = provider(template, SPIGOT_PROVIDER_ID)?;
                let metadata = self.fetch_bytes(provider).await?;

                parse_jenkins_rss_versions(provider, &metadata)
            }
            "purpur" => {
                let provider = provider(template, PURPUR_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_purpur_versions(provider, &metadata)
            }
            "magma" => {
                let provider = provider(template, MAGMA_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_github_release_versions(provider, &metadata, &[".jar"], true)
            }
            "mohist" => {
                let provider = provider(template, MOHIST_PROJECT_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_project_versions(provider, &metadata)
            }
            "sponge" => {
                let provider = provider(template, SPONGE_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_github_release_versions(provider, &metadata, &[".jar"], false)
            }
            "arclight" => {
                let provider = provider(template, ARCLIGHT_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_github_release_versions(provider, &metadata, &[".jar"], false)
            }
            "youer" => {
                let provider = provider(template, YOUER_PROJECT_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_project_versions(provider, &metadata)
            }
            "silkard" => {
                let provider = provider(template, SILKARD_BRANCH_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_github_branch_versions(provider, &metadata)
            }
            "catserver" => {
                let provider = provider(template, CATSERVER_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_github_release_versions(provider, &metadata, &[".jar"], false)
            }
            "pufferfish" => {
                let mut versions = Vec::new();
                for provider_id in PUFFERFISH_PROVIDER_IDS {
                    let provider = provider(template, provider_id)?;
                    let metadata = self.fetch(provider).await?;
                    versions.extend(parse_pufferfish_versions(provider, &metadata)?);
                }

                Ok(versions)
            }
            "velocity" => {
                let provider = provider(template, VELOCITY_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_velocity_versions(provider, &metadata)
            }
            "folia" => {
                let provider = provider(template, FOLIA_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_paper_versions(provider, &metadata)
            }
            "waterfall" => {
                let provider = provider(template, WATERFALL_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_velocity_versions(provider, &metadata)
            }
            "bungeecord" => {
                let provider = provider(template, BUNGEECORD_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_bungeecord_versions(provider, &metadata)
            }
            "lightfall" => {
                let provider = provider(template, LIGHTFALL_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_github_release_versions(provider, &metadata, &[".jar"], false)
            }
            "geyser" => {
                let provider = provider(template, GEYSER_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_geyser_versions(provider, &metadata)
            }
            "bedrock-dedicated-server" => {
                let provider = provider(template, BEDROCK_DEDICATED_SERVER_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_bedrock_download_versions(provider, &metadata)
            }
            "leaf" => {
                let provider = provider(template, LEAF_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_github_release_versions(provider, &metadata, &[".jar"], false)
            }
            "pocketmine-mp" => {
                let provider = provider(template, POCKETMINE_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_github_release_versions(provider, &metadata, &[".phar"], false)
            }
            "nukkit" => {
                let provider = provider(template, NUKKIT_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_string_versions(provider, &metadata, InstallTemplateVersionKind::Server)
            }
            "cloudburst-nukkit" => {
                let provider = provider(template, CLOUDBURST_NUKKIT_PROVIDER_ID)?;
                let metadata = self.fetch(provider).await?;

                parse_string_versions(provider, &metadata, InstallTemplateVersionKind::Server)
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
        let bytes = self.fetch_bytes(provider).await?;

        serde_json::from_slice(&bytes)
            .map_err(|_| VersionMetadataError::InvalidResponse { provider_id })
    }

    async fn fetch_bytes(
        &self,
        provider: &VersionMetadataProvider,
    ) -> Result<Vec<u8>, VersionMetadataError> {
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

        Ok(bytes)
    }

    /// 读取 NeoForge installer 的大小和 SHA-256，生成可由任意 Core 校验的下载清单。
    pub(crate) async fn neoforge_installer_manifest(
        &self,
        loader_version: &str,
    ) -> Result<DownloadManifest, VersionMetadataError> {
        if !valid_neoforge_version(loader_version) {
            return Err(VersionMetadataError::InvalidResponse {
                provider_id: NEOFORGE_PROVIDER_ID.to_owned(),
            });
        }
        let artifact_url = format!(
            "{NEOFORGE_MAVEN_BASE_URL}/{loader_version}/neoforge-{loader_version}-installer.jar"
        );
        let checksum_url = format!("{artifact_url}.sha256");
        let checksum = self
            .fetch_small_text(NEOFORGE_PROVIDER_ID, &checksum_url)
            .await?;
        let sha256 = checksum
            .split_whitespace()
            .next()
            .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .and_then(|value| Sha256Digest::from_hex(&value.to_ascii_lowercase()).ok())
            .ok_or_else(|| VersionMetadataError::InvalidResponse {
                provider_id: NEOFORGE_PROVIDER_ID.to_owned(),
            })?;
        let response = self
            .client
            .head(&artifact_url)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|source| VersionMetadataError::Request {
                provider_id: NEOFORGE_PROVIDER_ID.to_owned(),
                source,
            })?;
        let size_bytes = response
            .content_length()
            .filter(|size| *size > 0 && *size <= MAXIMUM_INSTALLER_BYTES)
            .ok_or_else(|| VersionMetadataError::InvalidResponse {
                provider_id: NEOFORGE_PROVIDER_ID.to_owned(),
            })?;

        Ok(DownloadManifest::new(
            artifact_url,
            size_bytes,
            sha256,
            DownloadPlatform::Any,
            DownloadArchitecture::Any,
        ))
    }

    async fn fetch_small_text(
        &self,
        provider_id: &str,
        url: &str,
    ) -> Result<String, VersionMetadataError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|source| VersionMetadataError::Request {
                provider_id: provider_id.to_owned(),
                source,
            })?;
        if response
            .content_length()
            .is_some_and(|length| length > MAXIMUM_CHECKSUM_BYTES as u64)
        {
            return Err(VersionMetadataError::ResponseTooLarge {
                provider_id: provider_id.to_owned(),
                maximum_bytes: MAXIMUM_CHECKSUM_BYTES,
            });
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|source| VersionMetadataError::Request {
                provider_id: provider_id.to_owned(),
                source,
            })?;
        if bytes.len() > MAXIMUM_CHECKSUM_BYTES {
            return Err(VersionMetadataError::ResponseTooLarge {
                provider_id: provider_id.to_owned(),
                maximum_bytes: MAXIMUM_CHECKSUM_BYTES,
            });
        }
        str::from_utf8(&bytes).map(str::to_owned).map_err(|_| {
            VersionMetadataError::InvalidResponse {
                provider_id: provider_id.to_owned(),
            }
        })
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

fn parse_neoforge_versions(
    provider: &VersionMetadataProvider,
    metadata: &[u8],
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    let metadata = str::from_utf8(metadata).map_err(|_| invalid_response(provider))?;
    let mut reader = Reader::from_str(metadata);
    reader.config_mut().trim_text(true);
    let mut loader_versions = Vec::new();
    let mut game_versions = BTreeSet::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) if event.name().as_ref() == b"version" => {
                let id = reader
                    .read_text(event.name())
                    .map_err(|_| invalid_response(provider))?
                    .into_owned();
                if id.is_empty() {
                    return Err(invalid_response(provider));
                }

                let game_version =
                    neoforge_game_version(&id).ok_or_else(|| invalid_response(provider))?;
                game_versions.insert(game_version.clone());
                loader_versions.push(
                    InstallTemplateVersion::new(
                        id.clone(),
                        provider.id().to_owned(),
                        InstallTemplateVersionKind::Loader,
                        is_stable_version(&id),
                        Some(format!(
                            "{NEOFORGE_MAVEN_BASE_URL}/{id}/neoforge-{id}-installer.jar"
                        )),
                    )
                    .with_game_version(game_version),
                );
            }
            Ok(Event::Eof) => break,
            Err(_) => return Err(invalid_response(provider)),
            _ => {}
        }
    }

    if loader_versions.is_empty() {
        Err(invalid_response(provider))
    } else {
        let mut versions = game_versions
            .into_iter()
            .map(|game_version| {
                InstallTemplateVersion::new(
                    game_version,
                    provider.id().to_owned(),
                    InstallTemplateVersionKind::Game,
                    true,
                    None,
                )
            })
            .collect::<Vec<_>>();
        versions.extend(loader_versions);
        Ok(versions)
    }
}

pub(crate) fn neoforge_game_version(loader_version: &str) -> Option<String> {
    let mut segments = loader_version.split('.');
    let minor = segments.next()?.parse::<u16>().ok()?;
    let patch = segments.next()?.parse::<u16>().ok()?;
    if patch == 0 {
        Some(format!("1.{minor}"))
    } else {
        Some(format!("1.{minor}.{patch}"))
    }
}

fn valid_neoforge_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_digit()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+'))
}

fn parse_forge_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    let groups = metadata
        .as_object()
        .ok_or_else(|| invalid_response(provider))?;

    parse_string_version_groups(
        provider,
        groups,
        InstallTemplateVersionKind::Server,
        is_stable_version,
    )
}

fn parse_purpur_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    parse_string_versions(provider, metadata, InstallTemplateVersionKind::Game)
}

fn parse_pufferfish_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    let entries = metadata
        .get("builds")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_response(provider))?;

    let versions = entries
        .iter()
        .filter(|entry| {
            entry.get("result").and_then(Value::as_str) == Some("SUCCESS")
                && entry
                    .get("artifacts")
                    .and_then(Value::as_array)
                    .is_some_and(|artifacts| {
                        artifacts.iter().any(|artifact| {
                            artifact
                                .get("relativePath")
                                .and_then(Value::as_str)
                                .is_some_and(|path| path.ends_with(".jar"))
                        })
                    })
        })
        .map(|entry| {
            let id = entry
                .get("number")
                .and_then(Value::as_u64)
                .map(|number| number.to_string())
                .ok_or_else(|| invalid_response(provider))?;
            let metadata_url = required_string(provider, entry, "url")?;

            Ok(InstallTemplateVersion::new(
                id,
                provider.id().to_owned(),
                InstallTemplateVersionKind::Server,
                true,
                Some(metadata_url),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if versions.is_empty() {
        Err(invalid_response(provider))
    } else {
        Ok(versions)
    }
}

fn parse_bungeecord_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    let entries = metadata
        .get("builds")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_response(provider))?;

    entries
        .iter()
        .filter(|entry| entry.get("result").and_then(Value::as_str) == Some("SUCCESS"))
        .map(|entry| {
            let id = entry
                .get("number")
                .and_then(Value::as_u64)
                .map(|number| number.to_string())
                .ok_or_else(|| invalid_response(provider))?;
            let metadata_url = required_string(provider, entry, "url")?;

            Ok(InstallTemplateVersion::new(
                id,
                provider.id().to_owned(),
                InstallTemplateVersionKind::Server,
                true,
                Some(metadata_url),
            ))
        })
        .collect::<Result<Vec<_>, _>>()
        .and_then(|versions| {
            if versions.is_empty() {
                Err(invalid_response(provider))
            } else {
                Ok(versions)
            }
        })
}

fn parse_jenkins_rss_versions(
    provider: &VersionMetadataProvider,
    metadata: &[u8],
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    let metadata = str::from_utf8(metadata).map_err(|_| invalid_response(provider))?;
    let mut reader = Reader::from_str(metadata);
    reader.config_mut().trim_text(true);
    let mut versions = Vec::new();
    let mut in_entry = false;
    let mut title = None;
    let mut metadata_url = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) if event.name().as_ref() == b"entry" => {
                in_entry = true;
                title = None;
                metadata_url = None;
            }
            Ok(Event::Start(event)) if in_entry && event.name().as_ref() == b"title" => {
                title = Some(
                    reader
                        .read_text(event.name())
                        .map_err(|_| invalid_response(provider))?
                        .into_owned(),
                );
            }
            Ok(Event::Start(event)) if in_entry && event.name().as_ref() == b"link" => {
                metadata_url = rss_link_url(&event);
            }
            Ok(Event::Empty(event)) if in_entry && event.name().as_ref() == b"link" => {
                metadata_url = rss_link_url(&event);
            }
            Ok(Event::End(event)) if event.name().as_ref() == b"entry" => {
                let title = title.take().ok_or_else(|| invalid_response(provider))?;
                let id = jenkins_rss_build_id(provider, &title)?;
                let metadata_url = metadata_url
                    .take()
                    .ok_or_else(|| invalid_response(provider))?;
                versions.push(InstallTemplateVersion::new(
                    id,
                    provider.id().to_owned(),
                    InstallTemplateVersionKind::Server,
                    title.ends_with("(stable)"),
                    Some(metadata_url),
                ));
                in_entry = false;
            }
            Ok(Event::Eof) => break,
            Err(_) => return Err(invalid_response(provider)),
            _ => {}
        }
    }

    if versions.is_empty() {
        Err(invalid_response(provider))
    } else {
        Ok(versions)
    }
}

fn rss_link_url(event: &BytesStart<'_>) -> Option<String> {
    event
        .attributes()
        .filter_map(Result::ok)
        .find(|attribute| attribute.key.as_ref() == b"href")
        .and_then(|attribute| String::from_utf8(attribute.value.into_owned()).ok())
        .filter(|value| !value.is_empty())
}

fn jenkins_rss_build_id(
    provider: &VersionMetadataProvider,
    title: &str,
) -> Result<String, VersionMetadataError> {
    title
        .rsplit_once('#')
        .and_then(|(_, value)| value.split_whitespace().next())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| invalid_response(provider))
}

fn parse_geyser_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    parse_string_versions(provider, metadata, InstallTemplateVersionKind::Server)
}

fn parse_bedrock_download_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    let links = metadata
        .get("result")
        .and_then(|result| result.get("links"))
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_response(provider))?;
    let mut versions = BTreeMap::new();

    for link in links {
        let download_type = required_string(provider, link, "downloadType")?;
        let stable = match download_type.as_str() {
            "serverBedrockWindows" | "serverBedrockLinux" => true,
            "serverBedrockPreviewWindows" | "serverBedrockPreviewLinux" => false,
            _ => continue,
        };
        let download_url = required_string(provider, link, "downloadUrl")?;
        let id = bedrock_version_from_url(provider, &download_url)?;
        let entry = versions.entry(id).or_insert((stable, download_url.clone()));
        if stable {
            entry.0 = true;
            entry.1 = download_url;
        }
    }

    if versions.is_empty() {
        return Err(invalid_response(provider));
    }

    Ok(versions
        .into_iter()
        .map(|(id, (stable, download_url))| {
            InstallTemplateVersion::new(
                id,
                provider.id().to_owned(),
                InstallTemplateVersionKind::Server,
                stable,
                Some(download_url),
            )
        })
        .collect())
}

fn bedrock_version_from_url(
    provider: &VersionMetadataProvider,
    download_url: &str,
) -> Result<String, VersionMetadataError> {
    let url = Url::parse(download_url).map_err(|_| invalid_response(provider))?;
    let file_name = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .ok_or_else(|| invalid_response(provider))?;
    file_name
        .strip_prefix("bedrock-server-")
        .and_then(|value| value.strip_suffix(".zip"))
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| invalid_response(provider))
}

fn parse_github_release_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
    asset_suffixes: &[&str],
    include_prereleases: bool,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    let entries = metadata
        .as_array()
        .ok_or_else(|| invalid_response(provider))?;

    let versions = entries
        .iter()
        .filter(|entry| {
            entry.get("draft").and_then(Value::as_bool) != Some(true)
                && (include_prereleases
                    || entry.get("prerelease").and_then(Value::as_bool) != Some(true))
                && has_github_asset(entry, asset_suffixes)
        })
        .map(|entry| {
            let id = required_string(provider, entry, "tag_name")?;
            let metadata_url = required_string(provider, entry, "html_url")?;
            let stable = entry.get("prerelease").and_then(Value::as_bool) != Some(true)
                && is_stable_version(&id);

            Ok(InstallTemplateVersion::new(
                id,
                provider.id().to_owned(),
                InstallTemplateVersionKind::Server,
                stable,
                Some(metadata_url),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if versions.is_empty() {
        Err(invalid_response(provider))
    } else {
        Ok(versions)
    }
}

fn has_github_asset(entry: &Value, asset_suffixes: &[&str]) -> bool {
    entry
        .get("assets")
        .and_then(Value::as_array)
        .is_some_and(|assets| {
            assets.iter().any(|asset| {
                asset
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| asset_suffixes.iter().any(|suffix| name.ends_with(suffix)))
                    && asset
                        .get("browser_download_url")
                        .and_then(Value::as_str)
                        .is_some_and(|url| !url.is_empty())
            })
        })
}

fn parse_project_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    let entries = metadata
        .as_array()
        .ok_or_else(|| invalid_response(provider))?;

    entries
        .iter()
        .map(|entry| {
            let id = entry
                .get("name")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| invalid_response(provider))?;

            Ok(InstallTemplateVersion::new(
                id.clone(),
                provider.id().to_owned(),
                InstallTemplateVersionKind::Server,
                is_stable_version(&id),
                None,
            ))
        })
        .collect()
}

fn parse_github_branch_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    let entries = metadata
        .as_array()
        .ok_or_else(|| invalid_response(provider))?;
    let versions = entries
        .iter()
        .filter_map(|entry| {
            let id = entry.get("name")?.as_str()?.to_owned();
            is_version_branch(&id).then_some(InstallTemplateVersion::new(
                id.clone(),
                provider.id().to_owned(),
                InstallTemplateVersionKind::Server,
                is_stable_version(&id),
                None,
            ))
        })
        .collect::<Vec<_>>();

    if versions.is_empty() {
        Err(invalid_response(provider))
    } else {
        Ok(versions)
    }
}

fn is_version_branch(branch: &str) -> bool {
    branch
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
        && branch.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
}

fn parse_string_versions(
    provider: &VersionMetadataProvider,
    metadata: &Value,
    kind: InstallTemplateVersionKind,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError> {
    let entries = metadata
        .get("versions")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_response(provider))?;

    entries
        .iter()
        .map(|entry| {
            let id = entry
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| invalid_response(provider))?;

            Ok(InstallTemplateVersion::new(
                id.clone(),
                provider.id().to_owned(),
                kind,
                is_stable_version(&id),
                None,
            ))
        })
        .collect()
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
    parse_string_version_groups(provider, groups, kind, stable)
}

fn parse_string_version_groups<F>(
    provider: &VersionMetadataProvider,
    groups: &Map<String, Value>,
    kind: InstallTemplateVersionKind,
    stable: F,
) -> Result<Vec<InstallTemplateVersion>, VersionMetadataError>
where
    F: Fn(&str) -> bool,
{
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
    !version.contains("SNAPSHOT")
        && !version.contains("-alpha")
        && !version.contains("-beta")
        && !version.contains("-DEV")
        && !version.contains("-dev")
        && !version.contains("-pre")
        && !version.contains("-rc")
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

    use super::parse_bedrock_download_versions;
    use super::parse_bungeecord_versions;
    use super::parse_fabric_versions;
    use super::parse_forge_versions;
    use super::parse_geyser_versions;
    use super::parse_github_branch_versions;
    use super::parse_github_release_versions;
    use super::parse_jenkins_rss_versions;
    use super::parse_mojang_versions;
    use super::parse_neoforge_versions;
    use super::parse_paper_versions;
    use super::parse_project_versions;
    use super::parse_pufferfish_versions;
    use super::parse_purpur_versions;
    use super::parse_string_versions;
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
    fn parses_neoforge_maven_versions() {
        let versions = parse_neoforge_versions(
            &provider("neoforge-maven-service"),
            br#"<?xml version="1.0" encoding="UTF-8"?>
                <metadata>
                    <versioning>
                        <versions>
                            <version>20.2.12-beta</version>
                            <version>21.1.200</version>
                        </versions>
                    </versioning>
                </metadata>"#,
        )
        .expect("NeoForge metadata is valid");

        assert_eq!(versions.len(), 4);
        assert!(versions.iter().any(|version| {
            version.id() == "1.20.2" && version.kind() == InstallTemplateVersionKind::Game
        }));
        assert!(versions.iter().any(|version| {
            version.id() == "1.21.1" && version.kind() == InstallTemplateVersionKind::Game
        }));
        let beta = versions
            .iter()
            .find(|version| version.id() == "20.2.12-beta")
            .expect("beta loader is present");
        assert_eq!(beta.kind(), InstallTemplateVersionKind::Loader);
        assert_eq!(beta.game_version(), Some("1.20.2"));
        assert!(!beta.stable());
        let stable = versions
            .iter()
            .find(|version| version.id() == "21.1.200")
            .expect("stable loader is present");
        assert_eq!(stable.game_version(), Some("1.21.1"));
        assert!(stable.stable());
    }

    #[test]
    fn parses_forge_minecraft_and_build_versions() {
        let versions = parse_forge_versions(
            &provider("forge-version-service"),
            &json!({
                "1.21.8": ["1.21.8-58.1.20", "1.21.8-58.1.0"],
                "1.21.7": ["1.21.7-57.0.3"]
            }),
        )
        .expect("Forge metadata is valid");

        assert_eq!(versions.len(), 3);
        let latest = versions
            .iter()
            .find(|version| version.id() == "1.21.8-58.1.20")
            .expect("Forge build is present");
        assert_eq!(latest.kind(), InstallTemplateVersionKind::Server);
        assert!(versions.iter().all(|version| version.stable()));
    }

    #[test]
    fn parses_bungeecord_build_versions() {
        let versions = parse_bungeecord_versions(
            &provider("bungeecord-jenkins-service"),
            &json!({
                "builds": [
                    { "number": 2085, "result": "SUCCESS", "url": "https://example.invalid/2085/" },
                    { "number": 2084, "result": "FAILURE", "url": "https://example.invalid/2084/" },
                    { "number": 2083, "result": "SUCCESS", "url": "https://example.invalid/2083/" }
                ]
            }),
        )
        .expect("BungeeCord metadata is valid");

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].id(), "2085");
        assert_eq!(versions[0].kind(), InstallTemplateVersionKind::Server);
        assert_eq!(
            versions[0].metadata_url(),
            Some("https://example.invalid/2085/")
        );
    }

    #[test]
    fn parses_bukkit_and_spigot_jenkins_rss_versions() {
        let bukkit = parse_jenkins_rss_versions(
            &provider("bukkit-jenkins-rss"),
            br#"<?xml version="1.0" encoding="UTF-8"?>
                <feed xmlns="http://www.w3.org/2005/Atom">
                    <entry>
                        <title>Bukkit-RSS #1452 (stable)</title>
                        <link rel="alternate" href="https://example.invalid/bukkit/1452/" />
                    </entry>
                    <entry>
                        <title>Bukkit-RSS #1451 (unstable)</title>
                        <link rel="alternate" href="https://example.invalid/bukkit/1451/" />
                    </entry>
                </feed>"#,
        )
        .expect("Bukkit RSS metadata is valid");
        let spigot = parse_jenkins_rss_versions(
            &provider("spigot-jenkins-rss"),
            br#"<feed xmlns="http://www.w3.org/2005/Atom">
                    <entry>
                        <title>Spigot-RSS #719 (stable)</title>
                        <link rel="alternate" href="https://example.invalid/spigot/719/" />
                    </entry>
                </feed>"#,
        )
        .expect("Spigot RSS metadata is valid");

        assert_eq!(bukkit.len(), 2);
        assert_eq!(bukkit[0].id(), "1452");
        assert!(bukkit[0].stable());
        assert!(!bukkit[1].stable());
        assert_eq!(bukkit[0].kind(), InstallTemplateVersionKind::Server);
        assert_eq!(spigot[0].id(), "719");
        assert_eq!(
            spigot[0].metadata_url(),
            Some("https://example.invalid/spigot/719/")
        );
    }

    #[test]
    fn parses_geyser_server_versions() {
        let versions = parse_geyser_versions(
            &provider("geyser-version-service"),
            &json!({ "versions": ["2.11.0", "2.10.1"] }),
        )
        .expect("Geyser metadata is valid");

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].kind(), InstallTemplateVersionKind::Server);
        assert!(versions.iter().all(|version| version.stable()));
    }

    #[test]
    fn parses_bedrock_stable_and_preview_downloads() {
        let versions = parse_bedrock_download_versions(
            &provider("bedrock-dedicated-server-links"),
            &json!({
                "result": {
                    "links": [
                        {
                            "downloadType": "serverBedrockWindows",
                            "downloadUrl": "https://www.minecraft.net/bedrockdedicatedserver/bin-win/bedrock-server-1.26.36.1.zip"
                        },
                        {
                            "downloadType": "serverBedrockLinux",
                            "downloadUrl": "https://www.minecraft.net/bedrockdedicatedserver/bin-linux/bedrock-server-1.26.36.1.zip"
                        },
                        {
                            "downloadType": "serverBedrockPreviewWindows",
                            "downloadUrl": "https://www.minecraft.net/bedrockdedicatedserver/bin-win-preview/bedrock-server-1.26.50.22.zip"
                        },
                        {
                            "downloadType": "serverJar",
                            "downloadUrl": "https://example.invalid/server.jar"
                        }
                    ]
                }
            }),
        )
        .expect("Bedrock metadata is valid");

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].id(), "1.26.36.1");
        assert!(versions[0].stable());
        assert_eq!(versions[1].id(), "1.26.50.22");
        assert!(!versions[1].stable());
    }

    #[test]
    fn parses_purpur_versions() {
        let versions = parse_purpur_versions(
            &provider("purpur-version-service"),
            &json!({ "versions": ["1.21.8", "1.21.8-rc-1"] }),
        )
        .expect("Purpur metadata is valid");

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].kind(), InstallTemplateVersionKind::Game);
        assert!(versions[0].stable());
        assert!(!versions[1].stable());
    }

    #[test]
    fn parses_pufferfish_successful_jar_builds() {
        let versions = parse_pufferfish_versions(
            &provider("pufferfish-1.21-jenkins-service"),
            &json!({
                "builds": [
                    {
                        "number": 39,
                        "result": "SUCCESS",
                        "url": "https://example.invalid/job/Pufferfish-1.21/39/",
                        "artifacts": [{ "relativePath": "builds/pufferfish.jar" }]
                    },
                    {
                        "number": 38,
                        "result": "FAILURE",
                        "url": "https://example.invalid/job/Pufferfish-1.21/38/",
                        "artifacts": []
                    },
                    {
                        "number": 37,
                        "result": "SUCCESS",
                        "url": "https://example.invalid/job/Pufferfish-1.21/37/",
                        "artifacts": [{ "relativePath": "builds/notes.txt" }]
                    }
                ]
            }),
        )
        .expect("Pufferfish metadata is valid");

        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].id(), "39");
        assert!(versions[0].stable());
        assert_eq!(
            versions[0].metadata_url(),
            Some("https://example.invalid/job/Pufferfish-1.21/39/")
        );
    }

    #[test]
    fn parses_leaf_published_releases_with_jars() {
        let versions = parse_github_release_versions(
            &provider("leaf-github-releases"),
            &json!([
                {
                    "tag_name": "ver-1.21.8",
                    "draft": false,
                    "prerelease": false,
                    "html_url": "https://github.com/Winds-Studio/Leaf/releases/tag/ver-1.21.8",
                    "assets": [{
                        "name": "leaf-1.21.8.jar",
                        "browser_download_url": "https://example.invalid/leaf.jar"
                    }]
                },
                {
                    "tag_name": "ver-1.21.9-rc1",
                    "draft": false,
                    "prerelease": true,
                    "html_url": "https://example.invalid/rc",
                    "assets": [{
                        "name": "leaf-rc.jar",
                        "browser_download_url": "https://example.invalid/leaf-rc.jar"
                    }]
                },
                {
                    "tag_name": "ver-1.21.7",
                    "draft": false,
                    "prerelease": false,
                    "html_url": "https://example.invalid/no-jar",
                    "assets": []
                }
            ]),
            &[".jar"],
            false,
        )
        .expect("Leaf metadata is valid");

        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].id(), "ver-1.21.8");
        assert_eq!(versions[0].kind(), InstallTemplateVersionKind::Server);
        assert!(versions[0].stable());
    }

    #[test]
    fn parses_prerelease_phar_assets_and_opencollab_versions() {
        let pocketmine = parse_github_release_versions(
            &provider("pocketmine-github-releases"),
            &json!([
                {
                    "tag_name": "5.44.3",
                    "draft": false,
                    "prerelease": false,
                    "html_url": "https://example.invalid/pocketmine/5.44.3",
                    "assets": [{
                        "name": "PocketMine-MP.phar",
                        "browser_download_url": "https://example.invalid/PocketMine-MP.phar"
                    }]
                }
            ]),
            &[".phar"],
            false,
        )
        .expect("PocketMine-MP metadata is valid");
        let magma = parse_github_release_versions(
            &provider("magma-github-releases"),
            &json!([
                {
                    "tag_name": "va549e0d-DEV",
                    "draft": false,
                    "prerelease": true,
                    "html_url": "https://example.invalid/magma/dev",
                    "assets": [{
                        "name": "Magma-server.jar",
                        "browser_download_url": "https://example.invalid/Magma-server.jar"
                    }]
                }
            ]),
            &[".jar"],
            true,
        )
        .expect("Magma metadata is valid");
        let nukkit = parse_string_versions(
            &provider("nukkit-opencollab-maven-service"),
            &json!({ "versions": ["1.0-SNAPSHOT", "2.0.0-SNAPSHOT"] }),
            InstallTemplateVersionKind::Server,
        )
        .expect("OpenCollab metadata is valid");

        assert_eq!(pocketmine[0].id(), "5.44.3");
        assert!(pocketmine[0].stable());
        assert_eq!(magma[0].id(), "va549e0d-DEV");
        assert!(!magma[0].stable());
        assert!(nukkit.iter().all(|version| !version.stable()));
    }

    #[test]
    fn parses_mohist_project_versions() {
        let versions = parse_project_versions(
            &provider("mohist-project-api"),
            &json!([{ "name": "1.21.1" }, { "name": "26.2-SNAPSHOT" }]),
        )
        .expect("Mohist project metadata is valid");

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].id(), "1.21.1");
        assert_eq!(versions[0].kind(), InstallTemplateVersionKind::Server);
        assert!(versions[0].stable());
        assert!(!versions[1].stable());
    }

    #[test]
    fn parses_silkard_version_branches() {
        let versions = parse_github_branch_versions(
            &provider("silkard-github-branches"),
            &json!([
                { "name": "main" },
                { "name": "26.3" },
                { "name": "26.3-dev" }
            ]),
        )
        .expect("Silkard branch metadata is valid");

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].id(), "26.3");
        assert!(versions[0].stable());
        assert!(!versions[1].stable());
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
