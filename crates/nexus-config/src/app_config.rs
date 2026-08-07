use std::env;
use std::path::PathBuf;

use crate::ConfigError;
use crate::CoreConfig;
use crate::DesktopSessionConfig;
use crate::InitialAdminConfig;
use crate::LoggingConfig;
use crate::PanelConfig;
use crate::PanelMasterKey;
use crate::RunMode;

/// 解析后的完整进程启动配置。
///
/// 命令行选项覆盖同名环境变量，未指定运行模式时默认为 [`RunMode::All`]。
/// 该类型只保存校验后的值，具体服务启动仍由应用入口负责。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    mode: RunMode,
    core: CoreConfig,
    panel: PanelConfig,
    logging: LoggingConfig,
}

impl AppConfig {
    /// 从命令行参数和环境变量构造启动配置。
    ///
    /// 参数迭代器不应包含程序名；模式可以使用位置参数或 `--mode` 指定，
    /// 但只能出现一次。`--help` 和 `--version` 会通过专用错误返回控制流结果。
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, ConfigError> {
        let mut mode = None;
        let mut core_listen =
            environment_or_default("MCNP_CORE_LISTEN", CoreConfig::DEFAULT_LISTEN_ADDRESS);
        let mut panel_listen =
            environment_or_default("MCNP_PANEL_LISTEN", PanelConfig::DEFAULT_LISTEN_ADDRESS);
        let mut panel_audit_retention_events = environment_optional("MCNP_AUDIT_RETENTION_EVENTS")
            .unwrap_or_else(|| PanelConfig::DEFAULT_AUDIT_RETENTION_EVENTS.to_string());
        let mut panel_web_root = environment_optional("MCNP_PANEL_WEB_ROOT");
        let mut data_directory = environment_or_default("MCNP_DATA_DIR", "data");
        let mut log_filter = environment_or_default("MCNP_LOG_FILTER", "info");
        let core_pre_shared_key = environment_optional("MCNP_CORE_PSK");
        let mut core_tls_certificate = environment_optional("MCNP_CORE_TLS_CERT");
        let mut core_tls_private_key = environment_optional("MCNP_CORE_TLS_KEY");
        let initial_admin_username = environment_optional("MCNP_INITIAL_ADMIN_USERNAME");
        let initial_admin_password = environment_optional("MCNP_INITIAL_ADMIN_PASSWORD");
        let desktop_session_username = environment_optional("MCNP_DESKTOP_SESSION_USERNAME");
        let desktop_session_secret = environment_optional("MCNP_DESKTOP_SESSION_SECRET");
        let panel_master_key = environment_optional("MCNP_PANEL_MASTER_KEY")
            .map(|value| PanelMasterKey::from_base64url(&value))
            .transpose()?;
        let mut arguments = args.into_iter();

        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "core" | "panel" | "all" => {
                    if mode
                        .replace(argument.parse().map_err(|_| ConfigError::InvalidMode {
                            value: argument.clone(),
                        })?)
                        .is_some()
                    {
                        return Err(ConfigError::DuplicateMode);
                    }
                }
                "--mode" => {
                    let value = next_value(&mut arguments, "--mode")?;
                    if mode
                        .replace(value.parse().map_err(|_| ConfigError::InvalidMode {
                            value: value.clone(),
                        })?)
                        .is_some()
                    {
                        return Err(ConfigError::DuplicateMode);
                    }
                }
                "--core-listen" => core_listen = next_value(&mut arguments, "--core-listen")?,
                "--core-tls-cert" => {
                    core_tls_certificate = Some(next_value(&mut arguments, "--core-tls-cert")?);
                }
                "--core-tls-key" => {
                    core_tls_private_key = Some(next_value(&mut arguments, "--core-tls-key")?);
                }
                "--panel-listen" => panel_listen = next_value(&mut arguments, "--panel-listen")?,
                "--panel-audit-retention-events" => {
                    panel_audit_retention_events =
                        next_value(&mut arguments, "--panel-audit-retention-events")?;
                }
                "--panel-web-root" => {
                    panel_web_root = Some(next_value(&mut arguments, "--panel-web-root")?);
                }
                "--data-dir" => data_directory = next_value(&mut arguments, "--data-dir")?,
                "--log-filter" => log_filter = next_value(&mut arguments, "--log-filter")?,
                "--help" | "-h" => return Err(ConfigError::HelpRequested),
                "--version" | "-V" => return Err(ConfigError::VersionRequested),
                _ if argument.starts_with("--") => {
                    return Err(ConfigError::UnsupportedOption { option: argument });
                }
                _ => return Err(ConfigError::InvalidMode { value: argument }),
            }
        }

        let core = CoreConfig::new(
            core_listen,
            PathBuf::from(&data_directory),
            core_pre_shared_key,
        )?
        .with_tls_identity_paths(
            core_tls_certificate.map(PathBuf::from),
            core_tls_private_key.map(PathBuf::from),
        )?;
        let initial_admin = match (initial_admin_username, initial_admin_password) {
            (Some(username), Some(password)) => Some(InitialAdminConfig::new(username, password)?),
            (None, None) => None,
            _ => return Err(ConfigError::IncompleteInitialAdminCredentials),
        };
        let desktop_session = match (desktop_session_username, desktop_session_secret) {
            (Some(username), Some(secret)) => Some(DesktopSessionConfig::new(username, secret)?),
            (None, None) => None,
            _ => return Err(ConfigError::IncompleteDesktopSessionCredentials),
        };
        let audit_retention_events =
            panel_audit_retention_events.parse::<usize>().map_err(|_| {
                ConfigError::InvalidPanelAuditRetention {
                    value: panel_audit_retention_events.clone(),
                }
            })?;
        let mut panel = PanelConfig::new(panel_listen, PathBuf::from(data_directory))?
            .with_audit_retention_events(audit_retention_events)?;
        if let Some(initial_admin) = initial_admin {
            panel = panel.with_initial_admin(initial_admin);
        }
        if let Some(desktop_session) = desktop_session {
            panel = panel.with_desktop_session(desktop_session);
        }
        if let Some(master_key) = panel_master_key {
            panel = panel.with_master_key(master_key);
        }
        if let Some(web_root) = panel_web_root {
            panel = panel.with_web_root(PathBuf::from(web_root));
        }
        let logging = LoggingConfig::new(log_filter)?;

        Ok(Self {
            mode: mode.unwrap_or(RunMode::All),
            core,
            panel,
            logging,
        })
    }

    /// 返回进程运行模式。
    #[must_use]
    pub const fn mode(&self) -> RunMode {
        self.mode
    }

    /// 返回 Core 配置。
    #[must_use]
    pub const fn core(&self) -> &CoreConfig {
        &self.core
    }

    /// 返回 Panel 配置。
    #[must_use]
    pub const fn panel(&self) -> &PanelConfig {
        &self.panel
    }

    /// 返回日志配置。
    #[must_use]
    pub const fn logging(&self) -> &LoggingConfig {
        &self.logging
    }

    /// 返回命令行帮助文本。
    #[must_use]
    pub const fn usage() -> &'static str {
        concat!(
            "Usage: mcnp [core|panel|all] [OPTIONS]\n\n",
            "Options:\n",
            "  --mode MODE              Run core, panel, or all\n",
            "  --core-listen ADDRESS    Core TCP listen address\n",
            "  --core-tls-cert PATH     Core TLS certificate chain in PEM format\n",
            "  --core-tls-key PATH      Core TLS private key in PEM format\n",
            "  --panel-listen ADDRESS   Panel HTTP listen address\n",
            "  --panel-audit-retention-events COUNT\n",
            "                           Retained Panel audit event count (100-100000)\n",
            "  --panel-web-root PATH    Static WebUI directory served by Panel\n",
            "  --data-dir PATH          Runtime data directory\n",
            "  --log-filter FILTER      tracing filter directive\n",
            "  -h, --help               Print help\n",
            "  -V, --version            Print version\n\n",
            "Environment:\n",
            "  MCNP_CORE_PSK            Required by core and all; unpadded Base64URL PSK\n",
            "  MCNP_CORE_LISTEN          Default Core TCP listen address\n",
            "  MCNP_CORE_TLS_CERT        Optional Core TLS certificate chain path\n",
            "  MCNP_CORE_TLS_KEY         Optional Core TLS private key path\n",
            "  MCNP_PANEL_LISTEN         Default Panel HTTP listen address\n",
            "  MCNP_PANEL_MASTER_KEY     Required by panel and all; 32-byte unpadded Base64URL key\n",
            "  MCNP_PANEL_WEB_ROOT       Static WebUI directory served by Panel\n",
            "  MCNP_AUDIT_RETENTION_EVENTS  Retained Panel audit event count (default 10000)\n",
            "  MCNP_INITIAL_ADMIN_USERNAME  Initial administrator username for an empty database\n",
            "  MCNP_INITIAL_ADMIN_PASSWORD  Initial administrator password for an empty database\n",
            "  MCNP_DESKTOP_SESSION_USERNAME  Local Desktop account for trusted native sessions\n",
            "  MCNP_DESKTOP_SESSION_SECRET    Local Desktop device secret (32-1024 bytes)\n",
            "  MCNP_DATA_DIR             Default runtime data directory\n",
            "  MCNP_LOG_FILTER           Default tracing filter directive\n",
            "  MCNP_LOG_FORMAT           Set to json for line-delimited JSON logs",
        )
    }
}

