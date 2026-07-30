use nexus_config::AppConfig;
use nexus_config::ConfigError;
use nexus_config::CoreConfig;
use nexus_config::PanelConfig;
use nexus_config::RunMode;
use nexus_domain::PRODUCT_NAME;
use nexus_domain::PRODUCT_VERSION;
use tracing_subscriber::EnvFilter;

#[tokio::main]
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

async fn run_all(core_config: CoreConfig, panel_config: PanelConfig) -> Result<(), String> {
    let core_server = nexus_core::CoreServer::bind(&core_config)
        .await
        .map_err(|error| error.to_string())?;
    let panel_server = nexus_panel::PanelServer::bind(&panel_config)
        .await
        .map_err(|error| error.to_string())?;
    let mut core_task = tokio::spawn(core_server.serve());

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
