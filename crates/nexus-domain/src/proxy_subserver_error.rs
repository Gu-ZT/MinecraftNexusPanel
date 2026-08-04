//! 代理后端关系的输入校验错误。

use thiserror::Error;

/// 创建或更新代理后端关系时可能出现的领域错误。
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ProxySubserverError {
    /// 关系 ID 为空、超长或含有非允许字符。
    #[error("proxy subserver ID is invalid")]
    InvalidId,
    /// 显示名称为空、超长、首尾含空白或包含 NUL。
    #[error("proxy subserver name is invalid")]
    InvalidName,
    /// 主机为空、含空白、路径分隔符或 NUL。
    #[error("proxy subserver host is invalid")]
    InvalidHost,
    /// 端口为 0。
    #[error("proxy subserver port is invalid")]
    InvalidPort,
}
