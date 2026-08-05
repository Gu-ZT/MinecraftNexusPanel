use serde::Deserialize;
use serde::Serialize;

use crate::InstanceAuditRecord;

/// 一次实例审计查询返回的记录页。
///
/// 当前查询按最新记录优先返回并使用数量上限；`next_cursor` 保留给后续
/// 持久化审计存储接入稳定游标时使用，当前始终为空。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceAuditPage {
    items: Vec<InstanceAuditRecord>,
    next_cursor: Option<String>,
}

impl InstanceAuditPage {
    /// 创建一页实例审计记录。
    #[must_use]
    pub const fn new(items: Vec<InstanceAuditRecord>, next_cursor: Option<String>) -> Self {
        Self { items, next_cursor }
    }

    /// 返回当前页审计记录。
    #[must_use]
    pub fn items(&self) -> &[InstanceAuditRecord] {
        &self.items
    }

    /// 返回下一页游标。
    #[must_use]
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}
