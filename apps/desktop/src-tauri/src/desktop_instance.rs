//! Desktop 单实例约束和重复启动处理。
//!
//! 第二次启动不会创建新的 sidecar 或窗口，而是把启动请求转交给首个实例；首个实例再
//! 恢复并聚焦已有主窗口。该插件必须在其他插件之前注册，避免重复启动流程被后续插件
//! 先行初始化。

use crate::desktop_tray;
use tauri::Runtime;
use tauri::plugin::TauriPlugin;
use tauri_plugin_single_instance::init;

/// 创建桌面单实例插件，并把重复启动转换为主窗口唤醒。
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    init(|app, _arguments, _working_directory| {
        desktop_tray::show_main_window(app);
    })
}
