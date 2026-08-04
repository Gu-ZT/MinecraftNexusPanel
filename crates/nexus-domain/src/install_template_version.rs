//! 安装模板版本目录中的单个版本项。

use serde::Deserialize;
use serde::Serialize;

use crate::InstallTemplateVersionKind;

/// 描述游戏、加载器或服务端版本及其来源稳定性。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallTemplateVersion {
    id: String,
    provider_id: String,
    kind: InstallTemplateVersionKind,
    stable: bool,
    metadata_url: Option<String>,
}

impl InstallTemplateVersion {
    /// 创建一个模板版本目录项。
    #[must_use]
    pub fn new(
        id: String,
        provider_id: String,
        kind: InstallTemplateVersionKind,
        stable: bool,
        metadata_url: Option<String>,
    ) -> Self {
        Self {
            id,
            provider_id,
            kind,
            stable,
            metadata_url,
        }
    }

    /// 返回版本目录项标识。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回提供该版本元数据的 provider 标识。
    #[must_use]
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// 返回版本项的语义层级。
    #[must_use]
    pub const fn kind(&self) -> InstallTemplateVersionKind {
        self.kind
    }

    /// 判断该版本是否由 provider 标记为稳定版本。
    #[must_use]
    pub const fn stable(&self) -> bool {
        self.stable
    }

    /// 返回可选的 provider 详情 URL。
    #[must_use]
    pub fn metadata_url(&self) -> Option<&str> {
        self.metadata_url.as_deref()
    }
}
