//! 下载清单的 CPU 架构约束。

use serde::Deserialize;
use serde::Serialize;

/// 受管下载产物支持的 CPU 架构。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DownloadArchitecture {
    /// 与 CPU 架构无关的通用产物。
    Any,
    /// AArch64/ARM64。
    Aarch64,
    /// x86_64/AMD64。
    X86_64,
}

impl DownloadArchitecture {
    /// 返回当前编译目标对应的架构；未知架构返回 `None`。
    #[must_use]
    pub fn current() -> Option<Self> {
        match std::env::consts::ARCH {
            "aarch64" => Some(Self::Aarch64),
            "x86_64" => Some(Self::X86_64),
            _ => None,
        }
    }

    /// 判断该架构是否与当前编译目标一致。
    #[must_use]
    pub fn is_current(self) -> bool {
        self == Self::Any || Self::current() == Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::DownloadArchitecture;

    #[test]
    fn accepts_universal_downloads_on_the_current_architecture() {
        assert!(DownloadArchitecture::Any.is_current());
    }
}
