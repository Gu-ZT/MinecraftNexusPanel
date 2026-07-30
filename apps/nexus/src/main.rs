use nexus_config::AppConfig;
use nexus_config::RunMode;

fn main() {
    let config = AppConfig::from_args(std::env::args().skip(1)).unwrap_or_else(|message| {
        eprintln!("{message}");
        std::process::exit(2);
    });

    match config.mode() {
        RunMode::Core => nexus_core::run(),
        RunMode::Panel => nexus_panel::run(),
        RunMode::All => {
            nexus_core::run();
            nexus_panel::run();
        }
    }
}
