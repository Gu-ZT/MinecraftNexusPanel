use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProxyTopology {
    None,
    OneToMany,
    OneToOne,
}

impl ProxyTopology {
    #[must_use]
    pub const fn allows_backend_count(self, count: usize) -> bool {
        match self {
            Self::None => count == 0,
            Self::OneToMany => true,
            Self::OneToOne => count <= 1,
        }
    }
}
