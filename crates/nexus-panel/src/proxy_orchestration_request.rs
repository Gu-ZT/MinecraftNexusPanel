use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyOrchestrationRequest {
    #[serde(default = "default_include_backends")]
    include_backends: bool,
    #[serde(default)]
    timeout_seconds: Option<u16>,
}

impl ProxyOrchestrationRequest {
    pub fn validate(&self) -> Result<(), ()> {
        if self
            .timeout_seconds
            .is_some_and(|seconds| !(1..=300).contains(&seconds))
        {
            return Err(());
        }

        Ok(())
    }

    #[must_use]
    pub const fn include_backends(&self) -> bool {
        self.include_backends
    }

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
