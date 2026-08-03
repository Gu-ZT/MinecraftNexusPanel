use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeKind {
    Java,
    NodeJs,
    Python,
}

impl RuntimeKind {
    pub const ALL: [Self; 3] = [Self::Java, Self::NodeJs, Self::Python];
}
