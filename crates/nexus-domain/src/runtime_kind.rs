//! Core 支持的受管运行时类型。

use serde::Deserialize;
use serde::Serialize;

/// 可被 Core 发现、安装、校验和删除的运行时。
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeKind {
    /// Java 虚拟机。
    Java,
    /// Node.js。
    NodeJs,
    /// Python。
    Python,
}

impl RuntimeKind {
    /// 当前支持的运行时类型全集，用于稳定的 API 展示顺序。
    pub const ALL: [Self; 3] = [Self::Java, Self::NodeJs, Self::Python];
}
