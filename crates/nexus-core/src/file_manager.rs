use std::fs;
use std::fs::File;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use nexus_domain::FileContent;
use nexus_domain::FileEntry;
use nexus_domain::FileKind;
use nexus_domain::FilePage;
use nexus_domain::Instance;
use sha2::Digest;
use sha2::Sha256;
use tempfile::NamedTempFile;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::FileManagerError;

pub const MAXIMUM_FILE_READ_BYTES: usize = 32 * 1024;
pub const MAXIMUM_FILE_WRITE_BYTES: usize = 1024 * 1024;
const DEFAULT_FILE_LIST_LIMIT: usize = 50;
const MAXIMUM_FILE_LIST_LIMIT: usize = 200;

#[derive(Clone)]
pub struct FileManager {
    data_directory: Arc<PathBuf>,
}

impl FileManager {
    #[must_use]
    pub fn new(data_directory: &Path) -> Self {
        Self {
            data_directory: Arc::new(data_directory.to_path_buf()),
        }
    }

    pub fn list(
        &self,
        instance: &Instance,
        relative_path: &str,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<FilePage, FileManagerError> {
        let root = self.instance_root(instance)?;
        let directory = self.resolve_existing(&root, relative_path, true)?;
        let metadata = fs::symlink_metadata(&directory).map_err(|source| FileManagerError::Io {
            operation: "read metadata for",
            path: directory.clone(),
            source,
        })?;
        if !metadata.is_dir() {
            return Err(FileManagerError::NotDirectory { path: directory });
        }

        let mut entries = fs::read_dir(&directory)
            .map_err(|source| FileManagerError::Io {
                operation: "read directory",
                path: directory.clone(),
                source,
            })?
            .map(|entry| {
                let entry = entry.map_err(|source| FileManagerError::Io {
                    operation: "read directory entry",
                    path: directory.clone(),
                    source,
                })?;
                let path = entry.path();
                let relative_path = relative_file_path(&root, &path)?;
                file_entry(&path, relative_path)
            })
            .collect::<Result<Vec<_>, FileManagerError>>()?;
        entries.sort_by(|left, right| left.path().cmp(right.path()));

        let limit = limit.unwrap_or(DEFAULT_FILE_LIST_LIMIT);
        if !(1..=MAXIMUM_FILE_LIST_LIMIT).contains(&limit) {
            return Err(FileManagerError::InvalidPath {
                path: format!("limit={limit}"),
            });
        }
        let start_index = cursor.map_or(0, |cursor| {
            entries.partition_point(|entry| entry.path() <= cursor)
        });
        let has_more = entries.len().saturating_sub(start_index) > limit;
        let page = entries
            .into_iter()
            .skip(start_index)
            .take(limit)
            .collect::<Vec<_>>();
        let next_cursor = if has_more {
            page.last().map(|entry| entry.path().to_owned())
        } else {
            None
        };

        Ok(FilePage::new(page, next_cursor))
    }

    pub fn read(
        &self,
        instance: &Instance,
        relative_path: &str,
        offset: u64,
        length: usize,
    ) -> Result<FileContent, FileManagerError> {
        if length > MAXIMUM_FILE_READ_BYTES {
            return Err(FileManagerError::ContentTooLarge {
                maximum_bytes: MAXIMUM_FILE_READ_BYTES,
            });
        }
        let root = self.instance_root(instance)?;
        let path = self.resolve_existing(&root, relative_path, false)?;
        let metadata = fs::metadata(&path).map_err(|source| FileManagerError::Io {
            operation: "read metadata for",
            path: path.clone(),
            source,
        })?;
        if !metadata.is_file() {
            return Err(FileManagerError::NotFile { path });
        }

        let sha256 = hash_file(&path)?;
        let mut file = File::open(&path).map_err(|source| FileManagerError::Io {
            operation: "open",
            path: path.clone(),
            source,
        })?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|source| FileManagerError::Io {
                operation: "seek",
                path: path.clone(),
                source,
            })?;
        let mut buffer = vec![0_u8; length.saturating_add(1)];
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|source| FileManagerError::Io {
                operation: "read",
                path,
                source,
            })?;
        let eof = bytes_read <= length;
        buffer.truncate(bytes_read.min(length));

