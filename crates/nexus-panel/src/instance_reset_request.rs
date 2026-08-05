use serde::Deserialize;

/// 确认管理员已经处理 Core 重启后无法确认的旧进程状态。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InstanceResetRequest {
    confirmation: String,
}

impl InstanceResetRequest {
    /// 返回请求中的复位确认文本。
    pub(crate) fn confirmation(&self) -> &str {
        &self.confirmation
    }
}
