use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::io::copy;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use nexus_domain::FileContent;
use nexus_domain::FileEntry;
use nexus_domain::FileKind;
use nexus_domain::FilePage;
use nexus_domain::Instance;
use nexus_domain::TaskId;
use serde_json::Value;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;
use tempfile::NamedTempFile;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::spawn;
use tokio::task::spawn_blocking;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::FileBatchOperation;
use crate::FileManagerError;
use crate::file_download::FileDownload;
use crate::file_upload::FileUpload;

pub const MAXIMUM_FILE_READ_BYTES: usize = 32 * 1024;
pub const MAXIMUM_FILE_WRITE_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_FILE_BATCH_OPERATIONS: usize = 64;
pub const MAXIMUM_FILE_ARCHIVE_PATHS: usize = 128;
pub const FILE_TRANSFER_CHUNK_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_FILE_TRANSFER_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const MAXIMUM_FILE_ARCHIVE_ENTRIES: usize = 16 * 1024;
pub const MAXIMUM_FILE_ARCHIVE_BYTES: u64 = MAXIMUM_FILE_TRANSFER_BYTES;
const MAXIMUM_ACTIVE_FILE_TRANSFERS: usize = 16;
const DEFAULT_FILE_LIST_LIMIT: usize = 50;
const MAXIMUM_FILE_LIST_LIMIT: usize = 200;

#[derive(Clone)]
pub struct FileManager {
    data_directory: Arc<PathBuf>,
    tasks: Arc<Mutex<HashMap<TaskId, Value>>>,
    uploads: Arc<Mutex<HashMap<TaskId, FileUpload>>>,
    downloads: Arc<Mutex<HashMap<TaskId, FileDownload>>>,
}

impl FileManager {
    #[must_use]
    pub fn new(data_directory: &Path) -> Self {
        Self {
            data_directory: Arc::new(data_directory.to_path_buf()),
            tasks: Arc::new(Mutex::new(HashMap::new())),
            uploads: Arc::new(Mutex::new(HashMap::new())),
            downloads: Arc::new(Mutex::new(HashMap::new())),
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

    pub fn mkdir(
        &self,
        instance: &Instance,
        relative_path: &str,
        recursive: bool,
    ) -> Result<FileEntry, FileManagerError> {
        let root = self.instance_root(instance)?;
        let relative = parse_relative_path(relative_path, false)?;
        let target = root.join(&relative);
        let mut current = root.clone();

        for component in relative.components() {
            current.push(component.as_os_str());
            match fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(FileManagerError::SymlinkNotAllowed { path: current });
                }
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => return Err(FileManagerError::NotDirectory { path: current }),
                Err(error) if error.kind() == ErrorKind::NotFound && recursive => {
                    fs::create_dir(&current).map_err(|source| FileManagerError::Io {
                        operation: "create directory",
                        path: current.clone(),
                        source,
                    })?;
                }
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    return Err(FileManagerError::NotFound {
                        path: current.parent().unwrap_or(&root).to_path_buf(),
                    });
                }
                Err(source) => {
                    return Err(FileManagerError::Io {
                        operation: "read directory metadata for",
                        path: current,
                        source,
                    });
                }
            }
        }

        let canonical = fs::canonicalize(&target).map_err(|source| FileManagerError::Io {
            operation: "resolve created directory",
            path: target.clone(),
            source,
        })?;
        if !canonical.starts_with(&root) {
            return Err(FileManagerError::PathEscapes { path: canonical });
        }

