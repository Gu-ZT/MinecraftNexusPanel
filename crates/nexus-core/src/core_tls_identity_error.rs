use std::io;
use std::path::PathBuf;

use rcgen::Error as RcgenError;
use rustls::Error as RustlsError;
use thiserror::Error;

/// Core TLS 身份的生成、读取、校验和文件权限错误。
#[derive(Debug, Error)]
pub enum CoreTlsIdentityError {
    /// 创建默认 TLS 身份所在目录失败。
    #[error("failed to create the Core TLS identity directory {path}")]
    CreateDirectory {
        /// TLS 身份目录路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的目录创建错误。
        source: io::Error,
    },
    /// 证书链文件为空，无法建立 TLS 身份。
    #[error("Core TLS certificate chain is empty: {path}")]
    EmptyCertificateChain {
        /// 空证书链文件路径。
        path: PathBuf,
    },
    /// 生成默认自签名 TLS 身份失败。
    #[error("failed to generate the default Core TLS identity")]
    Generate(#[source] RcgenError),
    /// 默认身份的证书和私钥文件未能同时准备好。
    #[error("default Core TLS identity is incomplete: {certificate_path}, {private_key_path}")]
    IncompleteDefaultIdentity {
        /// 默认证书文件路径。
        certificate_path: PathBuf,
        /// 默认私钥文件路径。
        private_key_path: PathBuf,
    },
    /// 外部配置的证书和私钥没有成对提供。
    #[error("configured Core TLS identity is incomplete")]
    IncompleteConfiguredIdentity,
    /// TLS 库拒绝了证书或私钥内容。
    #[error("Core TLS certificate or private key is invalid")]
    InvalidIdentity(#[source] RustlsError),
    /// 私钥文件中没有受支持的私钥。
    #[error("Core TLS private key file does not contain a supported key: {path}")]
    MissingPrivateKey {
        /// 未找到受支持私钥的文件路径。
        path: PathBuf,
    },
    /// 读取 Core TLS 证书文件失败。
    #[error("failed to read the Core TLS certificate {path}")]
    ReadCertificate {
        /// 读取失败的证书文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的读取错误。
        source: io::Error,
    },
    /// 读取 Core TLS 私钥文件失败。
    #[error("failed to read the Core TLS private key {path}")]
    ReadPrivateKey {
        /// 读取失败的私钥文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的读取错误。
        source: io::Error,
    },
    /// 限制 Core TLS 私钥文件权限失败。
    #[error("failed to restrict permissions on the Core TLS private key {path}")]
    RestrictPrivateKey {
        /// 需要收紧权限的私钥文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的权限修改错误。
        source: io::Error,
    },
    /// 写入默认 Core TLS 证书失败。
    #[error("failed to write the default Core TLS certificate {path}")]
    WriteCertificate {
        /// 写入失败的证书文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的写入错误。
        source: io::Error,
    },
    /// 写入默认 Core TLS 私钥失败。
    #[error("failed to write the default Core TLS private key {path}")]
    WritePrivateKey {
        /// 写入失败的私钥文件路径。
        path: PathBuf,
        #[source]
        /// 操作系统返回的写入错误。
        source: io::Error,
    },
}
