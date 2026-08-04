//! MCNP Mobile Tauri 容器入口。

use tauri::Builder;
use tauri::Error;
use tauri::generate_context;

#[cfg(mobile)]
use tauri::mobile_entry_point;

#[cfg_attr(mobile, mobile_entry_point)]
/// 构建并运行移动端 Tauri 应用。
pub fn run() -> Result<(), Error> {
    Builder::default().run(generate_context!())
}