        relative_file_entry(&root, &target)
    }

    pub fn move_entry(
        &self,
        instance: &Instance,
        from: &str,
        to: &str,
        overwrite: bool,
    ) -> Result<FileEntry, FileManagerError> {
        let root = self.instance_root(instance)?;
        let source_relative = parse_relative_path(from, false)?;
        let target_relative = parse_relative_path(to, false)?;
        let source = root.join(&source_relative);
        let target = root.join(&target_relative);
        if source == target {
            return relative_file_entry(&root, &source);
        }

        let source_metadata = fs::symlink_metadata(&source).map_err(|source_error| {
            if source_error.kind() == ErrorKind::NotFound {
                FileManagerError::NotFound {
                    path: source.clone(),
                }
            } else {
                FileManagerError::Io {
                    operation: "read source metadata for",
                    path: source.clone(),
                    source: source_error,
                }
            }
        })?;
        if source_metadata.file_type().is_symlink() {
            return Err(FileManagerError::SymlinkNotAllowed { path: source });
        }
        let source_canonical =
            fs::canonicalize(&source).map_err(|source_error| FileManagerError::Io {
                operation: "resolve source",
                path: source.clone(),
                source: source_error,
            })?;
        if !source_canonical.starts_with(&root) {
            return Err(FileManagerError::PathEscapes {
                path: source_canonical,
            });
        }

        let parent = target
            .parent()
            .ok_or_else(|| FileManagerError::InvalidPath {
                path: to.to_owned(),
            })?;
        let canonical_parent = fs::canonicalize(parent).map_err(|source_error| {
            if source_error.kind() == ErrorKind::NotFound {
                FileManagerError::NotFound {
                    path: parent.to_path_buf(),
                }
            } else {
                FileManagerError::Io {
                    operation: "resolve target directory",
                    path: parent.to_path_buf(),
                    source: source_error,
                }
            }
        })?;
        if !canonical_parent.starts_with(&root) {
            return Err(FileManagerError::PathEscapes {
                path: canonical_parent,
            });
        }
        if !fs::metadata(parent)
            .map_err(|source_error| FileManagerError::Io {
                operation: "read target directory metadata for",
                path: parent.to_path_buf(),
                source: source_error,
            })?
            .is_dir()
        {
            return Err(FileManagerError::NotDirectory {
                path: parent.to_path_buf(),
            });
        }
        if canonical_parent.starts_with(&source_canonical) {
            return Err(FileManagerError::InvalidPath {
                path: to.to_owned(),
            });
        }

        if let Ok(target_metadata) = fs::symlink_metadata(&target) {
            if target_metadata.file_type().is_symlink() {
                return Err(FileManagerError::SymlinkNotAllowed { path: target });
            }
            if !overwrite {
                return Err(FileManagerError::AlreadyExists { path: target });
            }
            if source_metadata.is_dir() != target_metadata.is_dir() {
                return Err(FileManagerError::AlreadyExists { path: target });
            }
            if target_metadata.is_dir() {
                if fs::read_dir(&target)
                    .map_err(|source| FileManagerError::Io {
                        operation: "read target directory",
                        path: target.clone(),
                        source,
                    })?
                    .next()
                    .is_some()
                {
                    return Err(FileManagerError::DirectoryNotEmpty { path: target });
                }
                fs::remove_dir(&target).map_err(|source| FileManagerError::Io {
                    operation: "remove target directory",
                    path: target.clone(),
                    source,
                })?;
            } else {
                fs::remove_file(&target).map_err(|source| FileManagerError::Io {
                    operation: "remove target file",
                    path: target.clone(),
                    source,
                })?;
            }
        }

        fs::rename(&source, &target).map_err(|source_error| FileManagerError::Io {
            operation: "move",
            path: target.clone(),
            source: source_error,
        })?;

        relative_file_entry(&root, &target)
    }

    pub fn start_delete(
        &self,
        instance: &Instance,
        relative_path: &str,
        recursive: bool,
    ) -> Result<TaskId, FileManagerError> {
        self.validate_delete_target(instance, relative_path, recursive)?;
        let task_id = TaskId::new();
        {
            let mut tasks = self
                .tasks
                .lock()
                .map_err(|_| FileManagerError::TaskStorePoisoned)?;
            if tasks.len() >= 512 {
                tasks.clear();
            }
            tasks.insert(
                task_id,
                json!({
                    "taskId": task_id,
                    "kind": "FILE_DELETE",
                    "state": "RUNNING",
                    "progress": null,
                }),
            );
        }

        let manager = self.clone();
        let worker = manager.clone();
        let instance = instance.clone();
        let relative_path = relative_path.to_owned();
        let task_path = relative_path.clone();
        spawn(async move {
            let result =
                spawn_blocking(move || worker.delete_sync(&instance, &relative_path, recursive))
                    .await;
            let result = match result {
                Ok(Ok(())) => Ok(json!({ "path": task_path, "deleted": true })),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            manager.finish_delete_task(task_id, result);
        });

        Ok(task_id)
    }

    pub(crate) fn start_archive(
        &self,
        instance: &Instance,
        paths: Vec<String>,
        output_path: String,
    ) -> Result<TaskId, FileManagerError> {
        if paths.is_empty() || paths.len() > MAXIMUM_FILE_ARCHIVE_PATHS {
            return Err(FileManagerError::InvalidPath {
                path: format!("paths={}", paths.len()),
            });
        }
        let root = self.instance_root(instance)?;
        let target = self.resolve_write_target(&root, &output_path)?;
        if paths.iter().any(|path| path == &output_path) {
            return Err(FileManagerError::InvalidPath {
                path: output_path.clone(),
            });
        }
        let entries = collect_archive_entries(&root, &paths, &output_path)?;
        let total = entries.len().max(1);
        let task_id = TaskId::new();
        {
            let mut tasks = self
                .tasks
                .lock()
                .map_err(|_| FileManagerError::TaskStorePoisoned)?;
            if tasks.len() >= 512 {
                tasks.clear();
            }
            tasks.insert(
                task_id,
                json!({
                    "taskId": task_id,
                    "kind": "FILE_ARCHIVE_CREATE",
                    "state": "RUNNING",
                    "progress": { "completed": 0, "total": total },
                }),
            );
        }

        let manager = self.clone();
        let worker = manager.clone();
        spawn(async move {
            let result = spawn_blocking(move || {
                worker.create_archive(task_id, root, target, entries, total)
            })
            .await;
            match result {
                Ok(Ok(entry)) => manager.finish_archive_task(task_id, Ok(entry), total),
                Ok(Err(error)) => {
                    manager.finish_archive_task(task_id, Err(error.to_string()), total)
                }
                Err(error) => manager.finish_archive_task(task_id, Err(error.to_string()), total),
            }
        });

        Ok(task_id)
    }

    pub(crate) fn start_batch(
        &self,
        instance: &Instance,
        operations: Vec<FileBatchOperation>,
    ) -> Result<TaskId, FileManagerError> {
        if operations.is_empty() || operations.len() > MAXIMUM_FILE_BATCH_OPERATIONS {
            return Err(FileManagerError::InvalidPath {
                path: format!("operations={}", operations.len()),
            });
        }

        let task_id = TaskId::new();
        let total = operations.len();
        {
            let mut tasks = self
                .tasks
                .lock()
                .map_err(|_| FileManagerError::TaskStorePoisoned)?;
            if tasks.len() >= 512 {
                tasks.clear();
            }
            tasks.insert(
                task_id,
                json!({
                    "taskId": task_id,
                    "kind": "FILE_BATCH",
                    "state": "RUNNING",
                    "progress": { "completed": 0, "total": total },
                    "results": [],
                }),
            );
        }

        let manager = self.clone();
        let worker = manager.clone();
        let instance = instance.clone();
        spawn(async move {
            let result =
                spawn_blocking(move || worker.execute_batch(&instance, task_id, operations)).await;
            match result {
                Ok(Ok(results)) => manager.finish_batch_task(task_id, Ok(results)),
                Ok(Err((results, failed_index, error))) => {
                    manager.finish_batch_task(task_id, Err((results, failed_index, error)));
                }
                Err(error) => {
                    manager.finish_batch_task(task_id, Err((Vec::new(), 0, error.to_string())))
                }
            }
        });

        Ok(task_id)
    }

    pub(crate) fn begin_upload(
        &self,
        instance: &Instance,
        relative_path: &str,
        expected_size: u64,
        expected_sha256: &str,
    ) -> Result<Value, FileManagerError> {
        if expected_size > MAXIMUM_FILE_TRANSFER_BYTES {
            return Err(FileManagerError::ContentTooLarge {
                maximum_bytes: usize::try_from(MAXIMUM_FILE_TRANSFER_BYTES).unwrap_or(usize::MAX),
            });
        }
        validate_hash(expected_sha256)?;
        let root = self.instance_root(instance)?;
        let target = self.resolve_write_target(&root, relative_path)?;
        let parent = target
            .parent()
            .ok_or_else(|| FileManagerError::InvalidPath {
                path: relative_path.to_owned(),
            })?;
        let temporary = NamedTempFile::new_in(parent).map_err(|source| FileManagerError::Io {
            operation: "create upload temporary file in",
            path: parent.to_path_buf(),
            source,
        })?;
        let transfer_id = TaskId::new();
        let upload = FileUpload {
            root_path: root,
            target_path: target,
            temporary,
            expected_size,
            expected_sha256: expected_sha256.to_ascii_lowercase(),
            next_offset: 0,
        };
        let mut uploads = self
            .uploads
            .lock()
            .map_err(|_| FileManagerError::TaskStorePoisoned)?;
        if uploads.len() >= MAXIMUM_ACTIVE_FILE_TRANSFERS {
            return Err(FileManagerError::TooManyTransfers);
        }
        uploads.insert(transfer_id, upload);

        Ok(json!({
            "transferId": transfer_id,
            "chunkSize": FILE_TRANSFER_CHUNK_BYTES,
            "nextOffset": 0,
            "sizeBytes": expected_size,
        }))
    }

    pub(crate) fn begin_download(
        &self,
        instance: &Instance,
        relative_path: &str,
    ) -> Result<Value, FileManagerError> {
        let root = self.instance_root(instance)?;
        let relative = parse_relative_path(relative_path, false)?;
        let path = root.join(relative);
        let metadata = fs::symlink_metadata(&path).map_err(|source| {
            if source.kind() == ErrorKind::NotFound {
                FileManagerError::NotFound { path: path.clone() }
            } else {
                FileManagerError::Io {
                    operation: "read download source metadata for",
                    path: path.clone(),
                    source,
                }
            }
        })?;
        if metadata.file_type().is_symlink() {
            return Err(FileManagerError::SymlinkNotAllowed { path });
        }
        if !metadata.is_file() {
            return Err(FileManagerError::NotFile { path });
        }
        let source_path = fs::canonicalize(&path).map_err(|source| FileManagerError::Io {
            operation: "resolve download source",
            path: path.clone(),
            source,
        })?;
        if !source_path.starts_with(&root) {
            return Err(FileManagerError::PathEscapes { path: source_path });
        }
        let expected_sha256 = hash_file(&source_path)?;
        let transfer_id = TaskId::new();
        let mut downloads = self
            .downloads
            .lock()
            .map_err(|_| FileManagerError::TaskStorePoisoned)?;
        if downloads.len() >= MAXIMUM_ACTIVE_FILE_TRANSFERS {
            return Err(FileManagerError::TooManyTransfers);
        }
        downloads.insert(
            transfer_id,
            FileDownload {
                source_path,
                expected_size: metadata.len(),
                expected_sha256: expected_sha256.clone(),
                next_offset: 0,
            },
        );

        Ok(json!({
            "transferId": transfer_id,
            "chunkSize": FILE_TRANSFER_CHUNK_BYTES,
            "nextOffset": 0,
            "sizeBytes": metadata.len(),
            "sha256": expected_sha256,
        }))
    }

    pub(crate) fn write_upload_chunk(
        &self,
        transfer_id: TaskId,
        offset: u64,
        content: &[u8],
        expected_sha256: Option<&str>,
    ) -> Result<Value, FileManagerError> {
        if content.is_empty() {
            return Err(FileManagerError::InvalidPath {
                path: "empty transfer chunk".to_owned(),
            });
        }
        if content.len() > FILE_TRANSFER_CHUNK_BYTES {
            return Err(FileManagerError::TransferChunkTooLarge {
                maximum_bytes: FILE_TRANSFER_CHUNK_BYTES,
            });
        }
        if let Some(expected_sha256) = expected_sha256 {
            validate_hash(expected_sha256)?;
            let actual_sha256 = sha256_hex(Sha256::digest(content));
            if !expected_sha256.eq_ignore_ascii_case(&actual_sha256) {
                return Err(FileManagerError::TransferChunkHashMismatch {
                    expected: expected_sha256.to_owned(),
                    actual: actual_sha256,
                });
            }
        }

        let mut uploads = self
            .uploads
            .lock()
            .map_err(|_| FileManagerError::TaskStorePoisoned)?;
        let upload = uploads
            .get_mut(&transfer_id)
            .ok_or(FileManagerError::TransferNotFound { transfer_id })?;
        let temporary_path = upload.temporary.path().to_path_buf();
        let content_length = u64::try_from(content.len()).unwrap_or(u64::MAX);
        let end_offset = offset.saturating_add(content_length);
        if end_offset > upload.expected_size {
            return Err(FileManagerError::TransferSizeMismatch {
                expected: upload.expected_size,
                actual: end_offset,
            });
        }
        if offset < upload.next_offset {
            if end_offset > upload.next_offset {
                return Err(FileManagerError::TransferOffsetMismatch {
                    expected: upload.next_offset,
                    actual: offset,
                });
            }
            let file = upload.temporary.as_file_mut();
            file.seek(SeekFrom::Start(offset))
                .map_err(|source| FileManagerError::Io {
                    operation: "seek upload temporary file",
                    path: temporary_path.clone(),
                    source,
                })?;
            let mut existing = vec![0_u8; content.len()];
            file.read_exact(&mut existing)
                .map_err(|source| FileManagerError::Io {
                    operation: "read upload temporary file",
                    path: temporary_path.clone(),
                    source,
                })?;
            if existing != content {
                return Err(FileManagerError::TransferOffsetMismatch {
                    expected: upload.next_offset,
                    actual: offset,
                });
            }
        } else if offset == upload.next_offset {
            let file = upload.temporary.as_file_mut();
            file.seek(SeekFrom::Start(offset))
                .map_err(|source| FileManagerError::Io {
                    operation: "seek upload temporary file",
                    path: temporary_path.clone(),
                    source,
                })?;
            file.write_all(content)
                .map_err(|source| FileManagerError::Io {
                    operation: "write upload temporary file",
                    path: temporary_path.clone(),
                    source,
                })?;
            file.sync_data().map_err(|source| FileManagerError::Io {
                operation: "sync upload temporary file",
                path: temporary_path,
                source,
            })?;
            upload.next_offset = end_offset;
        } else {
            return Err(FileManagerError::TransferOffsetMismatch {
                expected: upload.next_offset,
                actual: offset,
            });
        }

        Ok(json!({
            "transferId": transfer_id,
            "nextOffset": upload.next_offset,
            "sizeBytes": upload.expected_size,
        }))
    }

    pub(crate) fn read_download_chunk(
        &self,
        transfer_id: TaskId,
        offset: u64,
    ) -> Result<Value, FileManagerError> {
        let mut downloads = self
            .downloads
            .lock()
            .map_err(|_| FileManagerError::TaskStorePoisoned)?;
        let download = downloads
            .get_mut(&transfer_id)
            .ok_or(FileManagerError::TransferNotFound { transfer_id })?;
        if offset > download.next_offset {
            return Err(FileManagerError::TransferOffsetMismatch {
                expected: download.next_offset,
                actual: offset,
            });
        }

        let metadata = fs::symlink_metadata(&download.source_path).map_err(|source| {
            if source.kind() == ErrorKind::NotFound {
                FileManagerError::NotFound {
                    path: download.source_path.clone(),
                }
            } else {
                FileManagerError::Io {
                    operation: "read download source metadata for",
                    path: download.source_path.clone(),
                    source,
                }
            }
        })?;
        if metadata.file_type().is_symlink() {
            return Err(FileManagerError::SymlinkNotAllowed {
                path: download.source_path.clone(),
            });
        }
        if !metadata.is_file() {
            return Err(FileManagerError::NotFile {
                path: download.source_path.clone(),
            });
        }
        if metadata.len() != download.expected_size {
            return Err(FileManagerError::TransferSizeMismatch {
                expected: download.expected_size,
                actual: metadata.len(),
            });
        }

        let mut file =
            File::open(&download.source_path).map_err(|source| FileManagerError::Io {
                operation: "open download source",
                path: download.source_path.clone(),
                source,
            })?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|source| FileManagerError::Io {
                operation: "seek download source",
                path: download.source_path.clone(),
                source,
            })?;
        let remaining = download.expected_size.saturating_sub(offset);
        let chunk_length = usize::try_from(remaining.min(FILE_TRANSFER_CHUNK_BYTES as u64))
            .unwrap_or(FILE_TRANSFER_CHUNK_BYTES);
        let mut content = vec![0_u8; chunk_length];
        let bytes_read = file
            .read(&mut content)
            .map_err(|source| FileManagerError::Io {
                operation: "read download source",
                path: download.source_path.clone(),
                source,
            })?;
        content.truncate(bytes_read);
        let end_offset = offset.saturating_add(bytes_read as u64);
        if bytes_read < chunk_length {
            return Err(FileManagerError::TransferSizeMismatch {
                expected: download.expected_size,
                actual: offset.saturating_add(bytes_read as u64),
            });
        }
        if offset == download.next_offset {
            download.next_offset = end_offset;
        }

        Ok(json!({
            "transferId": transfer_id,
            "offset": offset,
            "nextOffset": download.next_offset,
            "sizeBytes": download.expected_size,
            "sha256": sha256_hex(Sha256::digest(&content)),
            "fileSha256": download.expected_sha256,
            "dataBase64": STANDARD.encode(content),
            "eof": end_offset == download.expected_size,
        }))
    }

    pub(crate) fn commit_upload(&self, transfer_id: TaskId) -> Result<FileEntry, FileManagerError> {
        let upload = {
            let mut uploads = self
                .uploads
                .lock()
                .map_err(|_| FileManagerError::TaskStorePoisoned)?;
            let upload = uploads
                .get(&transfer_id)
                .ok_or(FileManagerError::TransferNotFound { transfer_id })?;
            if upload.next_offset != upload.expected_size {
                return Err(FileManagerError::TransferIncomplete {
                    expected: upload.expected_size,
                    actual: upload.next_offset,
                });
            }
            let actual_sha256 = hash_file(upload.temporary.path())?;
            if !upload.expected_sha256.eq_ignore_ascii_case(&actual_sha256) {
                return Err(FileManagerError::TransferHashMismatch {
                    expected: upload.expected_sha256.clone(),
                    actual: actual_sha256,
                });
            }
            uploads
                .remove(&transfer_id)
                .ok_or(FileManagerError::TransferNotFound { transfer_id })?
        };

        let target_path = upload.target_path.clone();
        upload
            .temporary
            .persist(&target_path)
            .map_err(|error| FileManagerError::Io {
                operation: "replace uploaded file",
                path: target_path,
                source: error.error,
            })?;
        relative_file_entry(&upload.root_path, &upload.target_path)
    }

    pub(crate) fn abort_upload(&self, transfer_id: TaskId) -> Result<(), FileManagerError> {
        let mut uploads = self
            .uploads
            .lock()
            .map_err(|_| FileManagerError::TaskStorePoisoned)?;
        uploads
            .remove(&transfer_id)
            .map(|_| ())
            .ok_or(FileManagerError::TransferNotFound { transfer_id })
    }

    pub(crate) fn commit_download(&self, transfer_id: TaskId) -> Result<(), FileManagerError> {
        let mut downloads = self
            .downloads
            .lock()
            .map_err(|_| FileManagerError::TaskStorePoisoned)?;
        let download = downloads
            .get(&transfer_id)
            .ok_or(FileManagerError::TransferNotFound { transfer_id })?;
        if download.next_offset != download.expected_size {
            return Err(FileManagerError::TransferIncomplete {
                expected: download.expected_size,
                actual: download.next_offset,
            });
        }
        let actual_sha256 = hash_file(&download.source_path)?;
        if !download
            .expected_sha256
            .eq_ignore_ascii_case(&actual_sha256)
        {
            return Err(FileManagerError::TransferHashMismatch {
                expected: download.expected_sha256.clone(),
                actual: actual_sha256,
            });
        }
        downloads.remove(&transfer_id);
        Ok(())
    }

    pub(crate) fn abort_download(&self, transfer_id: TaskId) -> Result<(), FileManagerError> {
        let mut downloads = self
            .downloads
            .lock()
            .map_err(|_| FileManagerError::TaskStorePoisoned)?;
        downloads
            .remove(&transfer_id)
            .map(|_| ())
            .ok_or(FileManagerError::TransferNotFound { transfer_id })
    }

    pub fn task(&self, task_id: TaskId) -> Result<Option<Value>, FileManagerError> {
        let tasks = self
            .tasks
            .lock()
            .map_err(|_| FileManagerError::TaskStorePoisoned)?;
        Ok(tasks.get(&task_id).cloned())
    }

    fn validate_delete_target(
        &self,
        instance: &Instance,
        relative_path: &str,
        recursive: bool,
    ) -> Result<(), FileManagerError> {
        let root = self.instance_root(instance)?;
        let relative = parse_relative_path(relative_path, false)?;
        let path = root.join(relative);
        let metadata = fs::symlink_metadata(&path).map_err(|source| {
            if source.kind() == ErrorKind::NotFound {
                FileManagerError::NotFound { path: path.clone() }
            } else {
                FileManagerError::Io {
                    operation: "read delete target metadata for",
                    path: path.clone(),
                    source,
                }
            }
        })?;
        if metadata.file_type().is_symlink() {
            return Err(FileManagerError::SymlinkNotAllowed { path });
        }
        if metadata.is_dir() && !recursive && directory_has_entries(&path)? {
            return Err(FileManagerError::DirectoryNotEmpty { path });
        }
        let canonical = fs::canonicalize(&path).map_err(|source| FileManagerError::Io {
            operation: "resolve delete target",
            path: path.clone(),
            source,
        })?;
        if !canonical.starts_with(&root) {
            return Err(FileManagerError::PathEscapes { path: canonical });
        }

        Ok(())
    }

    fn delete_sync(
        &self,
        instance: &Instance,
        relative_path: &str,
        recursive: bool,
    ) -> Result<(), FileManagerError> {
        self.validate_delete_target(instance, relative_path, recursive)?;
        let root = self.instance_root(instance)?;
        let path = root.join(parse_relative_path(relative_path, false)?);
        let metadata = fs::symlink_metadata(&path).map_err(|source| FileManagerError::Io {
            operation: "read delete target metadata for",
            path: path.clone(),
            source,
        })?;
        if metadata.is_dir() {
            if recursive {
                fs::remove_dir_all(&path).map_err(|source| FileManagerError::Io {
                    operation: "delete directory",
                    path,
                    source,
                })?;
            } else {
                fs::remove_dir(&path).map_err(|source| FileManagerError::Io {
                    operation: "delete directory",
                    path,
                    source,
                })?;
            }
        } else if metadata.is_file() {
            fs::remove_file(&path).map_err(|source| FileManagerError::Io {
                operation: "delete file",
                path,
                source,
            })?;
        } else {
            return Err(FileManagerError::NotFile { path });
        }

        Ok(())
    }

    fn execute_batch(
        &self,
        instance: &Instance,
        task_id: TaskId,
        operations: Vec<FileBatchOperation>,
    ) -> Result<Vec<Value>, (Vec<Value>, usize, String)> {
        let total = operations.len();
        let mut results = Vec::with_capacity(total);
        for (index, operation) in operations.into_iter().enumerate() {
            let result = match operation {
                FileBatchOperation::CreateDirectory { path, recursive } => self
                    .mkdir(instance, &path, recursive)
                    .map(|entry| json!({ "entry": entry })),
                FileBatchOperation::Move {
                    from,
                    to,
                    overwrite,
                } => self
                    .move_entry(instance, &from, &to, overwrite)
                    .map(|entry| json!({ "entry": entry })),
                FileBatchOperation::Write {
                    path,
                    content,
                    expected_sha256,
                } => self
                    .write(instance, &path, &content, expected_sha256.as_deref())
                    .map(|entry| json!({ "entry": entry })),
                FileBatchOperation::Delete { path, recursive } => self
                    .delete_sync(instance, &path, recursive)
                    .map(|()| json!({ "path": path, "deleted": true })),
            };

            match result {
                Ok(result) => {
                    results.push(json!({
                        "index": index,
                        "state": "SUCCEEDED",
                        "result": result,
                    }));
                    self.update_batch_progress(task_id, index + 1, total);
                }
                Err(error) => {
                    results.push(json!({
                        "index": index,
                        "state": "FAILED",
                        "error": error.to_string(),
                    }));
                    return Err((results, index, error.to_string()));
                }
            }
        }

        Ok(results)
    }

    fn finish_delete_task(&self, task_id: TaskId, result: Result<Value, String>) {
        let Ok(mut tasks) = self.tasks.lock() else {
            return;
        };
        let Some(task) = tasks.get_mut(&task_id) else {
            return;
        };
        match result {
            Ok(result) => {
                task["state"] = json!("SUCCEEDED");
                if let Some(object) = result.as_object() {
                    for (key, value) in object {
                        task[key] = value.clone();
                    }
                }
            }
            Err(error) => {
                task["state"] = json!("FAILED");
                task["error"] = json!(error);
            }
        }
    }

    fn update_batch_progress(&self, task_id: TaskId, completed: usize, total: usize) {
        let Ok(mut tasks) = self.tasks.lock() else {
            return;
        };
        let Some(task) = tasks.get_mut(&task_id) else {
            return;
        };
        task["progress"] = json!({ "completed": completed, "total": total });
    }

    fn finish_batch_task(
        &self,
        task_id: TaskId,
        result: Result<Vec<Value>, (Vec<Value>, usize, String)>,
    ) {
        let Ok(mut tasks) = self.tasks.lock() else {
            return;
        };
        let Some(task) = tasks.get_mut(&task_id) else {
            return;
        };
        match result {
            Ok(results) => {
                let total = results.len();
                task["state"] = json!("SUCCEEDED");
                task["progress"] = json!({ "completed": total, "total": total });
                task["results"] = json!(results);
            }
            Err((results, failed_index, error)) => {
                task["state"] = json!("FAILED");
                task["failedIndex"] = json!(failed_index);
                task["error"] = json!(error);
                task["results"] = json!(results);
            }
        }
    }

    fn create_archive(
        &self,
        task_id: TaskId,
        root: PathBuf,
        target: PathBuf,
        entries: Vec<(PathBuf, String, bool)>,
        total: usize,
    ) -> Result<FileEntry, FileManagerError> {
        let parent = target
            .parent()
            .ok_or_else(|| FileManagerError::InvalidPath {
                path: target.to_string_lossy().into_owned(),
            })?;
        let mut temporary =
            NamedTempFile::new_in(parent).map_err(|source| FileManagerError::Io {
                operation: "create archive temporary file in",
                path: parent.to_path_buf(),
                source,
            })?;
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut archive = ZipWriter::new(temporary.as_file_mut());
        for (index, (path, relative_path, is_directory)) in entries.iter().enumerate() {
            if *is_directory {
                let directory_path = format!("{relative_path}/");
                archive
                    .add_directory(directory_path, options)
                    .map_err(|error| archive_error("add archive directory", path, error))?;
            } else {
                archive
                    .start_file(relative_path, options)
                    .map_err(|error| archive_error("add archive file", path, error))?;
                let mut source = File::open(path).map_err(|source| FileManagerError::Io {
                    operation: "open archive source",
                    path: path.clone(),
                    source,
                })?;
                copy(&mut source, &mut archive).map_err(|source| FileManagerError::Io {
                    operation: "write archive file",
                    path: path.clone(),
                    source,
                })?;
            }
            self.update_archive_progress(task_id, index + 1, total);
        }
        archive
            .finish()
            .map_err(|error| archive_error("finish archive", &target, error))?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|source| FileManagerError::Io {
                operation: "sync archive",
                path: target.clone(),
                source,
            })?;
        temporary
            .persist(&target)
            .map_err(|error| FileManagerError::Io {
                operation: "replace archive",
                path: target.clone(),
                source: error.error,
            })?;
        relative_file_entry(&root, &target)
    }

    fn update_archive_progress(&self, task_id: TaskId, completed: usize, total: usize) {
        let Ok(mut tasks) = self.tasks.lock() else {
            return;
        };
        let Some(task) = tasks.get_mut(&task_id) else {
            return;
        };
        task["progress"] = json!({ "completed": completed, "total": total });
    }

    fn finish_archive_task(
        &self,
        task_id: TaskId,
        result: Result<FileEntry, String>,
        total: usize,
    ) {
        let Ok(mut tasks) = self.tasks.lock() else {
            return;
        };
        let Some(task) = tasks.get_mut(&task_id) else {
            return;
        };
        match result {
            Ok(entry) => {
                task["state"] = json!("SUCCEEDED");
                task["progress"] = json!({ "completed": total, "total": total });
                task["archive"] = json!(entry);
            }
            Err(error) => {
                task["state"] = json!("FAILED");
                task["error"] = json!(error);
            }
        }
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

fn collect_archive_entries(
    root: &Path,
    paths: &[String],
    output_path: &str,
) -> Result<Vec<(PathBuf, String, bool)>, FileManagerError> {
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    let mut total_bytes = 0;
    for relative_path in paths {
        let path = resolve_archive_source(root, relative_path)?;
        collect_archive_entry(
            &path,
            relative_path,
            output_path,
            &mut seen,
            &mut entries,
            &mut total_bytes,
        )?;
    }
    entries.sort_by(|left, right| left.1.cmp(&right.1));
    Ok(entries)
}

fn resolve_archive_source(root: &Path, relative_path: &str) -> Result<PathBuf, FileManagerError> {
    let relative = parse_relative_path(relative_path, true)?;
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path).map_err(|source| {
        if source.kind() == ErrorKind::NotFound {
            FileManagerError::NotFound { path: path.clone() }
        } else {
            FileManagerError::Io {
                operation: "read archive source metadata",
                path: path.clone(),
                source,
            }
        }
    })?;
    if metadata.file_type().is_symlink() {
        return Err(FileManagerError::SymlinkNotAllowed { path });
    }
    let canonical = fs::canonicalize(&path).map_err(|source| FileManagerError::Io {
        operation: "resolve archive source",
        path: path.clone(),
        source,
    })?;
    if !canonical.starts_with(root) {
        return Err(FileManagerError::PathEscapes { path: canonical });
    }
    Ok(canonical)
}

