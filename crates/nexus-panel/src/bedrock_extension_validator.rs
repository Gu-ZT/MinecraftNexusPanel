//! 基岩插件归档的 manifest 读取和 API 兼容性校验。
//!
//! 校验在归档写入 Core 之前执行，因而失败时不会留下半成品插件。PocketMine-MP
//! 常用 PHAR/TAR 归档，Nukkit 系列常用 JAR/ZIP 归档；两者都必须在归档根部
//! 提供 'plugin.yml'。当前 API 匹配只在调用方提供目标 API 列表时执行，缺少
//! 目标列表时不会凭 Minecraft 版本字符串猜测 Bedrock 插件 API。

use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;

use nexus_domain::InstanceKind;
use serde_yaml::Value;
use tar::Archive;
use zip::ZipArchive;

const MAXIMUM_MANIFEST_BYTES: u64 = 64 * 1024;
const MAXIMUM_PHAR_STUB_BYTES: usize = 1024 * 1024;

/// 在归档落盘到 Core 之前校验基岩插件的 manifest 和可选 API 交集。
pub(crate) fn validate_artifact(
    path: &Path,
    file_name: &str,
    instance_kind: InstanceKind,
    expected_api_versions: Option<&[String]>,
) -> Result<(), String> {
    if !matches!(
        instance_kind,
        InstanceKind::PocketMineMp | InstanceKind::Nukkit | InstanceKind::CloudburstNukkit
    ) {
        return Ok(());
    }

    let manifest = read_manifest(path, file_name)?;
    if let Some(expected_api_versions) = expected_api_versions {
        if expected_api_versions.is_empty() {
            return Err("Bedrock API compatibility target cannot be empty".to_owned());
        }
        if !manifest.api_versions.iter().any(|actual| {
            expected_api_versions
                .iter()
                .any(|expected| expected == actual)
        }) {
            return Err(format!(
                "Bedrock plugin API versions [{}] do not match the requested target",
                manifest.api_versions.join(", ")
            ));
        }
    }

    Ok(())
}

fn read_manifest(path: &Path, file_name: &str) -> Result<PluginManifest, String> {
    let extension = Path::new(file_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    let manifest = match extension.as_deref() {
        Some("jar" | "zip") => read_zip_manifest(path)?,
        Some("phar" | "tar") => read_tar_manifest(path)?,
        _ => {
            return Err(
                "Bedrock plugin artifacts must use a JAR/ZIP or PHAR/TAR archive".to_owned(),
            );
        }
    };
    parse_manifest(&manifest)
}

fn read_zip_manifest(path: &Path) -> Result<Vec<u8>, String> {
    let file =
        File::open(path).map_err(|error| format!("failed to open plugin archive: {error}"))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| format!("plugin archive is not a valid ZIP: {error}"))?;
    let mut entry = archive
        .by_name("plugin.yml")
        .map_err(|_| "plugin archive does not contain a root plugin.yml".to_owned())?;
    read_limited(&mut entry)
}

fn read_tar_manifest(path: &Path) -> Result<Vec<u8>, String> {
    let mut file =
        File::open(path).map_err(|error| format!("failed to open plugin archive: {error}"))?;
    let offset = find_tar_offset(&mut file)?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| format!("failed to seek to the PHAR archive: {error}"))?;

    let mut archive = Archive::new(file);
    let entries = archive
        .entries()
        .map_err(|error| format!("plugin archive is not a valid TAR: {error}"))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|error| format!("failed to read plugin archive entry: {error}"))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let entry_path = entry
            .path()
            .map_err(|error| format!("plugin archive entry path is invalid: {error}"))?;
        if entry_path == Path::new("plugin.yml") {
            return read_limited(&mut entry);
        }
    }

    Err("plugin archive does not contain a root plugin.yml".to_owned())
}

fn find_tar_offset(file: &mut File) -> Result<u64, String> {
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("failed to seek to the plugin archive: {error}"))?;
    let mut prefix = vec![0; MAXIMUM_PHAR_STUB_BYTES];
    let bytes_read = file
        .read(&mut prefix)
        .map_err(|error| format!("failed to inspect the PHAR archive: {error}"))?;
    prefix.truncate(bytes_read);

    if bytes_read < 262 {
        return Err("plugin archive does not contain a TAR header".to_owned());
    }
    for magic_start in 257..=bytes_read - 5 {
        if &prefix[magic_start..magic_start + 5] == b"ustar" {
            return u64::try_from(magic_start - 257)
                .map_err(|_| "PHAR archive offset is invalid".to_owned());
        }
    }
    Err("plugin archive does not contain a readable PHAR/TAR payload".to_owned())
}

