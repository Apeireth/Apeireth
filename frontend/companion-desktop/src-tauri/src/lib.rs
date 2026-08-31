//! Apeireth 桌面伙伴 — 薄 Tauri shell
//!
//! 窗口管理 + 托盘 + 通知 + 全局快捷键 + 后端进程监督 + 生产日志.
//! **Agent runtime 不在这里** — 对话/记忆/工具/治理全部由 Apeireth Canonical Gateway
//! 后端承担 (apeireth gateway serve). 本壳只负责桌面承载与进程生命周期.

// Public so tests/supervisor_lifecycle.rs can drive the real lifecycle against a
// spawned canonical backend. The unit tests cover state without a child process;
// the Ready transition, crash detection, restart, and owned shutdown only exist
// at runtime and are exercised from that integration test.
pub mod backend_supervisor;
mod logging;

use backend_supervisor::{BackendInfo, BackendSupervisor};
use logging::{DesktopLogger, LogLevel};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State, WebviewUrl, WebviewWindowBuilder,
};

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

#[tauri::command]
async fn get_backend_status(supervisor: State<'_, Arc<BackendSupervisor>>) -> Result<BackendInfo, String> {
    Ok(supervisor.info().await)
}

#[tauri::command]
async fn start_backend(supervisor: State<'_, Arc<BackendSupervisor>>) -> Result<String, String> {
    supervisor.start().await
}

#[tauri::command]
async fn stop_backend(supervisor: State<'_, Arc<BackendSupervisor>>) -> Result<String, String> {
    supervisor.stop().await
}

#[tauri::command]
async fn restart_backend(supervisor: State<'_, Arc<BackendSupervisor>>) -> Result<String, String> {
    supervisor.restart().await
}

#[tauri::command]
fn get_log_directory(logger: State<'_, Arc<DesktopLogger>>) -> Result<String, String> {
    Ok(logger.log_directory().to_string_lossy().to_string())
}

#[tauri::command]
async fn open_log_directory(logger: State<'_, Arc<DesktopLogger>>) -> Result<(), String> {
    let log_dir = logger.log_directory();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(log_dir)
            .spawn()
            .map_err(|e| format!("Failed to open log directory: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(log_dir)
            .spawn()
            .map_err(|e| format!("Failed to open log directory: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(log_dir)
            .spawn()
            .map_err(|e| format!("Failed to open log directory: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn toggle_quick_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("quick") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "打开主窗", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quick", "快捷窗口", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    Menu::with_items(app, &[&show, &quick, &settings, &quit])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize production logger
    let logger = match DesktopLogger::new() {
        Ok(logger) => Arc::new(logger),
        Err(e) => {
            eprintln!("Failed to initialize logger: {}", e);
            std::process::exit(1);
        }
    };

    // Bound log growth before appending this session's output.
    if let Err(error) = logger.rotate_if_needed() {
        eprintln!("log rotation failed: {error}");
    }

    logger.log_desktop(
        LogLevel::Info,
        &format!("desktop.start version={}", env!("CARGO_PKG_VERSION")),
    );

    // Initialize backend supervisor with the persistent logger attached, so
    // backend stdout/stderr lands in apeireth-backend.log.
    let supervisor = Arc::new(BackendSupervisor::with_logger(logger.clone()));

    tauri::Builder::default()
        // 单实例: 二次启动聚焦已有主窗而不是再开一个 (尽量靠前注册).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .manage(supervisor.clone())
        .manage(logger.clone())
        .invoke_handler(tauri::generate_handler![
            ping,
            get_backend_status,
            start_backend,
            stop_backend,
            restart_backend,
            get_log_directory,
            open_log_directory,
            open_settings,
            toggle_quick_window
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            // Auto-start backend on app launch
            let supervisor_clone = supervisor.clone();
            let logger_clone = logger.clone();
            tauri::async_runtime::spawn(async move {
                logger_clone.log_desktop(LogLevel::Info, "Backend auto-start initiated");
                match supervisor_clone.start().await {
                    Ok(msg) => {
                        eprintln!("Backend auto-start: {}", msg);
                        logger_clone.log_desktop(LogLevel::Info, &format!("Backend auto-start success: {}", msg));
                    }
                    Err(e) => {
                        eprintln!("Backend auto-start failed: {}", e);
                        logger_clone.log_desktop(LogLevel::Error, &format!("Backend auto-start failed: {}", e));
                    }
                }
            });

            // 主窗口由 tauri.conf.json 声明 (app.windows[0] label=main), 这里不再重复创建.

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
                .tooltip("Apeireth 伙伴")
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
                    "quick" => toggle_quick_window(app.clone()),
                    "settings" => open_settings(app.clone()),
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
        .build(tauri::generate_context!())
        .expect("error building companion-desktop")
        .run(move |app_handle, event| {
            // Cleanup: stop owned backend on app exit
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(supervisor) = app_handle.try_state::<Arc<BackendSupervisor>>() {
                    tauri::async_runtime::block_on(async move {
                        let _ = supervisor.stop().await;
                    });
                }
            }
        });
}
