//! Desktop 开机启动注册和状态查询。
//!
//! 登录项始终携带 `--minimized`，使操作系统自动启动 MCNP 时直接驻留托盘；手动启动不携带
//! 该参数，因此会在本地 Panel 就绪后显示主窗口。

use std::env;

use tauri::AppHandle;
use tauri::Runtime;
use tauri::plugin::TauriPlugin;
use tauri_plugin_autostart::Builder;
use tauri_plugin_autostart::ManagerExt;

const AUTOSTART_ARGUMENT: &str = "--minimized";

/// 创建以最小化参数注册当前可执行文件的官方开机启动插件。
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new()
        .app_name("MCNP Desktop")
        .arg(AUTOSTART_ARGUMENT)
        .build()
}

/// 判断当前进程是否由登录项以最小化模式启动。
pub fn launched_minimized() -> bool {
    env::args_os().any(|argument| argument == AUTOSTART_ARGUMENT)
}

/// 查询当前用户的 MCNP 登录项是否启用。
#[tauri::command]
pub fn desktop_autostart_enabled<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| error.to_string())
}

/// 启用或关闭当前用户的 MCNP 登录项，并返回操作系统确认后的最终状态。
#[tauri::command]
pub fn set_desktop_autostart_enabled<R: Runtime>(
    app: AppHandle<R>,
    enabled: bool,
) -> Result<bool, String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|error| error.to_string())?;
    manager.is_enabled().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::AUTOSTART_ARGUMENT;

    #[test]
    fn autostart_argument_is_stable() {
        assert_eq!(AUTOSTART_ARGUMENT, "--minimized");
    }
}
