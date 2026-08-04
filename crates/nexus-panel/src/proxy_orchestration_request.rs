use serde::Deserialize;

/// 代理启停编排请求参数。
///
/// 默认连带管理后端；停止超时限制为 1 到 300 秒，最终拓扑和状态校验由 Core 执行。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyOrchestrationRequest {
    #[serde(default = "default_include_backends")]
    include_backends: bool,
    #[serde(default)]
    timeout_seconds: Option<u16>,
}

impl ProxyOrchestrationRequest {
    /// 校验停止超时是否在允许范围内。
    pub fn validate(&self) -> Result<(), ()> {
        if self
            .timeout_seconds
            .is_some_and(|seconds| !(1..=300).contains(&seconds))
        {
            return Err(());
        }

        Ok(())
    }

    /// 返回是否连带启停后端实例。
    #[must_use]
    pub const fn include_backends(&self) -> bool {
        self.include_backends
    }

    /// 返回可选的停止超时秒数。
    #[must_use]
    pub const fn timeout_seconds(&self) -> Option<u16> {
        self.timeout_seconds
    }
}

impl Default for ProxyOrchestrationRequest {
    fn default() -> Self {
        Self {
            include_backends: default_include_backends(),
            timeout_seconds: None,
        }
    }
}

const fn default_include_backends() -> bool {
    true
}
