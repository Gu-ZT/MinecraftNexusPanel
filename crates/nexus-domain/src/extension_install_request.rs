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
    #[serde(default)]
    bedrock_api_versions: Option<Vec<String>>,
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

    /// 返回可选的目标 Bedrock 插件 API 版本列表。
    ///
    /// 调用方未提供该列表时，Panel 仍会校验 manifest 结构，但不会把
    /// Minecraft 服务端版本字符串误当成 PocketMine/Nukkit API 版本。
    #[must_use]
    pub fn bedrock_api_versions(&self) -> Option<&[String]> {
        self.bedrock_api_versions.as_deref()
    }
}