fn read_limited(reader: &mut impl Read) -> Result<Vec<u8>, String> {
    let mut content = Vec::new();
    reader
        .take(MAXIMUM_MANIFEST_BYTES + 1)
        .read_to_end(&mut content)
        .map_err(|error| format!("failed to read plugin manifest: {error}"))?;
    if content.len() as u64 > MAXIMUM_MANIFEST_BYTES {
        return Err("plugin manifest exceeds the 64 KiB limit".to_owned());
    }
    Ok(content)
}

fn parse_manifest(content: &[u8]) -> Result<PluginManifest, String> {
    let value: Value = serde_yaml::from_slice(content)
        .map_err(|error| format!("plugin.yml is not valid YAML: {error}"))?;
    let mapping = value
        .as_mapping()
        .ok_or_else(|| "plugin.yml root must be a mapping".to_owned())?;
    for field in ["name", "main", "version"] {
        let value = mapping
            .get(Value::String(field.to_owned()))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty());
        if value.is_none() {
            return Err(format!("plugin.yml requires a non-empty {field} field"));
        }
    }

    let api = mapping
        .get(Value::String("api".to_owned()))
        .ok_or_else(|| "plugin.yml requires an api field".to_owned())?;
    let api_versions = match api {
        Value::String(value) if !value.trim().is_empty() => vec![value.trim().to_owned()],
        Value::Sequence(values) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| value.trim().to_owned())
                    .ok_or_else(|| "plugin.yml api entries must be non-empty strings".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("plugin.yml api must be a string or string list".to_owned()),
    };
    if api_versions.is_empty() {
        return Err("plugin.yml api must not be empty".to_owned());
    }

    Ok(PluginManifest { api_versions })
}

struct PluginManifest {
    api_versions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

    use nexus_domain::InstanceKind;
    use tar::Builder;
    use tar::Header;
    use tempfile::tempdir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::validate_artifact;

    #[test]
    fn validates_a_nukkit_zip_manifest_and_api_target() {
        let directory = tempdir().expect("temporary directory is created");
        let path = directory.path().join("example.jar");
        let file = fs::File::create(&path).expect("plugin archive is created");
        let mut archive = ZipWriter::new(file);
        archive
            .start_file("plugin.yml", SimpleFileOptions::default())
            .expect("manifest entry is created");
        archive
            .write_all(
                br#"name: Example
main: example.Main
version: 1.0.0
api:
  - 1.0.0
"#,
            )
            .expect("manifest is written");
        archive.finish().expect("plugin archive is finished");

        validate_artifact(
            &path,
            "example.jar",
            InstanceKind::Nukkit,
            Some(&["1.0.0".to_owned()]),
        )
        .expect("matching Nukkit API is accepted");
        assert!(
            validate_artifact(
                &path,
                "example.jar",
                InstanceKind::Nukkit,
                Some(&["2.0.0".to_owned()])
            )
            .is_err()
        );
    }

    #[test]
    fn validates_a_pocketmine_phar_manifest() {
        let directory = tempdir().expect("temporary directory is created");
        let path = directory.path().join("example.phar");
        let file = fs::File::create(&path).expect("plugin archive is created");
        let mut archive = Builder::new(file);
        let content = br#"name: Example
main: example\Main
version: 1.0.0
api: 5.0.0
"#;
        let mut header = Header::new_gnu();
        header
            .set_path("plugin.yml")
            .expect("manifest path is valid");
        header.set_size(content.len() as u64);
        header.set_cksum();
        archive
            .append(&header, &content[..])
            .expect("manifest is written");
        archive.finish().expect("plugin archive is finished");

        validate_artifact(&path, "example.phar", InstanceKind::PocketMineMp, None)
            .expect("PocketMine manifest is accepted");
    }

    #[test]
    fn skips_manifest_validation_for_java_extensions() {
        let directory = tempdir().expect("temporary directory is created");
        let path = directory.path().join("example.jar");
        fs::write(&path, b"not a real archive").expect("test file is written");

        validate_artifact(&path, "example.jar", InstanceKind::Paper, None)
            .expect("Java extensions use their existing compatibility path");
    }
}
