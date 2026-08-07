use thiserror::Error;

use crate::VersionMetadataError;

/// 模板级一键搭建请求解析错误。
#[derive(Debug, Error)]
pub(crate) enum TemplateProvisionError {
    /// 用户提交的模板参数不符合版本或实例约束。
    #[error("template provision field is invalid: {field}")]
    InvalidField {
        /// 不符合约束的请求字段。
        field: &'static str,
    },
    /// 模板尚未提供经过验证的安装配方。
    #[error("template {template_id} does not have a verified provision recipe")]
    UnsupportedTemplate {
        /// 尚未支持自动安装的模板标识。
        template_id: String,
    },
    /// 版本目录或下载清单解析失败。
    #[error(transparent)]
    VersionMetadata(#[from] VersionMetadataError),
}
