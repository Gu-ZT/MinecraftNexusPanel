use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClientType {
    Browser,
    Native,
}

impl ClientType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Browser => "BROWSER",
            Self::Native => "NATIVE",
        }
    }
}
