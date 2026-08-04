//! 支持保留“未修改”和“清空”语义的补丁字段。

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

/// 结构化更新中一个字段的三态值。
///
/// 与 `Option<T>` 不同，`Unchanged` 表示不写入字段，`Clear` 表示显式
/// 清空字段，适合实例设置和配置文档的 Merge Patch。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum PatchField<T> {
    #[default]
    /// 保留原值。
    Unchanged,
    /// 设置为指定值。
    Set(T),
    /// 显式清空。
    Clear,
}

impl<T> PatchField<T> {
    /// 判断该字段是否不会修改目标值。
    #[must_use]
    pub const fn is_unchanged(&self) -> bool {
        matches!(self, Self::Unchanged)
    }
}

impl<T> Serialize for PatchField<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Unchanged | Self::Clear => serializer.serialize_none(),
            Self::Set(value) => value.serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for PatchField<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| value.map_or(Self::Clear, Self::Set))
    }
}
