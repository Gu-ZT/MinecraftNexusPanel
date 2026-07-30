use tauri::Builder;
use tauri::Error;
use tauri::generate_context;

pub fn run() -> Result<(), Error> {
    Builder::default().run(generate_context!())
}
