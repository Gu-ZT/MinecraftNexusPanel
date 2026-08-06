//! Desktop 本地 Core/Panel sidecar 的启动、秘密引导和生命周期管理。
//!
//! sidecar 由 Tauri 进程托管，使用动态 loopback 端口和仅存在于当前用户数据目录的秘密
//! 启动；其输出管道由 `desktop_logs` 模块异步收集，避免桌面壳吞掉故障诊断信息。

use std::fs;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use getrandom::fill;
use serde::Deserialize;
use serde::Serialize;
use tauri::AppHandle;
use tauri::Manager;
use tauri::Runtime;
use thiserror::Error;

use crate::desktop_logs::LOG_DIRECTORY_NAME;
use crate::desktop_logs::prepare_sidecar_log;
use crate::desktop_logs::redact_sensitive_fields;

const DATA_DIRECTORY_NAME: &str = "data";
const SECRETS_FILE_NAME: &str = "desktop-secrets.json";
const ADMIN_USERNAME: &str = "admin";
const PANEL_PORT_START: u16 = 18_080;
const CORE_PORT_START: u16 = 25_580;
const PORT_SEARCH_LIMIT: u16 = 32;
const PANEL_STARTUP_ATTEMPTS: usize = 100;
const PANEL_STARTUP_INTERVAL: Duration = Duration::from_millis(100);

/// Desktop 启动本地服务后提供给前端的运行时信息。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRuntimeInfo {
    /// 本地 Panel HTTP API 地址。
    pub api_base_url: String,
    /// 首次初始化管理员的用户名。
    pub initial_admin_username: String,
    /// 尚未完成首次登录时显示的一次性引导密码。
    pub initial_admin_password: Option<String>,
}

/// Desktop 首次启动生成并持久化的本地秘密。
///
/// Panel 主密钥和 Core PSK 必须跨进程重启保持不变，否则已有 Core 注册信息无法解密。
/// 引导密码只在首位管理员完成首次登录前保留，完成后会从文件中删除。
#[derive(Clone, Debug, Deserialize, Serialize)]
struct DesktopSecrets {
    panel_master_key: String,
    core_psk: String,
    initial_admin_username: String,
    initial_admin_password: Option<String>,
}

/// Desktop 管理的本地 MCNP sidecar 生命周期和引导状态。
pub struct DesktopRuntime {
    child: Mutex<Option<Child>>,
    secrets_path: PathBuf,
    secrets: Mutex<DesktopSecrets>,
    info: Mutex<DesktopRuntimeInfo>,
}

impl DesktopRuntime {
    /// 生成或读取本地秘密，启动 `mcnp all`，并等待 Panel 监听器就绪。
    pub fn start<R: Runtime>(app: &AppHandle<R>) -> Result<Self, DesktopRuntimeError> {
        let app_data_directory = app
            .path()
            .app_data_dir()
            .map_err(|error| DesktopRuntimeError::AppPath(error.to_string()))?;
        fs::create_dir_all(&app_data_directory).map_err(|source| DesktopRuntimeError::Io {
            path: app_data_directory.clone(),
            source,
        })?;

        let secrets_path = app_data_directory.join(SECRETS_FILE_NAME);
        let secrets = DesktopSecrets::load_or_create(&secrets_path)?;
        let panel_address = select_loopback_port(PANEL_PORT_START)?;
        let core_address = select_loopback_port(CORE_PORT_START)?;
        let data_directory = app_data_directory.join(DATA_DIRECTORY_NAME);
        let sidecar_path = locate_sidecar(app)?;
        let (log_path, log_file) =
            prepare_sidecar_log(&app_data_directory).map_err(|source| DesktopRuntimeError::Io {
                path: app_data_directory.join(LOG_DIRECTORY_NAME),
                source,
            })?;
        let mut child = spawn_sidecar(
            &sidecar_path,
            &app_data_directory,
            &data_directory,
            panel_address,
            core_address,
            &secrets,
        )?;
        if let Err(error) = attach_sidecar_logs(&mut child, log_path, log_file) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }

