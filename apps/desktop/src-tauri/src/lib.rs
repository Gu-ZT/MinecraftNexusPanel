//! MCNP Desktop Tauri 容器入口。

mod desktop_autostart;
mod desktop_instance;
mod desktop_runtime;
mod desktop_tray;

use desktop_runtime::DesktopRuntime;
use desktop_runtime::DesktopRuntimeError;
use desktop_runtime::DesktopRuntimeInfo;
use tauri::Builder;
use tauri::Error;
use tauri::Manager;
use tauri::RunEvent;
use tauri::State;
use tauri::generate_context;

/// 构建并运行桌面 Tauri 应用。
pub fn run() -> Result<(), Error> {
    Builder::default()
        .plugin(desktop_instance::plugin())
        .plugin(desktop_autostart::plugin())
        .invoke_handler(tauri::generate_handler![
            desktop_runtime,
            complete_initial_admin,
            desktop_autostart::desktop_autostart_enabled,
            desktop_autostart::set_desktop_autostart_enabled
        ])
        .setup(|app| {
            let runtime = DesktopRuntime::start(app.handle()).map_err(setup_error)?;
            let panel_address = runtime.info().map_err(setup_error)?.api_base_url;
            app.manage(runtime);
            desktop_tray::setup(app, &panel_address)?;
            if desktop_autostart::launched_minimized() {
                desktop_tray::hide_main_window(app.handle());
            } else {
                desktop_tray::show_main_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(desktop_tray::handle_window_event)
        .build(generate_context!())?
        .run(|app_handle, event| {
            if matches!(event, RunEvent::Exit) {
                app_handle.state::<DesktopRuntime>().stop();
            }
        });
    Ok(())
}

/// 返回本地 Panel 地址和首次启动管理员引导凭据。
#[tauri::command]
fn desktop_runtime(state: State<'_, DesktopRuntime>) -> Result<DesktopRuntimeInfo, String> {
    state.info().map_err(|error| error.to_string())
}

/// 删除已经使用过的首次启动管理员引导密码。
#[tauri::command]
fn complete_initial_admin(state: State<'_, DesktopRuntime>) -> Result<(), String> {
    state
        .complete_initial_admin()
        .map_err(|error| error.to_string())
}

fn setup_error(error: DesktopRuntimeError) -> Box<dyn std::error::Error> {
    Box::new(error)
}
