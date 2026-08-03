use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DownloadPlatform {
    Linux,
    Macos,
    Windows,
}

impl DownloadPlatform {
    #[must_use]
    pub fn current() -> Option<Self> {
        match std::env::consts::OS {
            "linux" => Some(Self::Linux),
            "macos" => Some(Self::Macos),
            "windows" => Some(Self::Windows),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_current(self) -> bool {
        Self::current() == Some(self)
    }
}
