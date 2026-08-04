use thiserror::Error;

/// 协议版本协商失败的原因。
#[derive(Debug, Error, Eq, PartialEq)]
pub enum ProtocolVersionError {
    /// 本地和远端主版本不同，不能安全解释同一套线协议。
    #[error("protocol major version mismatch: local {local}, remote {remote}")]
    MajorMismatch {
        /// 当前端支持的协议主版本。
        local: u16,
        /// 远端宣告的协议主版本。
        remote: u16,
    },
}