        Ok(FileContent::new(STANDARD.encode(buffer), sha256, eof))
    }

    pub fn write(
        &self,
        instance: &Instance,
        relative_path: &str,
        content: &[u8],
        expected_sha256: Option<&str>,
    ) -> Result<FileEntry, FileManagerError> {
        if content.len() > MAXIMUM_FILE_WRITE_BYTES {
            return Err(FileManagerError::ContentTooLarge {
                maximum_bytes: MAXIMUM_FILE_WRITE_BYTES,
            });
        }
        let root = self.instance_root(instance)?;
        let path = self.resolve_write_target(&root, relative_path)?;
        if let Some(expected_sha256) = expected_sha256 {
            validate_hash(expected_sha256)?;
            let actual_sha256 = if path.exists() {
                hash_file(&path)?
            } else {
                String::new()
            };
            if !expected_sha256.eq_ignore_ascii_case(&actual_sha256) {
                return Err(FileManagerError::HashMismatch {
                    expected: expected_sha256.to_owned(),
                    actual: actual_sha256,
                });
            }
        }

        let parent = path.parent().ok_or_else(|| FileManagerError::InvalidPath {
            path: relative_path.to_owned(),
        })?;
        let parent = fs::canonicalize(parent).map_err(|source| {
            FileManagerError::CanonicalizeInstanceDirectory {
                path: parent.to_path_buf(),
                source,
            }
        })?;
        if !parent.starts_with(&root) {
            return Err(FileManagerError::PathEscapes { path: parent });
        }

        let mut temporary =
            NamedTempFile::new_in(&parent).map_err(|source| FileManagerError::Io {
                operation: "create temporary file in",
                path: parent.clone(),
                source,
            })?;
        temporary
            .write_all(content)
            .and_then(|_| temporary.flush())
            .and_then(|_| temporary.as_file().sync_all())
            .map_err(|source| FileManagerError::Io {
                operation: "write",
                path: path.clone(),
                source,
            })?;
        temporary
            .persist(&path)
            .map_err(|error| FileManagerError::Io {
                operation: "replace",
                path: path.clone(),
                source: error.error,
            })?;

        let hash = sha256_hex(Sha256::digest(content));
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| FileManagerError::NonUtf8Path { path: path.clone() })?;
        let relative_path = relative_file_path(&root, &path)?;
        Ok(FileEntry::new(
            name.to_owned(),
            relative_path,
            FileKind::File,
            content.len() as u64,
            current_timestamp(),
            Some(hash),
        ))
    }

    fn instance_root(&self, instance: &Instance) -> Result<PathBuf, FileManagerError> {
        let data_directory = fs::canonicalize(self.data_directory.as_ref()).map_err(|source| {
            FileManagerError::CanonicalizeDataDirectory {
                path: self.data_directory.as_ref().clone(),
                source,
            }
        })?;
        let instance_directory = data_directory.join(instance.directory());
        fs::create_dir_all(&instance_directory).map_err(|source| {
            FileManagerError::CreateInstanceDirectory {
                path: instance_directory.clone(),
                source,
            }
        })?;
        let instance_directory = fs::canonicalize(&instance_directory).map_err(|source| {
            FileManagerError::CanonicalizeInstanceDirectory {
                path: instance_directory,
                source,
            }
        })?;
        if !instance_directory.starts_with(&data_directory) {
            return Err(FileManagerError::PathEscapes {
                path: instance_directory,
            });
        }

        Ok(instance_directory)
    }

    fn resolve_existing(
        &self,
        root: &Path,
        relative: &str,
        allow_root: bool,
    ) -> Result<PathBuf, FileManagerError> {
        let relative_path = parse_relative_path(relative, allow_root)?;
        let path = root.join(relative_path);
        fs::symlink_metadata(&path).map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                FileManagerError::NotFound { path: path.clone() }
            } else {
                FileManagerError::Io {
                    operation: "read metadata for",
                    path: path.clone(),
                    source: error,
                }
            }
        })?;
        let canonical = fs::canonicalize(&path).map_err(|source| FileManagerError::Io {
            operation: "resolve",
            path: path.clone(),
            source,
        })?;
        if !canonical.starts_with(root) {
            return Err(FileManagerError::PathEscapes { path: canonical });
        }

        Ok(canonical)
    }

    fn resolve_write_target(
        &self,
        root: &Path,
        relative: &str,
    ) -> Result<PathBuf, FileManagerError> {
        let relative_path = parse_relative_path(relative, false)?;
        let path = root.join(relative_path);
        if let Ok(metadata) = fs::symlink_metadata(&path) {
            if metadata.file_type().is_symlink() {
                return Err(FileManagerError::SymlinkNotAllowed { path });
            }
            if metadata.is_dir() {
                return Err(FileManagerError::NotFile { path });
            }
        }
        let parent = path.parent().ok_or_else(|| FileManagerError::InvalidPath {
            path: relative.to_owned(),
        })?;
        let canonical_parent = fs::canonicalize(parent).map_err(|source| {
            if source.kind() == ErrorKind::NotFound {
                FileManagerError::NotFound {
                    path: parent.to_path_buf(),
                }
            } else {
                FileManagerError::Io {
                    operation: "resolve parent directory",
                    path: parent.to_path_buf(),
                    source,
                }
            }
        })?;
        if !canonical_parent.starts_with(root) {
            return Err(FileManagerError::PathEscapes {
                path: canonical_parent,
            });
        }

        Ok(path)
    }
}

