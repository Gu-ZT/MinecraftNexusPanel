use nexus_protocol::PresharedKeyError;
use thiserror::Error;

/// 启动参数、环境变量或安全配置校验错误。
#[derive(Debug, Error, Eq, PartialEq)]
pub enum ConfigError {
    /// 同时指定了多个运行模式。
    #[error("mode can only be specified once")]
    DuplicateMode,
    /// 用户请求显示帮助并正常结束解析。
    #[error("help requested")]
    HelpRequested,
    /// 运行模式名称不受支持。
    #[error("invalid mode: {value}")]
    InvalidMode {
        /// 用户传入的原始运行模式名称。
        value: String,
    },
    /// 监听地址不是合法 Socket 地址。
    #[error("invalid socket address for {option}: {value}")]
    InvalidSocketAddress {
        /// 产生错误的配置选项名称。
        option: &'static str,
        /// 用户传入的原始地址文本。
        value: String,
    },
    /// Core 预共享密钥无法按协议格式解析。
    #[error("invalid Core pre-shared key")]
    InvalidCorePreSharedKey(#[source] PresharedKeyError),
    /// Panel 主密钥不是恰好 32 字节的无填充 Base64URL。
    #[error("Panel master key must be exactly 32 bytes encoded as unpadded Base64URL")]
    InvalidPanelMasterKey,
    /// Panel 审计保留数量不是整数或超出安全范围。
    #[error("Panel audit retention event count is invalid: {value}")]
    InvalidPanelAuditRetention {
        /// 用户提供的原始数量文本。
        value: String,
    },
    /// 初始管理员用户名为空或超出长度限制。
    #[error("initial administrator username must contain between 1 and 64 characters")]
    InvalidInitialAdminUsername,
    /// Desktop 会话引导用户名为空或超出长度限制。
    #[error("Desktop session username must contain between 1 and 64 characters")]
    InvalidDesktopSessionUsername,
    /// Core TLS 证书和私钥没有成对配置。
    #[error("Core TLS certificate and private key must be configured together")]
    IncompleteCoreTlsIdentity,
    /// 初始管理员用户名和密码只配置了其中一项。
    #[error("initial administrator username and password must be configured together")]
    IncompleteInitialAdminCredentials,
    /// Desktop 会话引导用户名和设备秘密只配置了其中一项。
    #[error("Desktop session username and secret must be configured together")]
    IncompleteDesktopSessionCredentials,
    /// 日志过滤器为空或只包含空白字符。
    #[error("logging filter cannot be empty")]
    EmptyLogFilter,
    /// 需要值的命令行选项没有后续参数。
    #[error("missing value for {option}")]
    MissingValue {
        /// 缺少值的命令行选项名称。
        option: &'static str,
    },
    /// 命令行选项名称不受支持。
    #[error("unsupported option: {option}")]
    UnsupportedOption {
        /// 用户传入的未知选项名称。
        option: String,
    },
    /// 用户请求显示版本并正常结束解析。
    #[error("version requested")]
    VersionRequested,
    /// 初始管理员密码不满足长度要求。
    #[error("initial administrator password must contain between 12 and 1024 bytes")]
    WeakInitialAdminPassword,
    /// Desktop 设备秘密不足以抵抗穷举攻击。
    #[error("Desktop session secret must contain between 32 and 1024 bytes")]
    WeakDesktopSessionSecret,
}
