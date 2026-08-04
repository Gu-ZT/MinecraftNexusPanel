//! Core 对系统 PATH 和受管目录中的运行时发现与版本校验。
//!
//! 发现过程只执行版本探测，不会安装或删除系统运行时；带有安装清单的目录才会
//! 被识别为可由 MCNP 管理的运行时。

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use nexus_domain::ManagedRuntime;
use nexus_domain::RuntimeInstallManifest;
use nexus_domain::RuntimeKind;
use nexus_domain::RuntimeSource;
use nexus_domain::RuntimeValidation;
use tokio::process::Command;
use tokio::time::timeout;

const VERSION_CHECK_TIMEOUT: Duration = Duration::from_secs(3);

/// 扫描并验证 Java、Node.js 和 Python 运行时的内部服务。
#[derive(Clone)]
pub(crate) struct RuntimeDiscovery {
    managed_root: PathBuf,
}

impl RuntimeDiscovery {
    /// 创建指向 Core 受管运行时根目录的发现服务。
    pub(crate) fn new(data_directory: &Path) -> Self {
        Self {
            managed_root: data_directory.join("runtimes"),
        }
    }

    /// 扫描受管目录和系统 PATH，并验证每个候选运行时版本。
    pub(crate) async fn discover(&self) -> Vec<ManagedRuntime> {
        let mut runtimes = Vec::new();
        for candidate in self.candidates() {
            runtimes.push(validate_candidate(candidate).await);
        }

        runtimes
    }

    /// 返回受管运行时根目录。
    pub(crate) fn managed_root(&self) -> &Path {
        &self.managed_root
    }

    /// 按稳定运行时标识查找并验证受管运行时。
    pub(crate) async fn find_managed(&self, runtime_id: &str) -> Option<ManagedRuntime> {
        self.discover()
            .await
            .into_iter()
            .find(|runtime| runtime.runtime_id() == Some(runtime_id))
    }

    /// 查找受管运行时目录，不执行可执行文件版本校验。
    pub(crate) fn find_managed_path(&self, runtime_id: &str) -> Option<PathBuf> {
        for kind in RuntimeKind::ALL {
            let path = self
                .managed_root
                .join(kind_directory(kind))
                .join(runtime_id);
            if path.is_dir() {
                return Some(path);
            }
        }
        None
    }

    fn candidates(&self) -> Vec<RuntimeCandidate> {
        let mut candidates = self.managed_candidates();
        candidates.extend(system_candidates());
        candidates.sort_by(|left, right| {
            (left.kind, left.source, &left.executable).cmp(&(
                right.kind,
                right.source,
                &right.executable,
            ))
        });
        candidates
            .dedup_by(|left, right| left.kind == right.kind && left.executable == right.executable);

        candidates
    }

    fn managed_candidates(&self) -> Vec<RuntimeCandidate> {
        let mut candidates = Vec::new();
        for kind in RuntimeKind::ALL {
            let directory = self.managed_root.join(kind_directory(kind));
            let Ok(versions) = fs::read_dir(directory) else {
                continue;
            };
            for version in versions.flatten() {
                let Ok(file_type) = version.file_type() else {
                    continue;
                };
                if !file_type.is_dir() {
                    continue;
                }
                if let Some(manifest) = read_manifest(&version.path()) {
                    candidates.push(RuntimeCandidate {
                        kind,
                        source: RuntimeSource::Managed,
                        runtime_id: Some(manifest.runtime_id().to_owned()),
                        distribution: Some(manifest.distribution().to_owned()),
                        executable: normalize_path(version.path().join(manifest.executable_path())),
                    });
                    continue;
                }
                for executable_name in executable_names(kind) {
                    let executable = version.path().join("bin").join(executable_name);
                    if executable.is_file() {
                        candidates.push(RuntimeCandidate {
                            kind,
                            source: RuntimeSource::Managed,
                            runtime_id: Some(version.file_name().to_string_lossy().into_owned()),
                            distribution: None,
                            executable: normalize_path(executable),
                        });
                    }
                }
            }
        }

        candidates
    }
}

/// 尚未执行版本探测的运行时候选项。
#[derive(Eq, PartialEq)]
struct RuntimeCandidate {
    kind: RuntimeKind,
    source: RuntimeSource,
    runtime_id: Option<String>,
    distribution: Option<String>,
    executable: PathBuf,
}

async fn validate_candidate(candidate: RuntimeCandidate) -> ManagedRuntime {
    let mut command = Command::new(&candidate.executable);
    command.arg(version_argument(candidate.kind));
    let version = match timeout(VERSION_CHECK_TIMEOUT, command.output()).await {
        Ok(Ok(output)) if output.status.success() => {
            let mut text = output.stdout;
            text.extend_from_slice(&output.stderr);
            parse_version(candidate.kind, &text)
        }
        Ok(Ok(_)) | Ok(Err(_)) | Err(_) => None,
    };
    let validation = if version.is_some() {
        RuntimeValidation::Valid
    } else {
        RuntimeValidation::Invalid
    };

    match (candidate.runtime_id, candidate.distribution) {
        (Some(runtime_id), Some(distribution)) => ManagedRuntime::managed(
            runtime_id,
            candidate.kind,
            distribution,
            candidate.executable.to_string_lossy().into_owned(),
            version,
            validation,
        ),
        _ => ManagedRuntime::new(
            candidate.kind,
            candidate.source,
            candidate.executable.to_string_lossy().into_owned(),
            version,
            validation,
        ),
    }
}