        if let Err(error) = wait_for_panel(&mut child, panel_address) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }

        let info = DesktopRuntimeInfo {
            api_base_url: format!("http://{panel_address}"),
            initial_admin_username: secrets.initial_admin_username.clone(),
            initial_admin_password: secrets.initial_admin_password.clone(),
        };

        Ok(Self {
            child: Mutex::new(Some(child)),
            secrets_path,
            secrets: Mutex::new(secrets),
            info: Mutex::new(info),
        })
    }

    /// 返回当前 Desktop 运行时信息。
    pub fn info(&self) -> Result<DesktopRuntimeInfo, DesktopRuntimeError> {
        self.info
            .lock()
            .map(|info| info.clone())
            .map_err(|_| DesktopRuntimeError::StatePoisoned)
    }

    /// 标记首位管理员已完成登录，并删除引导密码。
    pub fn complete_initial_admin(&self) -> Result<(), DesktopRuntimeError> {
        let mut secrets = self
            .secrets
            .lock()
            .map_err(|_| DesktopRuntimeError::StatePoisoned)?;
        let previous_password = secrets.initial_admin_password.take();
        if previous_password.is_none() {
            return Ok(());
        }

        if let Err(error) = secrets.persist(&self.secrets_path) {
            secrets.initial_admin_password = previous_password;
            return Err(error);
        }

        let mut info = self
            .info
            .lock()
            .map_err(|_| DesktopRuntimeError::StatePoisoned)?;
        info.initial_admin_password = None;
        Ok(())
    }

    /// 停止本地 MCNP sidecar，避免应用退出后留下 Core/Panel 进程。
    pub fn stop(&self) {
        let Ok(mut child_slot) = self.child.lock() else {
            return;
        };
        let Some(mut child) = child_slot.take() else {
            return;
        };
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for DesktopRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}

impl DesktopSecrets {
    fn load_or_create(path: &Path) -> Result<Self, DesktopRuntimeError> {
        if path.exists() {
            let content = fs::read(path).map_err(|source| DesktopRuntimeError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            let secrets: Self = serde_json::from_slice(&content)?;
            secrets.validate()?;
            return Ok(secrets);
        }

        let secrets = Self {
            panel_master_key: random_base64url(32)?,
            core_psk: random_base64url(32)?,
            initial_admin_username: ADMIN_USERNAME.to_owned(),
            initial_admin_password: Some(random_base64url(24)?),
        };
        secrets.persist(path)?;
        Ok(secrets)
    }

    fn persist(&self, path: &Path) -> Result<(), DesktopRuntimeError> {
        let content = serde_json::to_vec_pretty(self)?;
        fs::write(path, content).map_err(|source| DesktopRuntimeError::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    fn validate(&self) -> Result<(), DesktopRuntimeError> {
        if self.panel_master_key.is_empty()
            || self.core_psk.is_empty()
            || self.initial_admin_username.is_empty()
            || self
                .initial_admin_password
                .as_deref()
                .is_some_and(str::is_empty)
        {
            return Err(DesktopRuntimeError::InvalidSecrets);
        }
        Ok(())
    }
}

fn random_base64url(byte_count: usize) -> Result<String, DesktopRuntimeError> {
    let mut bytes = vec![0_u8; byte_count];
    fill(&mut bytes).map_err(|error| DesktopRuntimeError::Random(error.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn select_loopback_port(start: u16) -> Result<SocketAddr, DesktopRuntimeError> {
    for offset in 0..PORT_SEARCH_LIMIT {
        let Some(port) = start.checked_add(offset) else {
            break;
        };
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        if TcpListener::bind(address).is_ok() {
            return Ok(address);
        }
    }
    Err(DesktopRuntimeError::PortUnavailable { start })
}

fn locate_sidecar<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, DesktopRuntimeError> {
    let executable_name = if cfg!(windows) { "mcnp.exe" } else { "mcnp" };
    let mut candidates = Vec::new();
    if let Ok(resource_directory) = app.path().resource_dir() {
        candidates.push(resource_directory.join("binaries").join(executable_name));
        candidates.push(resource_directory.join(executable_name));
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(executable_name),
    );
    if let Ok(current_executable) = std::env::current_exe()
        && let Some(parent) = current_executable.parent()
    {
        candidates.push(parent.join(executable_name));
        candidates.push(parent.join("binaries").join(executable_name));
    }

    if let Some(path) = candidates.iter().find(|path| path.is_file()) {
        return Ok(path.clone());
    }
    Err(DesktopRuntimeError::SidecarNotFound {
        candidates: candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("; "),
    })
}

fn spawn_sidecar(
    sidecar_path: &Path,
    app_data_directory: &Path,
    data_directory: &Path,
    panel_address: SocketAddr,
    core_address: SocketAddr,
    secrets: &DesktopSecrets,
) -> Result<Child, DesktopRuntimeError> {
    let mut command = Command::new(sidecar_path);
    command
        .arg("all")
        .current_dir(app_data_directory)
        .env("MCNP_DATA_DIR", data_directory)
        .env("MCNP_PANEL_LISTEN", panel_address.to_string())
        .env("MCNP_CORE_LISTEN", core_address.to_string())
        .env("MCNP_CORE_PSK", &secrets.core_psk)
        .env("MCNP_PANEL_MASTER_KEY", &secrets.panel_master_key)
        .env("MCNP_LOG_FILTER", "info")
        .env("MCNP_LOG_FORMAT", "json")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(password) = &secrets.initial_admin_password {
        command.env(
            "MCNP_INITIAL_ADMIN_USERNAME",
            &secrets.initial_admin_username,
        );
        command.env("MCNP_INITIAL_ADMIN_PASSWORD", password);
    }
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);

    command
        .spawn()
        .map_err(|source| DesktopRuntimeError::Spawn {
            path: sidecar_path.to_path_buf(),
            source,
        })
}

/// 将 sidecar 两条输出管道交给独立线程消费，避免子进程因缓冲区填满而停顿。
fn attach_sidecar_logs(
    child: &mut Child,
    log_path: PathBuf,
    log_file: fs::File,
) -> Result<(), DesktopRuntimeError> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| DesktopRuntimeError::LogPipe {
            path: log_path.clone(),
            stream: "stdout",
        })?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| DesktopRuntimeError::LogPipe {
            path: log_path.clone(),
            stream: "stderr",
        })?;
    let shared_log_file = Arc::new(Mutex::new(log_file));
    spawn_log_reader(stdout, Arc::clone(&shared_log_file), "stdout");
    spawn_log_reader(stderr, shared_log_file, "stderr");
    Ok(())
}

/// 在后台线程中转发一条 sidecar 输出流，并串行写入共享日志文件。
fn spawn_log_reader<R: Read + Send + 'static>(
    stream: R,
    log_file: Arc<Mutex<fs::File>>,
    stream_name: &'static str,
) {
    let _ = thread::Builder::new()
        .name(format!("mcnp-sidecar-{stream_name}"))
        .spawn(move || {
            for line in BufReader::new(stream).lines() {
                let Ok(line) = line else {
                    break;
                };
                let Ok(mut file) = log_file.lock() else {
                    break;
                };
                let line = redact_sensitive_fields(&line);
                let _ = writeln!(file, "[{stream_name}] {line}");
                let _ = file.flush();
            }
        });
}

fn wait_for_panel(child: &mut Child, address: SocketAddr) -> Result<(), DesktopRuntimeError> {
    for _ in 0..PANEL_STARTUP_ATTEMPTS {
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return Ok(());
        }
        if let Some(status) = child
            .try_wait()
            .map_err(DesktopRuntimeError::WaitForSidecar)?
        {
            return Err(DesktopRuntimeError::SidecarExited(status.code()));
        }
        sleep(PANEL_STARTUP_INTERVAL);
    }
    Err(DesktopRuntimeError::PanelStartupTimeout { address })
}

