//! 代理后端拓扑约束。

use serde::Deserialize;
use serde::Serialize;

/// 描述代理能否以及最多能管理多少个后端实例。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProxyTopology {
    /// 普通服务端不允许登记代理后端。
    None,
    /// Java 代理可以登记任意数量的后端。
    OneToMany,
    /// Geyser 只能登记一个 Java 后端。
    OneToOne,
}

impl ProxyTopology {
    /// 判断给定后端数量是否符合拓扑约束。
    #[must_use]
    pub const fn allows_backend_count(self, count: usize) -> bool {
        match self {
            Self::None => count == 0,
            Self::OneToMany => true,
            Self::OneToOne => count <= 1,
        }
    }
}
