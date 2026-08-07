//! Desktop 系统托盘和主窗口可见性管理。
//!
//! 用户关闭主窗口时只隐藏窗口，确保本地 Core/Panel 能继续提供服务；只有托盘菜单中的
//! 显式退出动作才终止 Tauri 事件循环，并由应用退出钩子停止 sidecar。

use tauri::App;
use tauri::AppHandle;
use tauri::Manager;
use tauri::Result;
use tauri::Runtime;
use tauri::Window;
use tauri::WindowEvent;
use tauri::menu::MenuBuilder;
use tauri::menu::MenuItemBuilder;
use tauri::tray::MouseButton;
use tauri::tray::TrayIconBuilder;
use tauri::tray::TrayIconEvent;
use tauri_plugin_opener::OpenerExt;

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ICON_ID: &str = "mcnp-desktop";
const OPEN_MENU_ID: &str = "open-mcnp";
const OPEN_WEB_MENU_ID: &str = "open-web-panel";
const QUIT_MENU_ID: &str = "quit-mcnp";

/// 创建系统托盘，显示本地 Panel 地址，并绑定显示主窗口和显式退出动作。
pub fn setup<R: Runtime>(app: &App<R>, panel_address: &str) -> Result<()> {
    let open_item = MenuItemBuilder::with_id(OPEN_MENU_ID, "Open MCNP").build(app)?;
    let open_web_item = MenuItemBuilder::with_id(OPEN_WEB_MENU_ID, "Open Web Panel").build(app)?;
    let quit_item = MenuItemBuilder::with_id(QUIT_MENU_ID, "Quit MCNP").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&open_item)
        .item(&open_web_item)
        .separator()
        .item(&quit_item)
        .build()?;
    let panel_url = panel_address.to_owned();

    let mut builder = TrayIconBuilder::with_id(TRAY_ICON_ID)
        .menu(&menu)
        .tooltip(format!("MCNP Panel: {panel_address}"))
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            if event.id() == OPEN_MENU_ID {
                show_main_window(app);
            } else if event.id() == OPEN_WEB_MENU_ID {
                let _ = app.opener().open_url(panel_url.clone(), None::<String>);
            } else if event.id() == QUIT_MENU_ID {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// 将用户发起的主窗口关闭请求转换为隐藏到托盘。
pub fn handle_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if window.label() == MAIN_WINDOW_LABEL
        && let WindowEvent::CloseRequested { api, .. } = event
    {
        api.prevent_close();
        let _ = window.hide();
    }
}

/// 恢复、显示并聚焦主窗口。
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

/// 隐藏主窗口，使开机启动可以无打扰地驻留托盘。
pub fn hide_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.hide();
    }
}
