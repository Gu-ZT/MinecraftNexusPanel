//! 基岩版服务端和 Geyser 的专用运维画像。

use serde::Deserialize;
use serde::Serialize;

use crate::BedrockManagementKind;
use crate::BedrockTransport;
use crate::ExtensionKind;

/// 描述基岩传输、配置文件和扩展能力的服务端画像。
///
/// 该类型只描述领域能力，不代表对应版本已经具备可执行的安装配方。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockManagementProfile {
    management_kind: BedrockManagementKind,
    transport: BedrockTransport,
    default_bind_address: String,
    default_port: u16,
    configuration_files: Vec<String>,
    extension_kind: Option<ExtensionKind>,
    extension_directories: Vec<String>,
}

impl BedrockManagementProfile {
    /// 创建一个使用默认绑定地址 `0.0.0.0` 的画像。
    #[must_use]
    pub fn new(
        management_kind: BedrockManagementKind,
        transport: BedrockTransport,
        default_port: u16,
        configuration_files: Vec<String>,
        extension_kind: Option<ExtensionKind>,
    ) -> Self {
        Self {
            management_kind,
            transport,
            default_bind_address: "0.0.0.0".to_owned(),
            default_port,
            configuration_files,
            extension_kind,
            extension_directories: Vec::new(),
        }
    }

    /// 设置该端声明的插件目录；目录由端和版本决定，不能由调用方全局假定。
    #[must_use]
    pub fn with_extension_directories(mut self, extension_directories: Vec<String>) -> Self {
        self.extension_directories = extension_directories;
        self
    }

    /// 覆盖画像默认绑定地址。
    #[must_use]
    pub fn with_default_bind_address(mut self, default_bind_address: String) -> Self {
        self.default_bind_address = default_bind_address;
        self
    }

    /// 返回具体的基岩管理类型。
    #[must_use]
    pub const fn management_kind(&self) -> BedrockManagementKind {
        self.management_kind
    }

    /// 返回基岩传输类型。
    #[must_use]
    pub const fn transport(&self) -> BedrockTransport {
        self.transport
    }

    /// 返回服务端缺少配置时使用的绑定地址。
    #[must_use]
    pub fn default_bind_address(&self) -> &str {
        &self.default_bind_address
    }

    /// 返回服务端缺少配置时使用的 UDP 端口。
    #[must_use]
    pub const fn default_port(&self) -> u16 {
        self.default_port
    }

    /// 返回按优先级排列的配置文件路径。
    #[must_use]
    pub fn configuration_files(&self) -> &[String] {
        &self.configuration_files
    }

    /// 返回该端支持管理的扩展种类。
    #[must_use]
    pub const fn extension_kind(&self) -> Option<ExtensionKind> {
        self.extension_kind
    }

    /// 返回按模板声明的扩展目录。
    #[must_use]
    pub fn extension_directories(&self) -> &[String] {
        &self.extension_directories
    }
}