fn collect_archive_entry(
    path: &Path,
    relative_path: &str,
    output_path: &str,
    seen: &mut BTreeSet<String>,
    entries: &mut Vec<(PathBuf, String, bool)>,
    total_bytes: &mut u64,
) -> Result<(), FileManagerError> {
    if relative_path == output_path {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(path).map_err(|source| FileManagerError::Io {
        operation: "read archive entry metadata",
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() {
        return Err(FileManagerError::SymlinkNotAllowed {
            path: path.to_path_buf(),
        });
    }
    let is_directory = metadata.is_dir();
    if !is_directory && !metadata.is_file() {
        return Err(FileManagerError::NotFile {
            path: path.to_path_buf(),
        });
    }
    if !relative_path.is_empty() && seen.insert(relative_path.to_owned()) {
        if entries.len() >= MAXIMUM_FILE_ARCHIVE_ENTRIES {
            return Err(FileManagerError::ArchiveTooLarge {
                maximum_entries: MAXIMUM_FILE_ARCHIVE_ENTRIES,
                maximum_bytes: MAXIMUM_FILE_ARCHIVE_BYTES,
            });
        }
        if !is_directory {
            let next_total = total_bytes.checked_add(metadata.len()).ok_or(
                FileManagerError::ArchiveTooLarge {
                    maximum_entries: MAXIMUM_FILE_ARCHIVE_ENTRIES,
                    maximum_bytes: MAXIMUM_FILE_ARCHIVE_BYTES,
                },
            )?;
            if next_total > MAXIMUM_FILE_ARCHIVE_BYTES {
                return Err(FileManagerError::ArchiveTooLarge {
                    maximum_entries: MAXIMUM_FILE_ARCHIVE_ENTRIES,
                    maximum_bytes: MAXIMUM_FILE_ARCHIVE_BYTES,
                });
            }
            *total_bytes = next_total;
        }
        entries.push((path.to_path_buf(), relative_path.to_owned(), is_directory));
    }
    if !is_directory {
        return Ok(());
    }

    let mut children = fs::read_dir(path)
        .map_err(|source| FileManagerError::Io {
            operation: "read archive directory",
            path: path.to_path_buf(),
            source,
        })?
        .map(|entry| {
            let entry = entry.map_err(|source| FileManagerError::Io {
                operation: "read archive directory entry",
                path: path.to_path_buf(),
                source,
            })?;
            let child_path = entry.path();
            let child_name = entry
                .file_name()
                .to_str()
                .ok_or_else(|| FileManagerError::NonUtf8Path {
                    path: child_path.clone(),
                })?
                .to_owned();
            Ok((child_name, child_path))
        })
        .collect::<Result<Vec<_>, FileManagerError>>()?;
    children.sort_by(|left, right| left.0.cmp(&right.0));
    for (child_name, child_path) in children {
        let child_relative = if relative_path.is_empty() {
            child_name
        } else {
            format!("{relative_path}/{child_name}")
        };
        collect_archive_entry(
            &child_path,
            &child_relative,
            output_path,
            seen,
            entries,
            total_bytes,
        )?;
    }
    Ok(())
}

fn archive_error(operation: &'static str, path: &Path, error: impl Display) -> FileManagerError {
    FileManagerError::Io {
        operation,
        path: path.to_path_buf(),
        source: Error::other(error.to_string()),
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

fn relative_file_entry(root: &Path, path: &Path) -> Result<FileEntry, FileManagerError> {
    let relative_path = relative_file_path(root, path)?;
    file_entry(path, relative_path)
}

fn directory_has_entries(path: &Path) -> Result<bool, FileManagerError> {
    fs::read_dir(path)
        .map_err(|source| FileManagerError::Io {
            operation: "read directory",
            path: path.to_path_buf(),
            source,
        })?
        .next()
        .map_or(Ok(false), |entry| {
            entry.map(|_| true).map_err(|source| FileManagerError::Io {
                operation: "read directory entry",
                path: path.to_path_buf(),
                source,
            })
        })
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
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;
    use std::fs;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;
    use std::time::Duration;

    use nexus_domain::FileKind;
    use nexus_domain::Instance;
    use nexus_domain::InstanceCreate;
    use nexus_domain::InstanceId;
    use nexus_domain::InstanceKind;
    use nexus_domain::LaunchConfig;
    use nexus_domain::TaskId;
    use serde_json::Value;
    use serde_json::json;
    use tempfile::tempdir;
    use tokio::time::sleep;
    use tokio::time::timeout;
    use zip::ZipArchive;

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
    fn downloads_ordered_chunks_and_validates_completion() {
        let directory = tempdir().expect("temporary directory is created");
        let instance = instance();
        let instance_directory = directory.path().join(instance.directory());
        fs::create_dir_all(&instance_directory).expect("instance directory is created");
        let content = b"downloaded file contents";
        fs::write(instance_directory.join("server.properties"), content)
            .expect("download source is written");
        let manager = FileManager::new(directory.path());

        let start = manager
            .begin_download(&instance, "server.properties")
            .expect("download is accepted");
        let transfer_id = start["transferId"]
            .as_str()
            .expect("download transfer ID is returned")
            .parse::<TaskId>()
            .expect("download transfer ID is valid");
        assert_eq!(start["sizeBytes"], content.len());
        assert_eq!(start["nextOffset"], 0);

        let chunk = manager
            .read_download_chunk(transfer_id, 0)
            .expect("first download chunk is readable");
        assert_eq!(chunk["nextOffset"], content.len());
        assert_eq!(chunk["eof"], true);
        assert_eq!(
            STANDARD
                .decode(
                    chunk["dataBase64"]
                        .as_str()
                        .expect("chunk data is returned")
                )
                .expect("chunk data is valid Base64"),
            content
        );
        let retry = manager
            .read_download_chunk(transfer_id, 0)
            .expect("download chunk can be retried");
        assert_eq!(retry["dataBase64"], chunk["dataBase64"]);
        assert!(matches!(
            manager.read_download_chunk(transfer_id, content.len() as u64 + 1),
            Err(FileManagerError::TransferOffsetMismatch { .. })
        ));
        manager
            .commit_download(transfer_id)
            .expect("completed download can be committed");
        assert!(matches!(
            manager.commit_download(transfer_id),
            Err(FileManagerError::TransferNotFound { .. })
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

    #[test]
    fn creates_recursive_directories_and_moves_entries_safely() {
        let directory = tempdir().expect("temporary directory is created");
        let instance = instance();
        let manager = FileManager::new(directory.path());

        let directory_entry = manager
            .mkdir(&instance, "config/server", true)
            .expect("recursive directory is created");
        assert_eq!(directory_entry.kind(), FileKind::Directory);
        assert_eq!(directory_entry.path(), "config/server");
        manager
            .write(&instance, "config/server.properties", b"motd=MCNP", None)
            .expect("source file is written");

        let moved = manager
            .move_entry(
                &instance,
                "config/server.properties",
                "config/server/server.properties",
                false,
            )
            .expect("file is moved");
        assert_eq!(moved.path(), "config/server/server.properties");
        assert!(
            manager
                .move_entry(
                    &instance,
                    "config/server/server.properties",
                    "config/server/server.properties",
                    false,
                )
                .is_ok()
        );
        assert!(matches!(
            manager.mkdir(&instance, "config/server/server.properties/logs", true),
            Err(FileManagerError::NotDirectory { .. })
        ));
    }

    #[tokio::test]
    async fn deletes_files_and_recursive_directories_as_tasks() {
        let directory = tempdir().expect("temporary directory is created");
        let instance = instance();
        let instance_directory = directory.path().join(instance.directory());
        fs::create_dir_all(instance_directory.join("nested")).expect("directories are created");
        fs::write(instance_directory.join("delete-me.txt"), b"delete me").expect("file is written");
        fs::write(
            instance_directory.join("nested/child.txt"),
            b"delete recursively",
        )
        .expect("nested file is written");
        let manager = FileManager::new(directory.path());

        let file_task_id = manager
            .start_delete(&instance, "delete-me.txt", false)
            .expect("file deletion task is accepted");
        let file_task = wait_for_task(&manager, file_task_id).await;
        assert_eq!(file_task["kind"], "FILE_DELETE");
        assert_eq!(file_task["state"], "SUCCEEDED");
        assert_eq!(file_task["path"], "delete-me.txt");
        assert_eq!(file_task["deleted"], true);
        assert!(!instance_directory.join("delete-me.txt").exists());

        assert!(matches!(
            manager.start_delete(&instance, "nested", false),
            Err(FileManagerError::DirectoryNotEmpty { .. })
        ));

        let directory_task_id = manager
            .start_delete(&instance, "nested", true)
            .expect("recursive directory deletion task is accepted");
        let directory_task = wait_for_task(&manager, directory_task_id).await;
        assert_eq!(directory_task["state"], "SUCCEEDED");
        assert_eq!(directory_task["path"], "nested");
        assert!(!instance_directory.join("nested").exists());
    }

    #[tokio::test]
    async fn creates_zip_archive_task_with_selected_files_and_directories() {
        let directory = tempdir().expect("temporary directory is created");
        let instance = instance();
        let instance_directory = directory.path().join(instance.directory());
        fs::create_dir_all(instance_directory.join("config/nested"))
            .expect("archive source directories are created");
        fs::create_dir(instance_directory.join("config/empty"))
            .expect("empty archive directory is created");
        fs::create_dir(instance_directory.join("downloads"))
            .expect("archive output directory is created");
        fs::write(
            instance_directory.join("config/nested/server.properties"),
            b"motd=MCNP",
        )
        .expect("nested archive source file is written");
        fs::write(
            instance_directory.join("server.properties"),
            b"level-name=world",
        )
        .expect("archive source file is written");
        let manager = FileManager::new(directory.path());

        let task_id = manager
            .start_archive(
                &instance,
                vec!["config".to_owned(), "server.properties".to_owned()],
                "downloads/backup.zip".to_owned(),
            )
            .expect("archive task is accepted");
        let task = wait_for_task(&manager, task_id).await;

        assert_eq!(task["kind"], "FILE_ARCHIVE_CREATE");
        assert_eq!(task["state"], "SUCCEEDED");
        assert_eq!(task["progress"], json!({ "completed": 5, "total": 5 }));
        assert_eq!(task["archive"]["path"], "downloads/backup.zip");
        assert_eq!(task["archive"]["kind"], "FILE");

        let archive_file = File::open(instance_directory.join("downloads/backup.zip"))
            .expect("created archive is readable");
        let mut archive = ZipArchive::new(archive_file).expect("created archive is valid ZIP");
        let names = (0..archive.len())
            .map(|index| {
                archive
                    .by_index(index)
                    .expect("archive entry is readable")
                    .name()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "config/",
                "config/empty/",
                "config/nested/",
                "config/nested/server.properties",
                "server.properties",
            ]
        );
        let mut nested_file = archive
            .by_name("config/nested/server.properties")
            .expect("nested archive file exists");
        let mut content = Vec::new();
        nested_file
            .read_to_end(&mut content)
            .expect("nested archive file is readable");
        assert_eq!(content, b"motd=MCNP");
    }

    #[test]
    fn rejects_unsafe_archive_paths_before_creating_a_task() {
        let directory = tempdir().expect("temporary directory is created");
        let instance = instance();
        let instance_directory = directory.path().join(instance.directory());
        fs::create_dir_all(instance_directory.join("downloads"))
            .expect("archive output directory is created");
        fs::write(instance_directory.join("server.properties"), b"motd=MCNP")
            .expect("archive source file is written");
        let manager = FileManager::new(directory.path());

        assert!(matches!(
            manager.start_archive(
                &instance,
                vec!["../outside".to_owned()],
                "downloads/backup.zip".to_owned(),
            ),
            Err(FileManagerError::InvalidPath { .. })
        ));
        assert!(matches!(
            manager.start_archive(
                &instance,
                vec!["server.properties".to_owned()],
                "server.properties".to_owned(),
            ),
            Err(FileManagerError::InvalidPath { .. })
        ));

        let too_many_paths = (0..=super::MAXIMUM_FILE_ARCHIVE_PATHS)
            .map(|index| format!("missing-{index}"))
            .collect();
        assert!(matches!(
            manager.start_archive(&instance, too_many_paths, "downloads/backup.zip".to_owned()),
            Err(FileManagerError::InvalidPath { .. })
        ));
    }

    #[test]
    fn rejects_archive_entry_and_source_size_limits() {
        let directory = tempdir().expect("temporary directory is created");
        let instance = instance();
        let instance_directory = directory.path().join(instance.directory());
        fs::create_dir_all(&instance_directory).expect("instance directory is created");
        let source = instance_directory.join("server.properties");
        fs::write(&source, b"motd=MCNP").expect("archive source file is written");

        let mut seen = (0..super::MAXIMUM_FILE_ARCHIVE_ENTRIES)
            .map(|index| format!("existing-{index}"))
            .collect::<BTreeSet<_>>();
        let mut entries = (0..super::MAXIMUM_FILE_ARCHIVE_ENTRIES)
            .map(|index| (PathBuf::new(), format!("existing-{index}"), false))
            .collect::<Vec<_>>();
        let mut total_bytes = 0;
        assert!(matches!(
            super::collect_archive_entry(
                &source,
                "server.properties",
                "backup.zip",
                &mut seen,
                &mut entries,
                &mut total_bytes,
            ),
            Err(FileManagerError::ArchiveTooLarge { .. })
        ));

        let mut seen = BTreeSet::new();
        let mut entries = Vec::new();
        let mut total_bytes = super::MAXIMUM_FILE_ARCHIVE_BYTES;
        assert!(matches!(
            super::collect_archive_entry(
                &source,
                "server.properties",
                "backup.zip",
                &mut seen,
                &mut entries,
                &mut total_bytes,
            ),
            Err(FileManagerError::ArchiveTooLarge { .. })
        ));
    }

    async fn wait_for_task(manager: &FileManager, task_id: TaskId) -> Value {
        timeout(Duration::from_secs(2), async {
            loop {
                let task = manager
                    .task(task_id)
                    .expect("file task state is readable")
                    .expect("file task is present");
                if task["state"] != "RUNNING" {
                    return task;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("file task finishes before the timeout")
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
