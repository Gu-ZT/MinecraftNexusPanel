use std::path::PathBuf;

/// Core 内部的文件下载传输状态。
///
/// `next_offset` 是下一块必须使用的字节偏移，提交时还会重新校验整个文件摘要。
pub(crate) struct FileDownload {
    pub(crate) source_path: PathBuf,
    pub(crate) expected_size: u64,
    pub(crate) expected_sha256: String,
    pub(crate) next_offset: u64,
}