fn system_candidates() -> Vec<RuntimeCandidate> {
    let Some(path) = env::var_os("PATH") else {
        return Vec::new();
    };
    let mut candidates = Vec::new();
    for directory in env::split_paths(&path) {
        if directory.as_os_str().is_empty() {
            continue;
        }
        for kind in RuntimeKind::ALL {
            for executable_name in executable_names(kind) {
                let executable = directory.join(executable_name);
                if executable.is_file() {
                    candidates.push(RuntimeCandidate {
                        kind,
                        source: RuntimeSource::System,
                        runtime_id: None,
                        distribution: None,
                        executable: normalize_path(executable),
                    });
                }
            }
        }
    }

    candidates
}

fn read_manifest(path: &Path) -> Option<RuntimeInstallManifest> {
    let manifest_path = path.join(".mcnp-runtime.json");
    let bytes = fs::read(manifest_path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn normalize_path(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn kind_directory(kind: RuntimeKind) -> &'static str {
    match kind {
        RuntimeKind::Java => "java",
        RuntimeKind::NodeJs => "node",
        RuntimeKind::Python => "python",
    }
}

fn version_argument(kind: RuntimeKind) -> &'static str {
    match kind {
        RuntimeKind::Java => "-version",
        RuntimeKind::NodeJs | RuntimeKind::Python => "--version",
    }
}

#[cfg(windows)]
fn executable_names(kind: RuntimeKind) -> &'static [&'static str] {
    match kind {
        RuntimeKind::Java => &["java.exe"],
        RuntimeKind::NodeJs => &["node.exe"],
        RuntimeKind::Python => &["python.exe", "python3.exe"],
    }
}

#[cfg(not(windows))]
fn executable_names(kind: RuntimeKind) -> &'static [&'static str] {
    match kind {
        RuntimeKind::Java => &["java"],
        RuntimeKind::NodeJs => &["node"],
        RuntimeKind::Python => &["python3", "python"],
    }
}

fn parse_version(kind: RuntimeKind, output: &[u8]) -> Option<String> {
    let output = String::from_utf8_lossy(output);
    output
        .split(|character: char| {
            !character.is_ascii_alphanumeric() && !matches!(character, '.' | '_' | '-' | '+')
        })
        .map(|token| {
            if kind == RuntimeKind::NodeJs {
                token.strip_prefix('v').unwrap_or(token)
            } else {
                token
            }
        })
        .find(|token| is_version_token(token))
        .map(str::to_owned)
}

fn is_version_token(token: &str) -> bool {
    token.as_bytes().first().is_some_and(u8::is_ascii_digit)
        && token.chars().all(|character| {
            character.is_ascii_digit() || matches!(character, '.' | '_' | '-' | '+')
        })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::RuntimeDiscovery;
    use super::executable_names;
    use super::kind_directory;
    use super::normalize_path;
    use super::parse_version;
    use nexus_domain::RuntimeKind;
    use nexus_domain::RuntimeSource;
    use tempfile::tempdir;

    #[test]
    fn discovers_runtime_files_from_the_managed_layout() {
        let data_directory = tempdir().expect("temporary Core data directory is created");
        let executable = data_directory
            .path()
            .join("runtimes")
            .join(kind_directory(RuntimeKind::Java))
            .join("21")
            .join("bin")
            .join(executable_names(RuntimeKind::Java)[0]);
        fs::create_dir_all(
            executable
                .parent()
                .expect("runtime executable has a parent"),
        )
        .expect("managed runtime directory is created");
        fs::write(&executable, []).expect("runtime executable marker is written");

        let candidates = RuntimeDiscovery::new(data_directory.path()).managed_candidates();

        assert!(candidates.iter().any(|candidate| {
            candidate.kind == RuntimeKind::Java
                && candidate.source == RuntimeSource::Managed
                && candidate.executable == normalize_path(executable.clone())
        }));
    }

    #[test]
    fn parses_java_node_and_python_versions() {
        assert_eq!(
            parse_version(RuntimeKind::Java, b"openjdk version \"21.0.5\" 2024-10-15"),
            Some("21.0.5".to_owned())
        );
        assert_eq!(
            parse_version(RuntimeKind::NodeJs, b"v22.14.0"),
            Some("22.14.0".to_owned())
        );
        assert_eq!(
            parse_version(RuntimeKind::Python, b"Python 3.13.1"),
            Some("3.13.1".to_owned())
        );
    }
}
