use serde::Deserialize;
use serde::Serialize;

use crate::Instance;

/// 实例列表的一页及下一页游标。
///
/// 游标由 Core 生成并绑定查询上下文；Panel 应原样保存和传递它，不能用实例
/// 标识自行模拟分页。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstancePage {
    items: Vec<Instance>,
    next_cursor: Option<String>,
}

impl InstancePage {
    /// 创建实例分页结果。
    #[must_use]
    pub fn new(items: Vec<Instance>, next_cursor: Option<String>) -> Self {
        Self { items, next_cursor }
    }

    /// 返回当前页实例。
    #[must_use]
    pub fn items(&self) -> &[Instance] {
        &self.items
    }

    /// 返回下一页游标；没有下一页时为 `None`。
    #[must_use]
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}
