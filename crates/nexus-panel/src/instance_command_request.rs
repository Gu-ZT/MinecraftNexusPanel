use serde::Deserialize;

/// 实例控制台命令请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceCommandRequest {
    command: String,
}

impl InstanceCommandRequest {
    /// 返回待发送的控制台命令。
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
}