fn parse_relative_path(value: &str, allow_root: bool) -> Result<PathBuf, FileManagerError> {
    if value.is_empty() && allow_root {
        return Ok(PathBuf::new());
    }
    if value.is_empty()
        || value.contains('\0')
        || value.contains('\\')
        || value.starts_with('/')
        || value.contains(':')
    {
        return Err(FileManagerError::InvalidPath {
            path: value.to_owned(),
        });
    }

    let mut path = PathBuf::new();
    for component in value.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(FileManagerError::InvalidPath {
                path: value.to_owned(),
            });
        }
        path.push(component);
    }

    Ok(path)
}

fn relative_file_path(root: &Path, path: &Path) -> Result<String, FileManagerError> {
    path.strip_prefix(root)
        .map_err(|_| FileManagerError::PathEscapes {
            path: path.to_path_buf(),
        })?
        .components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .ok_or_else(|| FileManagerError::NonUtf8Path {
                    path: path.to_path_buf(),
                })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|components| components.join("/"))
}

fn file_entry(path: &Path, relative_path: String) -> Result<FileEntry, FileManagerError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| FileManagerError::Io {
        operation: "read metadata for",
        path: path.to_path_buf(),
        source,
    })?;
    let file_type = metadata.file_type();
    let kind = if file_type.is_file() {
        FileKind::File
    } else if file_type.is_dir() {
        FileKind::Directory
    } else if file_type.is_symlink() {
        FileKind::Symlink
    } else {
        FileKind::Other
    };
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| FileManagerError::NonUtf8Path {
            path: path.to_path_buf(),
        })?;
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|modified| OffsetDateTime::from(modified).format(&Rfc3339).ok())
        .unwrap_or_else(current_timestamp);

    Ok(FileEntry::new(
        name.to_owned(),
        relative_path,
        kind,
        metadata.len(),
        modified_at,
        None,
    ))
}

