use tauri::Builder;
use tauri::Error;
use tauri::generate_context;

#[cfg(mobile)]
use tauri::mobile_entry_point;

#[cfg_attr(mobile, mobile_entry_point)]
pub fn run() -> Result<(), Error> {
    Builder::default().run(generate_context!())
}