/// Desktop 本地服务启动、凭据和 sidecar 生命周期错误。
#[derive(Debug, Error)]
pub enum DesktopRuntimeError {
    /// 无法访问 Tauri 应用目录。
    #[error("unable to resolve the Desktop application directory: {0}")]
    AppPath(String),
    /// 文件读写失败。
    #[error("unable to access Desktop runtime file {path}: {source}")]
    Io {
        /// 出错的文件路径。
        path: PathBuf,
        /// 底层 IO 错误。
        #[source]
        source: io::Error,
    },
    /// JSON 秘密文件无法解析或序列化。
    #[error("Desktop runtime secrets are invalid: {0}")]
    Json(#[from] serde_json::Error),
    /// 操作系统随机源不可用。
    #[error("unable to generate Desktop runtime secrets: {0}")]
    Random(String),
    /// 读取到结构不完整的秘密文件。
    #[error("Desktop runtime secrets are incomplete")]
    InvalidSecrets,
    /// 没有找到随应用发布的 mcnp sidecar。
    #[error("mcnp sidecar was not found; checked: {candidates}")]
    SidecarNotFound {
        /// 已检查的候选路径。
        candidates: String,
    },
    /// sidecar 无法启动。
    #[error("unable to start mcnp sidecar {path}: {source}")]
    Spawn {
        /// sidecar 路径。
        path: PathBuf,
        /// 底层 IO 错误。
        #[source]
        source: io::Error,
    },
    /// 等待 sidecar 退出状态失败。
    #[error("unable to inspect mcnp sidecar: {0}")]
    WaitForSidecar(#[source] io::Error),
    /// sidecar 启动后立即退出。
    #[error("mcnp sidecar exited before Panel became ready with code {0:?}")]
    SidecarExited(Option<i32>),
    /// sidecar 输出管道未按预期创建。
    #[error("unable to capture sidecar {stream} output in {path}")]
    LogPipe {
        /// 日志文件路径。
        path: PathBuf,
        /// 输出流名称。
        stream: &'static str,
    },
    /// Panel 在限定时间内没有监听。
    #[error("Panel did not become ready at {address}")]
    PanelStartupTimeout {
        /// Panel 预期监听地址。
        address: SocketAddr,
    },
    /// 端口搜索失败。
    #[error("no available loopback port found starting at {start}")]
    PortUnavailable {
        /// 搜索起始端口。
        start: u16,
    },
    /// 运行时状态锁被异常中毒。
    #[error("Desktop runtime state is unavailable")]
    StatePoisoned,
}
