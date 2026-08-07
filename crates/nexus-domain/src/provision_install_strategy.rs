//! 一键搭建下载产物的安装策略。

use serde::Deserialize;
use serde::Serialize;

/// Core 将已校验下载产物转换为实例目录时使用的受控策略。
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProvisionInstallStrategy {
    /// 安全解压 ZIP 或 tar.gz 归档。
    #[default]
    ExtractArchive,
    /// 使用所选 Java 运行时执行供应商 installer JAR。
    JavaInstaller,
}
