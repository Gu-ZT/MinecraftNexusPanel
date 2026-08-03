use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DownloadArchitecture {
    Aarch64,
    X86_64,
}

impl DownloadArchitecture {
    #[must_use]
    pub fn current() -> Option<Self> {
        match std::env::consts::ARCH {
            "aarch64" => Some(Self::Aarch64),
            "x86_64" => Some(Self::X86_64),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_current(self) -> bool {
        Self::current() == Some(self)
    }
}
