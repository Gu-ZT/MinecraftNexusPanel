//! Core 发现或安装后的运行时记录。

use serde::Deserialize;
use serde::Serialize;

use crate::RuntimeKind;
use crate::RuntimeSource;
use crate::RuntimeValidation;

/// 一个可供实例启动使用的 Java、Node.js 或 Python 运行时。
///
/// 系统运行时只能被发现和验证；受管运行时才允许通过运行时管理器删除。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedRuntime {
    runtime_id: Option<String>,
    kind: RuntimeKind,
    source: RuntimeSource,
    distribution: Option<String>,
    executable: String,
    version: Option<String>,
    validation: RuntimeValidation,
}

impl ManagedRuntime {
    /// 创建运行时记录。
    #[must_use]
    pub fn new(
        kind: RuntimeKind,
        source: RuntimeSource,
        executable: String,
        version: Option<String>,
        validation: RuntimeValidation,
    ) -> Self {
        Self {
            runtime_id: None,
            kind,
            source,
            distribution: None,
            executable,
            version,
            validation,
        }
    }

    /// 创建受管运行时记录。
    #[must_use]
    pub fn managed(
        runtime_id: String,
        kind: RuntimeKind,
        distribution: String,
        executable: String,
        version: Option<String>,
        validation: RuntimeValidation,
    ) -> Self {
        Self {
            runtime_id: Some(runtime_id),
            kind,
            source: RuntimeSource::Managed,
            distribution: Some(distribution),
            executable,
            version,
            validation,
        }
    }

    /// 返回受管运行时标识；系统发现的运行时没有该标识。
    #[must_use]
    pub fn runtime_id(&self) -> Option<&str> {
        self.runtime_id.as_deref()
    }

    /// 返回运行时类型。
    #[must_use]
    pub const fn kind(&self) -> RuntimeKind {
        self.kind
    }

    /// 返回运行时来源。
    #[must_use]
    pub const fn source(&self) -> RuntimeSource {
        self.source
    }

    /// 返回发行版或供应商标识。
    #[must_use]
    pub fn distribution(&self) -> Option<&str> {
        self.distribution.as_deref()
    }

    /// 返回可执行文件路径。
    #[must_use]
    pub fn executable(&self) -> &str {
        &self.executable
    }

    /// 返回 Core 探测到的版本；无法解析时为 `None`。
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// 返回可执行文件验证结论。
    #[must_use]
    pub const fn validation(&self) -> RuntimeValidation {
        self.validation
    }
}
