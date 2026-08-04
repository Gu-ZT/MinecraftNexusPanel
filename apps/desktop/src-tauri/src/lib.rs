//! MCNP Desktop Tauri 容器入口。

use tauri::Builder;
use tauri::Error;
use tauri::generate_context;

/// 构建并运行桌面 Tauri 应用。
pub fn run() -> Result<(), Error> {
    Builder::default().run(generate_context!())
}