fn hash_file(path: &Path) -> Result<String, FileManagerError> {
    let mut file = File::open(path).map_err(|source| FileManagerError::Io {
        operation: "open",
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|source| FileManagerError::Io {
                operation: "hash",
                path: path.to_path_buf(),
                source,
            })?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(sha256_hex(hasher.finalize()))
}

fn sha256_hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_hash(value: &str) -> Result<(), FileManagerError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(FileManagerError::InvalidHash {
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use nexus_domain::FileKind;
    use nexus_domain::Instance;
    use nexus_domain::InstanceCreate;
    use nexus_domain::InstanceId;
    use nexus_domain::InstanceKind;
    use nexus_domain::LaunchConfig;
    use tempfile::tempdir;

    use super::FileManager;
    use super::FileManagerError;

    #[test]
    fn lists_entries_with_safe_relative_paths() {
        let directory = tempdir().expect("temporary directory is created");
        let instance = instance();
        let instance_directory = directory.path().join(instance.directory());
        fs::create_dir_all(&instance_directory).expect("instance directory is created");
        fs::write(instance_directory.join("server.properties"), b"motd=MCNP")
            .expect("test file is written");
        fs::create_dir(instance_directory.join("plugins")).expect("directory is created");

        let page = FileManager::new(directory.path())
            .list(&instance, "", None, None)
            .expect("instance entries are listed");

        assert_eq!(page.items().len(), 2);
        assert_eq!(page.items()[0].kind(), FileKind::Directory);
        assert_eq!(page.items()[0].path(), "plugins");
        assert_eq!(page.items()[1].path(), "server.properties");
    }

    #[test]
    fn reads_chunks_and_rejects_parent_paths() {
        let directory = tempdir().expect("temporary directory is created");
        let instance = instance();
        let instance_directory = directory.path().join(instance.directory());
        fs::create_dir_all(&instance_directory).expect("instance directory is created");
        fs::write(instance_directory.join("server.properties"), b"motd=MCNP")
            .expect("test file is written");
        let manager = FileManager::new(directory.path());

        let content = manager
            .read(&instance, "server.properties", 0, 4)
            .expect("file chunk is read");
        assert_eq!(content.data_base64(), "bW90ZA==");
        assert_eq!(content.sha256().len(), 64);
        assert!(!content.eof());
        assert!(matches!(
            manager.read(&instance, "../outside", 0, 1),
            Err(FileManagerError::InvalidPath { .. })
        ));
    }

    #[test]
    fn writes_atomically_and_checks_the_previous_hash() {
        let directory = tempdir().expect("temporary directory is created");
        let instance = instance();
        let instance_directory = directory.path().join(instance.directory());
        fs::create_dir_all(&instance_directory).expect("instance directory is created");
        let manager = FileManager::new(directory.path());

        let entry = manager
            .write(&instance, "server.properties", b"motd=MCNP", None)
            .expect("file is written");
        let hash = entry.sha256().expect("written file has a hash").to_owned();
        assert_eq!(entry.kind(), FileKind::File);
        assert_eq!(
            fs::read(instance_directory.join("server.properties"))
                .expect("written file is readable"),
            b"motd=MCNP"
        );

        let stale_hash = "0".repeat(64);
        let error = manager
            .write(
                &instance,
                "server.properties",
                b"motd=Changed",
                Some(&stale_hash),
            )
            .expect_err("stale hash is rejected");
        assert!(matches!(error, FileManagerError::HashMismatch { .. }));
        manager
            .write(&instance, "server.properties", b"motd=Changed", Some(&hash))
            .expect("matching hash permits replacement");
    }

    fn instance() -> Instance {
        InstanceCreate::new(
            InstanceId::new("survival".to_owned()).expect("instance ID is valid"),
            "Survival".to_owned(),
            InstanceKind::Paper,
            "instances/survival".to_owned(),
            LaunchConfig::new(
                "java".to_owned(),
                Vec::new(),
                BTreeMap::new(),
                "stop".to_owned(),
                30,
            ),
        )
        .expect("instance is valid")
        .into_instance()
        .expect("instance is created")
    }
}
