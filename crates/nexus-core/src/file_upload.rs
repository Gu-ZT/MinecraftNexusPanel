use std::path::PathBuf;

use tempfile::NamedTempFile;

/// Core 内部的文件上传传输状态。
///
/// 内容先写入同一文件系统中的临时文件，只有完整大小和摘要校验通过后才会替换目标。
pub(crate) struct FileUpload {
    pub(crate) root_path: PathBuf,
    pub(crate) target_path: PathBuf,
    pub(crate) temporary: NamedTempFile,
    pub(crate) expected_size: u64,
    pub(crate) expected_sha256: String,
    pub(crate) next_offset: u64,
}
