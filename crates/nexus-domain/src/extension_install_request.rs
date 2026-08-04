//! 扩展安装请求。

use serde::Deserialize;
use serde::Serialize;

use crate::ExtensionPlanRequest;

/// 在解析计划基础上指定可选目标目录的安装请求。
///
/// 当模板对同一种扩展声明多个目录时，调用方必须提供 `directory`，
/// 以避免把插件或模组写入错误的位置。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionInstallRequest {
    #[serde(flatten)]
    plan: ExtensionPlanRequest,
    directory: Option<String>,
}

impl ExtensionInstallRequest {
    /// 返回用于重新解析的计划请求。
    #[must_use]
    pub const fn plan(&self) -> &ExtensionPlanRequest {
        &self.plan
    }

    /// 返回用户明确选择的目标目录。
    #[must_use]
    pub fn directory(&self) -> Option<&str> {
        self.directory.as_deref()
    }
}
