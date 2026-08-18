//! Apeireth 桌面伙伴 — 薄 Tauri shell
//!
//! 窗口管理 + 托盘 + 通知 + 全局快捷键.
//! **Agent runtime 不在这里** — 对话/记忆/工具/宪法全部由 apeireth-companion
//! 后端承担 (companion_serve :8090 OpenAI 兼容端点). 本壳只负责桌面承载.

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder,
};

/// 主窗口默认尺寸
const MAIN_W: f64 = 1280.0;
const MAIN_H: f64 = 820.0;
const MAIN_MIN_W: f64 = 980.0;
const MAIN_MIN_H: f64 = 640.0;

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "打开主窗", true, None::<&str>)?;
    Menu::with_items(app, &[&show, &quit])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![ping, open_settings])
        .setup(|app| {
            let handle = app.handle().clone();

            // 主窗口
            let _main = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Apeireth 伙伴")
                .inner_size(MAIN_W, MAIN_H)
                .min_inner_size(MAIN_MIN_W, MAIN_MIN_H)
                .center()
                .resizable(true)
                .build()?;

            // 快捷窗 (Alt+Space 呼出, 先只建主窗足够; 后续 Phase 2 加 quick window)
            let _ = WebviewWindowBuilder::new(app, "quick", WebviewUrl::App("index.html?window=quick".into()))
                .title("Apeireth 快捷")
                .inner_size(440.0, 390.0)
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .build();

            // 托盘
            let menu = build_menu(&handle)?;
            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(&handle)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // 关闭主窗时隐藏到托盘, 不退出 (桌面伴随体常驻)
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running companion-desktop");
}
