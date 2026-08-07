//! MCNP Core、Panel 及本地运行模式的启动配置。
//!
//! 配置 crate 只负责解析和校验启动输入，不读取或创建运行时数据文件；通过
//! `Debug` 输出配置时，预共享密钥、主密钥和初始密码必须保持脱敏。

mod app_config;
mod config_error;
mod core_config;
mod desktop_session_config;
mod initial_admin_config;
mod local_core_config;
mod logging_config;
mod panel_config;
mod panel_master_key;
mod run_mode;

pub use app_config::AppConfig;
pub use config_error::ConfigError;
pub use core_config::CoreConfig;
pub use desktop_session_config::DesktopSessionConfig;
pub use initial_admin_config::InitialAdminConfig;
pub use local_core_config::LocalCoreConfig;
pub use logging_config::LoggingConfig;
pub use panel_config::PanelConfig;
pub use panel_master_key::PanelMasterKey;
pub use run_mode::RunMode;
