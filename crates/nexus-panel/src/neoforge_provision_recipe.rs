//! NeoForge installer JAR 的版本化一键搭建配方。

use nexus_domain::DownloadManifest;
use nexus_domain::InstallRuntimeRequirement;
use nexus_domain::InstallTemplate;
use nexus_domain::InstallTemplateVersionKind;
use nexus_domain::InstanceKind;
use nexus_domain::ProvisionFile;
use nexus_domain::ProvisionInstallStrategy;
use nexus_domain::ProvisionPlan;
use nexus_domain::RuntimeArchiveFormat;

use crate::TemplateProvisionError;
use crate::TemplateProvisionRequest;
use crate::VersionMetadataClient;
use crate::version_metadata_client::neoforge_game_version;

/// 校验版本选择并生成可交给 Core 哈希确认的 NeoForge 安装计划。
pub(crate) async fn resolve_neoforge_provision_plan(
    template: &InstallTemplate,
    request: &TemplateProvisionRequest,
    metadata: &VersionMetadataClient,
) -> Result<ProvisionPlan, TemplateProvisionError> {
    validate_request(request)?;
    let expected_game_version = neoforge_game_version(request.loader_version()).ok_or(
        TemplateProvisionError::InvalidField {
            field: "loaderVersion",
        },
    )?;
    if expected_game_version != request.minecraft_version() {
        return Err(TemplateProvisionError::InvalidField {
            field: "minecraftVersion",
        });
    }
    let versions = metadata.list_versions(template).await?;
    if !versions.iter().any(|version| {
        version.kind() == InstallTemplateVersionKind::Loader
            && version.id() == request.loader_version()
            && version.game_version() == Some(request.minecraft_version())
    }) {
        return Err(TemplateProvisionError::InvalidField {
            field: "loaderVersion",
        });
    }

    let manifest = metadata
        .neoforge_installer_manifest(request.loader_version())
        .await?;
    Ok(build_neoforge_provision_plan(template, request, manifest))
}

fn build_neoforge_provision_plan(
    template: &InstallTemplate,
    request: &TemplateProvisionRequest,
    manifest: DownloadManifest,
) -> ProvisionPlan {
    let argument_file = format!(
        "libraries/net/neoforged/neoforge/{}/{{os}}_args.txt",
        request.loader_version()
    );
    let jvm_arguments = if request.jvm_arguments().is_empty() {
        vec!["-Xms1G".to_owned(), "-Xmx2G".to_owned()]
    } else {
        request.jvm_arguments().to_vec()
    };
    let jvm_argument_file = format!("{}\n", jvm_arguments.join("\n"));

    ProvisionPlan::new(
        template.id().to_owned(),
        request.minecraft_version().to_owned(),
        request.loader_version().to_owned(),
        request.instance_id().clone(),
        request.instance_name().trim().to_owned(),
        InstanceKind::NeoForge,
        request.instance_directory().trim().to_owned(),
        None,
        None,
        InstallRuntimeRequirement::Java,
        request.runtime_id().map(str::to_owned),
        manifest,
        RuntimeArchiveFormat::Zip,
        argument_file.clone(),
        vec![
            "@user_jvm_args.txt".to_owned(),
            format!("@{argument_file}"),
            "nogui".to_owned(),
        ],
        request.stop_command().to_owned(),
        request.stop_timeout_seconds(),
    )
    .with_install_strategy(ProvisionInstallStrategy::JavaInstaller)
    .with_required_runtime_version(required_java_major(request.minecraft_version()).to_owned())
    .with_files(vec![ProvisionFile::new(
        "user_jvm_args.txt".to_owned(),
        jvm_argument_file,
    )])
}

