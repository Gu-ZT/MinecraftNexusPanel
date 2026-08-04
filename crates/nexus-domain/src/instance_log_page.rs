use serde::Deserialize;
use serde::Serialize;

use crate::InstanceLogLine;

/// 一次日志查询返回的分页结果。
///
/// `event_cursor` 标识本页对应的事件位置，`next_cursor` 只在仍有后续页时提供。
/// 两者都应由调用方原样传回后续查询，不应当作可排序的数字处理。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceLogPage {
    event_cursor: String,
    items: Vec<InstanceLogLine>,
    next_cursor: Option<String>,
}

impl InstanceLogPage {
    /// 创建日志分页结果。
    #[must_use]
    pub const fn new(
        items: Vec<InstanceLogLine>,
        next_cursor: Option<String>,
        event_cursor: String,
    ) -> Self {
        Self {
            event_cursor,
            items,
            next_cursor,
        }
    }

    /// 返回本页对应的事件游标。
    #[must_use]
    pub fn event_cursor(&self) -> &str {
        &self.event_cursor
    }

    /// 返回本页日志行。
    #[must_use]
    pub fn items(&self) -> &[InstanceLogLine] {
        &self.items
    }

    /// 返回下一页游标；没有下一页时为 `None`。
    #[must_use]
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}
