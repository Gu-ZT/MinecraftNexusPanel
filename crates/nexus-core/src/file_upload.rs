use std::path::PathBuf;

use tempfile::NamedTempFile;

pub(crate) struct FileUpload {
    pub(crate) root_path: PathBuf,
    pub(crate) target_path: PathBuf,
    pub(crate) temporary: NamedTempFile,
    pub(crate) expected_size: u64,
    pub(crate) expected_sha256: String,
    pub(crate) next_offset: u64,
}
