//! 下载清单的平台约束。

use serde::Deserialize;
use serde::Serialize;

/// 受管下载产物支持的操作系统平台。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DownloadPlatform {
    /// 与操作系统无关的通用产物。
    Any,
    /// Linux。
    Linux,
    /// macOS。
    Macos,
    /// Windows。
    Windows,
}

impl DownloadPlatform {
    /// 返回当前编译目标对应的平台；未知平台返回 `None`。
    #[must_use]
    pub fn current() -> Option<Self> {
        match std::env::consts::OS {
            "linux" => Some(Self::Linux),
            "macos" => Some(Self::Macos),
            "windows" => Some(Self::Windows),
            _ => None,
        }
    }

    /// 判断该平台是否与当前编译目标一致。
    #[must_use]
    pub fn is_current(self) -> bool {
        self == Self::Any || Self::current() == Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::DownloadPlatform;

    #[test]
    fn accepts_universal_downloads_on_the_current_platform() {
        assert!(DownloadPlatform::Any.is_current());
    }
}