fn environment_or_default(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn environment_optional(name: &str) -> Option<String> {
    env::var(name).ok()
}

fn next_value(
    arguments: &mut impl Iterator<Item = String>,
    option: &'static str,
) -> Result<String, ConfigError> {
    arguments.next().ok_or(ConfigError::MissingValue { option })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::AppConfig;
    use crate::ConfigError;
    use crate::RunMode;

    #[test]
    fn parses_explicit_runtime_settings() {
        let arguments = [
            "core",
            "--core-listen",
            "127.0.0.1:25580",
            "--core-tls-cert",
            "test-cert.pem",
            "--core-tls-key",
            "test-key.pem",
            "--panel-listen",
            "127.0.0.1:8080",
            "--panel-audit-retention-events",
            "2500",
            "--panel-web-root",
            "web-ui",
            "--data-dir",
            "runtime-data",
            "--log-filter",
            "debug",
        ]
        .into_iter()
        .map(str::to_owned);

        let config = AppConfig::from_args(arguments).expect("explicit settings are valid");

        assert_eq!(config.mode(), RunMode::Core);
        assert_eq!(
            config.core().listen_address().to_string(),
            "127.0.0.1:25580"
        );
        assert_eq!(
            config.panel().listen_address().to_string(),
            "127.0.0.1:8080"
        );
        assert_eq!(
            config.core().data_directory().to_string_lossy(),
            "runtime-data"
        );
        assert_eq!(
            config
                .core()
                .tls_certificate_path()
                .expect("TLS certificate path is configured"),
            PathBuf::from("test-cert.pem")
        );
        assert_eq!(config.logging().filter(), "debug");
        assert_eq!(config.panel().audit_retention_events(), 2500);
        assert_eq!(
            config
                .panel()
                .web_root()
                .expect("Panel WebUI root is configured"),
            PathBuf::from("web-ui")
        );
    }

    #[test]
    fn rejects_multiple_modes() {
        let arguments = ["core", "panel"].into_iter().map(str::to_owned);

        assert!(AppConfig::from_args(arguments).is_err());
    }

    #[test]
    fn recognizes_a_version_request() {
        let arguments = ["--version"].into_iter().map(str::to_owned);

        assert_eq!(
            AppConfig::from_args(arguments),
            Err(ConfigError::VersionRequested)
        );
    }

    #[test]
    fn rejects_an_out_of_range_audit_retention() {
        let arguments = ["panel", "--panel-audit-retention-events", "1"]
            .into_iter()
            .map(str::to_owned);

        assert_eq!(
            AppConfig::from_args(arguments),
            Err(ConfigError::InvalidPanelAuditRetention {
                value: "1".to_owned(),
            })
        );
    }
}