fn validate_request(request: &TemplateProvisionRequest) -> Result<(), TemplateProvisionError> {
    if request.instance_name().trim().is_empty() || request.instance_name().len() > 128 {
        return Err(TemplateProvisionError::InvalidField {
            field: "instanceName",
        });
    }
    if request.instance_directory().trim().is_empty() || request.instance_directory().len() > 1024 {
        return Err(TemplateProvisionError::InvalidField {
            field: "instanceDirectory",
        });
    }
    if request.minecraft_version().is_empty() || request.minecraft_version().len() > 32 {
        return Err(TemplateProvisionError::InvalidField {
            field: "minecraftVersion",
        });
    }
    if request.loader_version().is_empty() || request.loader_version().len() > 128 {
        return Err(TemplateProvisionError::InvalidField {
            field: "loaderVersion",
        });
    }
    if request.jvm_arguments().len() > 128
        || request
            .jvm_arguments()
            .iter()
            .any(|argument| argument.is_empty() || argument.len() > 8192 || argument.contains('\0'))
    {
        return Err(TemplateProvisionError::InvalidField {
            field: "jvmArguments",
        });
    }
    if request.stop_command().is_empty()
        || request.stop_command().len() > 8192
        || request.stop_command().contains('\0')
        || !(1..=300).contains(&request.stop_timeout_seconds())
    {
        return Err(TemplateProvisionError::InvalidField {
            field: "stopCommand",
        });
    }
    Ok(())
}

fn required_java_major(minecraft_version: &str) -> &'static str {
    let mut parts = minecraft_version.split('.');
    let _major = parts.next();
    let minor = parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(0);
    let patch = parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(0);
    if minor > 20 || (minor == 20 && patch >= 5) {
        "21"
    } else {
        "17"
    }
}

#[cfg(test)]
mod tests {
    use nexus_domain::DownloadArchitecture;
    use nexus_domain::DownloadManifest;
    use nexus_domain::DownloadPlatform;
    use nexus_domain::ProvisionInstallStrategy;
    use nexus_domain::Sha256Digest;
    use serde_json::from_value;
    use serde_json::json;

    use super::build_neoforge_provision_plan;
    use super::required_java_major;
    use crate::TemplateProvisionRequest;
    use crate::install_template_catalog::install_template;

    #[test]
    fn selects_java_major_from_minecraft_version() {
        assert_eq!(required_java_major("1.20.4"), "17");
        assert_eq!(required_java_major("1.20.5"), "21");
        assert_eq!(required_java_major("1.21.1"), "21");
    }

    #[test]
    fn builds_neoforge_installer_and_platform_launch_arguments() {
        let template = install_template("neoforge").expect("NeoForge template exists");
        let request: TemplateProvisionRequest = from_value(json!({
            "instanceId": "neoforge-1.21.1-server-01",
            "instanceName": "NeoForge 1.21.1 Server 01",
            "instanceDirectory": "instances/neoforge-1.21.1-server-01",
            "minecraftVersion": "1.21.1",
            "loaderVersion": "21.1.217",
            "jvmArguments": ["-Xms1G", "-Xmx2G"]
        }))
        .expect("template request is valid");
        let manifest = DownloadManifest::new(
            "https://example.invalid/neoforge-installer.jar".to_owned(),
            1024,
            Sha256Digest::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .expect("digest is valid"),
            DownloadPlatform::Any,
            DownloadArchitecture::Any,
        );

        let plan = build_neoforge_provision_plan(&template, &request, manifest);

        assert_eq!(
            plan.install_strategy(),
            ProvisionInstallStrategy::JavaInstaller
        );
        assert_eq!(plan.required_runtime_version(), Some("21"));
        assert_eq!(
            plan.executable_path(),
            "libraries/net/neoforged/neoforge/21.1.217/{os}_args.txt"
        );
        assert_eq!(
            plan.launch_arguments(),
            [
                "@user_jvm_args.txt",
                "@libraries/net/neoforged/neoforge/21.1.217/{os}_args.txt",
                "nogui"
            ]
        );
        assert_eq!(plan.files()[0].content(), "-Xms1G\n-Xmx2G\n");
    }
}
