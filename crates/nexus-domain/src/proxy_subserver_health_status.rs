//! 代理后端 TCP 网络状态。

use serde::Deserialize;
use serde::Serialize;

/// 代理后端的网络层可达性。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProxySubserverHealthStatus {
    /// 关系被禁用，没有执行网络探测。
    Disabled,
    /// TCP 连接成功建立。
    Reachable,
    /// TCP 连接失败或探测超时。
    Unreachable,
}
