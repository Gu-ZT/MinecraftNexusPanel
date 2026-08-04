use serde::Deserialize;

/// 强制终止实例的确认请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceKillRequest {
    confirmation: String,
}

impl InstanceKillRequest {
    /// 返回必须与实例标识匹配的确认文本。
    #[must_use]
    pub fn confirmation(&self) -> &str {
        &self.confirmation
    }
}
