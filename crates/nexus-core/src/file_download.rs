use std::path::PathBuf;

pub(crate) struct FileDownload {
    pub(crate) source_path: PathBuf,
    pub(crate) expected_size: u64,
    pub(crate) expected_sha256: String,
    pub(crate) next_offset: u64,
}
