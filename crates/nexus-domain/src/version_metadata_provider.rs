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

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }
}
