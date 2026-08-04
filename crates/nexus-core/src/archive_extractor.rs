//! 受管运行时归档的安全解压工具。
//!
//! 归档条目必须是规范化相对路径，拒绝绝对路径、父目录跳转、符号链接和特殊
//! 文件类型，避免解压过程写出目标目录。

use std::fs::File;
use std::io;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use nexus_domain::RuntimeArchiveFormat;
use tar::Archive;
use zip::ZipArchive;

use crate::RuntimeManagerError;

/// 按声明格式将运行时归档解压到目标目录。
pub(crate) fn extract(
    archive_path: &Path,
    format: RuntimeArchiveFormat,
    destination: &Path,
) -> Result<(), RuntimeManagerError> {
    match format {
        RuntimeArchiveFormat::TarGz => extract_tar_gz(archive_path, destination),
        RuntimeArchiveFormat::Zip => extract_zip(archive_path, destination),
    }
}

fn extract_tar_gz(archive_path: &Path, destination: &Path) -> Result<(), RuntimeManagerError> {
    let file =
        File::open(archive_path).map_err(|source| archive_io("open", archive_path, source))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|source| archive_io("read", archive_path, source))?;

    for entry in entries {
        let mut entry = entry.map_err(|source| archive_io("read", archive_path, source))?;
        let path = entry
            .path()
            .map_err(|source| archive_io("read", archive_path, source))?
            .to_path_buf();
        let relative_path = safe_relative_path(&path.to_string_lossy())?;
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err(RuntimeManagerError::UnsafeArchiveEntry {
                path: relative_path,
            });
        }

        let output_path = destination.join(relative_path);
        if entry_type.is_dir() {
            std::fs::create_dir_all(&output_path)
                .map_err(|source| archive_io("create", &output_path, source))?;
        } else {
            let parent =
                output_path
                    .parent()
                    .ok_or_else(|| RuntimeManagerError::UnsafeArchiveEntry {
                        path: output_path.clone(),
                    })?;
            std::fs::create_dir_all(parent)
                .map_err(|source| archive_io("create", parent, source))?;
            entry
                .unpack(&output_path)
                .map_err(|source| archive_io("extract", &output_path, source))?;
        }
    }

    Ok(())
}

fn extract_zip(archive_path: &Path, destination: &Path) -> Result<(), RuntimeManagerError> {
    let file =
        File::open(archive_path).map_err(|source| archive_io("open", archive_path, source))?;
    let mut archive = ZipArchive::new(file).map_err(|source| RuntimeManagerError::Archive {
        operation: "read",
        path: archive_path.to_path_buf(),
        message: source.to_string(),
    })?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|source| RuntimeManagerError::Archive {
                operation: "read",
                path: archive_path.to_path_buf(),
                message: source.to_string(),
            })?;
        let relative_path = safe_relative_path(entry.name())?;
        if entry.is_symlink() {
            return Err(RuntimeManagerError::UnsafeArchiveEntry {
                path: relative_path,
            });
        }
        let output_path = destination.join(relative_path);
        if entry.is_dir() {
            std::fs::create_dir_all(&output_path)
                .map_err(|source| archive_io("create", &output_path, source))?;
            continue;
        }

        let parent =
            output_path
                .parent()
                .ok_or_else(|| RuntimeManagerError::UnsafeArchiveEntry {
                    path: output_path.clone(),
                })?;
        std::fs::create_dir_all(parent).map_err(|source| archive_io("create", parent, source))?;
        let mut output = File::create(&output_path)
            .map_err(|source| archive_io("create", &output_path, source))?;
        io::copy(&mut entry, &mut output)
            .map_err(|source| archive_io("write", &output_path, source))?;
        set_executable(&output_path, entry.unix_mode());
    }

    Ok(())
}

fn safe_relative_path(value: &str) -> Result<PathBuf, RuntimeManagerError> {
    let normalized = value.replace('\\', "/");
    let path = Path::new(&normalized);
    if normalized.is_empty() || path.is_absolute() {
        return Err(RuntimeManagerError::UnsafeArchiveEntry {
            path: path.to_path_buf(),
        });
    }

    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => relative.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(RuntimeManagerError::UnsafeArchiveEntry {
                    path: path.to_path_buf(),
                });
            }
        }
    }
    if relative.as_os_str().is_empty() {
        return Err(RuntimeManagerError::UnsafeArchiveEntry {
            path: path.to_path_buf(),
        });
    }

    Ok(relative)
}

#[cfg(unix)]
fn set_executable(path: &Path, unix_mode: Option<u32>) {
    let Some(unix_mode) = unix_mode else {
        return;
    };
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = metadata.permissions();
    permissions.set_mode(unix_mode & 0o777);
    let _ = std::fs::set_permissions(path, permissions);
}

#[cfg(not(unix))]
fn set_executable(_path: &Path, _unix_mode: Option<u32>) {}

fn archive_io(operation: &'static str, path: &Path, source: io::Error) -> RuntimeManagerError {
    RuntimeManagerError::ArchiveIo {
        operation,
        path: path.to_path_buf(),
        source,
    }
}
