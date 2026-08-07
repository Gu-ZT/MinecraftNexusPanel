//! MCNP Desktop Tauri 容器入口。

use std::time::Duration;

mod desktop_autostart;
mod desktop_credentials;
mod desktop_instance;
mod desktop_logs;
mod desktop_runtime;
mod desktop_tray;

use desktop_runtime::DesktopRuntime;
use desktop_runtime::DesktopRuntimeError;
use desktop_runtime::DesktopRuntimeInfo;
use reqwest::Client;
use reqwest::header::AUTHORIZATION;
use rustls::crypto::ring;
use serde_json::Value;
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
        .plugin(desktop_logs::plugin())
        .invoke_handler(tauri::generate_handler![
            desktop_runtime,
            desktop_session,
            desktop_autostart::desktop_autostart_enabled,
            desktop_autostart::set_desktop_autostart_enabled,
            desktop_logs::open_desktop_log_directory,
            desktop_credentials::get_desktop_refresh_token,
            desktop_credentials::set_desktop_refresh_token,
            desktop_credentials::clear_desktop_refresh_token
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

/// 使用只存在于 Tauri 和本地 sidecar 中的设备秘密换取原生会话。
#[tauri::command]
async fn desktop_session(state: State<'_, DesktopRuntime>) -> Result<Value, String> {
    let (endpoint, secret) = state
        .desktop_session_request()
        .map_err(|error| error.to_string())?;
    let client = desktop_session_client().map_err(|error| error.to_string())?;
    let response = client
        .post(endpoint)
        .header(AUTHORIZATION, format!("MCNP-Desktop {secret}"))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "local Desktop session request failed with {status}"
        ));
    }
    let body = response.bytes().await.map_err(|error| error.to_string())?;
    let session = serde_json::from_slice::<Value>(&body).map_err(|error| error.to_string())?;
    state
        .complete_initial_admin()
        .map_err(|error| error.to_string())?;
    Ok(session)
}

/// 构建只访问本机 Panel 的短超时客户端，并安装工作区选定的 rustls provider。
fn desktop_session_client() -> Result<Client, reqwest::Error> {
    let _ = ring::default_provider().install_default();
    Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
}

fn setup_error(error: DesktopRuntimeError) -> Box<dyn std::error::Error> {
    Box::new(error)
}

#[cfg(test)]
mod tests {
    use super::desktop_session_client;

    #[test]
    fn builds_the_loopback_desktop_session_client() {
        desktop_session_client().expect("Desktop session client should build");
    }
}
