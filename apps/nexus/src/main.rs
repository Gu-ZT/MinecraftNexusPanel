//! MCNP 命令行服务入口。
//!
//! 入口负责解析启动配置、初始化日志，并按 Core、Panel 或 All 模式启动对应服务。
//! All 模式会在同一进程中绑定 Core，再把本地 Core 连接信息注入 Panel。

use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use nexus_config::AppConfig;
use nexus_config::ConfigError;
use nexus_config::CoreConfig;
use nexus_config::LocalCoreConfig;
use nexus_config::PanelConfig;
use nexus_config::RunMode;
use nexus_domain::PRODUCT_NAME;
use nexus_domain::PRODUCT_VERSION;
use tracing_subscriber::EnvFilter;

#[tokio::main]
/// 解析配置并启动 MCNP 服务。
async fn main() {
    let config = match AppConfig::from_args(std::env::args().skip(1)) {
        Ok(config) => config,
        Err(ConfigError::HelpRequested) => {
            println!("{}", AppConfig::usage());
            return;
        }
        Err(ConfigError::VersionRequested) => {
            println!("{PRODUCT_NAME} {PRODUCT_VERSION}");
            return;
        }
        Err(error) => {
            eprintln!("{error}\n\n{}", AppConfig::usage());
            std::process::exit(2);
        }
    };

    initialize_logging(config.logging().filter());
    tracing::info!(
        product = PRODUCT_NAME,
        version = PRODUCT_VERSION,
        mode = ?config.mode(),
        "Starting MCNP"
    );

    let result = match config.mode() {
        RunMode::Core => nexus_core::run(config.core())
            .await
            .map_err(|error| error.to_string()),
        RunMode::Panel => nexus_panel::run(config.panel())
            .await
            .map_err(|error| error.to_string()),
        RunMode::All => run_all(config.core().clone(), config.panel().clone()).await,
    };

    if let Err(error) = result {
        tracing::error!(error = %error, "MCNP stopped with an error");
        std::process::exit(1);
    }
}

/// 在同一进程内协调 Core 和 Panel 的生命周期。
async fn run_all(core_config: CoreConfig, panel_config: PanelConfig) -> Result<(), String> {
    let core_server = nexus_core::CoreServer::bind(&core_config)
        .await
        .map_err(|error| error.to_string())?;
    let encoded_pre_shared_key = core_config
        .encoded_pre_shared_key()
        .ok_or("Core pre-shared key is required in all mode")?
        .to_owned();
    let local_core = LocalCoreConfig::new(
        core_server.core_id(),
        loopback_address(core_server.listen_address()),
        encoded_pre_shared_key,
    );
    let panel_config = panel_config.with_local_core(local_core);
    let mut core_task = tokio::spawn(core_server.serve());
    let panel_server = match nexus_panel::PanelServer::bind(&panel_config).await {
        Ok(panel_server) => panel_server,
        Err(error) => {
            core_task.abort();
            let _ = core_task.await;
            return Err(error.to_string());
        }
    };

    tokio::select! {
        core_result = &mut core_task => {
            match core_result {
                Ok(Ok(())) => Err("Core stopped unexpectedly".to_owned()),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(format!("Core task failed: {error}")),
            }
        }
        panel_result = panel_server.serve() => {
            core_task.abort();
            let _ = core_task.await;
            panel_result.map_err(|error| error.to_string())
        }
    }
}

/// 将未指定监听地址转换为 Panel 可连接的本机回环地址。
fn loopback_address(address: SocketAddr) -> SocketAddr {
    if address.ip().is_unspecified() {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), address.port())
    } else {
        address
    }
}

/// 初始化结构化日志过滤器；非法过滤器回退到 `info`。
fn initialize_logging(filter: &str) {
    let filter = EnvFilter::try_new(filter).unwrap_or_else(|error| {
        eprintln!("Invalid log filter; falling back to info: {error}");
        EnvFilter::new("info")
    });

    if let Err(error) = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init()
    {
        eprintln!("Unable to initialize structured logging: {error}");
    }
}
