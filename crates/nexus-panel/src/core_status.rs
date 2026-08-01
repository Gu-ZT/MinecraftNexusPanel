#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreStatus {
    AuthFailed,
    Incompatible,
    Offline,
    Online,
    Unknown,
}

impl CoreStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AuthFailed => "AUTH_FAILED",
            Self::Incompatible => "INCOMPATIBLE",
            Self::Offline => "OFFLINE",
            Self::Online => "ONLINE",
            Self::Unknown => "UNKNOWN",
        }
    }
}
