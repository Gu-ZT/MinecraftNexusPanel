//! 安装模板的版本元数据来源。

use serde::Deserialize;
use serde::Serialize;

/// 一个可审计的官方或受信版本目录提供方。
///
/// 提供方只证明版本元数据可读取；归档结构、启动命令和升级流程仍需
/// 通过版本化安装配方逐项验证。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionMetadataProvider {
    id: String,
    name: String,
    url: String,
}

impl VersionMetadataProvider {
    /// 创建版本元数据提供方描述。
    #[must_use]
    pub fn new(id: String, name: String, url: String) -> Self {
        Self { id, name, url }
    }

    /// 返回 provider 的稳定标识，用于模板版本项引用来源。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回 provider 的展示名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回 provider 的元数据入口 URL。
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }
}
