//! 一键搭建期间写入实例目录的受控文本文件。

use serde::Deserialize;
use serde::Serialize;

/// 描述安装完成后、实例目录原子提交前写入的一个 UTF-8 文本文件。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionFile {
    path: String,
    content: String,
}

impl ProvisionFile {
    /// 创建待写入的实例文本文件。
    #[must_use]
    pub fn new(path: String, content: String) -> Self {
        Self { path, content }
    }

    /// 返回实例目录内的相对路径。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 返回 UTF-8 文件内容。
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }
}
