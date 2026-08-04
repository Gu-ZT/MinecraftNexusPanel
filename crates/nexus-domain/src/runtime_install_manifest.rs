use serde::Deserialize;
use serde::Serialize;

use crate::DownloadManifest;
use crate::RuntimeArchiveFormat;
use crate::RuntimeKind;

/// 描述一次可复现的受管运行时安装输入。
///
/// 清单把下载产物、目标运行时类型和解压后的可执行文件路径绑定在一起。
/// Core 应在执行安装前校验归档摘要，并在安装后重新确认可执行文件位置。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInstallManifest {
    runtime_id: String,
    kind: RuntimeKind,
    distribution: String,
    version: String,
    archive: DownloadManifest,
    archive_format: RuntimeArchiveFormat,
    executable_path: String,
}

impl RuntimeInstallManifest {
    /// 创建运行时安装清单。
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        runtime_id: String,
        kind: RuntimeKind,
        distribution: String,
        version: String,
        archive: DownloadManifest,
        archive_format: RuntimeArchiveFormat,
        executable_path: String,
    ) -> Self {
        Self {
            runtime_id,
            kind,
            distribution,
            version,
            archive,
            archive_format,
            executable_path,
        }
    }

    /// 返回安装记录的稳定标识。
    #[must_use]
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    /// 返回运行时类型。
    #[must_use]
    pub const fn kind(&self) -> RuntimeKind {
        self.kind
    }

    /// 返回发行版或供应商标识。
    #[must_use]
    pub fn distribution(&self) -> &str {
        &self.distribution
    }

    /// 返回要安装的运行时版本。
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// 返回待校验的下载清单。
    #[must_use]
    pub const fn archive(&self) -> &DownloadManifest {
        &self.archive
    }

    /// 返回下载归档格式。
    #[must_use]
    pub const fn archive_format(&self) -> RuntimeArchiveFormat {
        self.archive_format
    }

    /// 返回相对于受管运行时目录的可执行文件路径。
    #[must_use]
    pub fn executable_path(&self) -> &str {
        &self.executable_path
    }
}
