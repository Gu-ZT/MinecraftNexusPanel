//! 扩展来源与请求筛选条件的兼容性结论。

use serde::Deserialize;
use serde::Serialize;

/// Panel 对扩展与 Minecraft/加载器筛选条件的判断。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExtensionCompatibility {
    /// 已知满足请求条件。
    Compatible,
    /// 已知不满足请求条件，安装前应阻止或明确提示。
    Incompatible,
    /// 来源未提供足够元数据，不能伪装成兼容。
    Unknown,
}
