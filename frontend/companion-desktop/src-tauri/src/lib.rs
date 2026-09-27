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
mod keychain;
mod logging;
mod provider_proxy;
mod stored_config;
pub mod workspace;

use backend_supervisor::{
    BackendCapabilityEnv, BackendInfo, BackendProviderEnv, BackendSupervisor,
};
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
async fn get_backend_status(
    supervisor: State<'_, Arc<BackendSupervisor>>,
) -> Result<BackendInfo, String> {
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

/// Apply the Settings-UI provider configuration to the sidecar environment.
///
/// Keys live in supervisor memory and the child's environment only; they are
/// never persisted (see `BackendProviderEnv`). Returns the safe `BackendInfo`
/// so the frontend can adopt the (possibly new) gateway endpoint.
#[tauri::command]
async fn apply_backend_provider_env(
    supervisor: State<'_, Arc<BackendSupervisor>>,
    env: BackendProviderEnv,
) -> Result<BackendInfo, String> {
    supervisor.apply_provider_env(env).await
}

/// Apply the Settings-UI configuration (provider env + advanced-capability
/// toggles) in one call, so a save that changed both restarts the backend
/// exactly once. Either part may be omitted (no-op for that part).
#[tauri::command]
async fn apply_backend_config(
    supervisor: State<'_, Arc<BackendSupervisor>>,
    provider: Option<BackendProviderEnv>,
    capabilities: Option<BackendCapabilityEnv>,
) -> Result<BackendInfo, String> {
    supervisor
        .apply_backend_config(provider, capabilities)
        .await
}

/// Fetch the gateway's effective config for the settings UI (wave-2 echo-back).
#[tauri::command]
async fn get_gateway_effective_config(
    supervisor: State<'_, Arc<BackendSupervisor>>,
) -> Result<serde_json::Value, String> {
    supervisor.get_gateway_effective_config().await
}

/// Read a provider key from the OS keychain. `None` when not stored.
#[tauri::command]
fn get_provider_key(provider: String) -> Option<String> {
    keychain::get_provider_key(&provider)
}

/// Persist a provider key to the OS keychain (never to disk).
#[tauri::command]
fn set_provider_key(provider: String, key: String) -> Result<(), String> {
    keychain::set_provider_key(&provider, &key)
}

/// Remove a provider key from the OS keychain.
#[tauri::command]
fn delete_provider_key(provider: String) -> Result<(), String> {
    keychain::delete_provider_key(&provider)
}

/// Whether a provider key is currently stored.
#[tauri::command]
fn has_provider_key(provider: String) -> bool {
    keychain::has_provider_key(&provider)
}

/// The current workspace directory, or an empty string when unset.
#[tauri::command]
async fn get_workspace_dir(
    supervisor: State<'_, Arc<BackendSupervisor>>,
) -> Result<String, String> {
    Ok(supervisor.get_workspace_dir().await)
}

/// Validate, persist and apply a workspace directory, returning the new value.
#[tauri::command]
async fn set_workspace_dir(
    supervisor: State<'_, Arc<BackendSupervisor>>,
    dir: String,
) -> Result<String, String> {
    supervisor.set_workspace_dir(dir).await
}

/// Common workspace candidates: home, documents, desktop, last-used.
#[tauri::command]
async fn list_workspace_suggestions(
    supervisor: State<'_, Arc<BackendSupervisor>>,
) -> Result<Vec<String>, String> {
    Ok(supervisor.list_workspace_suggestions().await)
}

#[tauri::command]
fn get_log_directory(logger: State<'_, Arc<DesktopLogger>>) -> Result<String, String> {
    Ok(logger.log_directory().to_string_lossy().to_string())
}

/// 只读读取「学习日志」: 自动校准引擎的调整记录 (tuning-log.jsonl, 数据目录,
/// 与侧车 session db 同落位)。文件缺失 = 空列表; 坏行跳过。
#[tauri::command]
async fn read_tuning_log(
    supervisor: State<'_, Arc<BackendSupervisor>>,
) -> Result<Vec<backend_supervisor::TuningLogEntry>, String> {
    supervisor.read_tuning_log().await
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

/// 快捷窗开关的纯决策（可单测）。
///
/// 关闭 = 真正销毁窗口，而不是 hide：隐藏透明窗在部分桌面环境会残留一层
/// 点不掉的透明残影（幽灵窗）。销毁后两条关闭路径（窗内 × / 托盘切换）与
/// 「托盘再点 = 新开/唤起」的状态完全一致，不存在隐藏中的透明窗口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuickWindowAction {
    /// 窗口可见：关闭（销毁）。
    Close,
    /// 窗口存在但不可见：唤起。
    Show,
    /// 窗口不存在（已销毁）：新开。
    Create,
}

fn quick_window_action(exists: bool, visible: bool) -> QuickWindowAction {
    match (exists, visible) {
        (true, true) => QuickWindowAction::Close,
        (true, false) => QuickWindowAction::Show,
        (false, _) => QuickWindowAction::Create,
    }
}

/// 快捷窗统一构建入口（启动预建 + 托盘唤起共用同一组窗口参数）。
fn build_quick_window(
    app: &tauri::AppHandle,
    visible: bool,
) -> tauri::Result<tauri::WebviewWindow> {
    WebviewWindowBuilder::new(
        app,
        "quick",
        WebviewUrl::App("index.html?window=quick".into()),
    )
    .title("Apeireth 快捷")
    .inner_size(440.0, 390.0)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(visible)
    .build()
}

/// 快捷窗外部处决命令：× 绝不自毁——webview 调用自身 destroy() 会自毁挂死
/// （表现为 Windows 鬼影窗：透明带框、点不掉、托盘再点才消）。改为 JS 发命令、
/// Rust 从外部「先隐身影、再销毁」，确定性拆除、残影无源。
#[tauri::command]
fn close_quick_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("quick") {
        let _ = window.hide();
        let _ = window.destroy();
    }
}

#[tauri::command]
fn toggle_quick_window(app: tauri::AppHandle) {
    let existing = app.get_webview_window("quick");
    let visible = existing
        .as_ref()
        .map(|window| window.is_visible().unwrap_or(false))
        .unwrap_or(false);
    match quick_window_action(existing.is_some(), visible) {
        QuickWindowAction::Close => {
            // 真正关闭（销毁）：透明无边框窗直接销毁会在部分 Windows/WebView2
            // 组合上残留透明残影——先隐身后销毁（视觉层先撤再拆），残影无源。
            if let Some(window) = existing {
                let _ = window.hide();
                let _ = window.destroy();
            }
        }
        QuickWindowAction::Show => {
            if let Some(window) = existing {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        QuickWindowAction::Create => {
            // 销毁后的快捷窗在此新开；与「唤起」同一入口，状态一致。
            if let Err(error) = build_quick_window(&app, true) {
                eprintln!("quick window create failed: {error}");
            }
            if let Some(window) = app.get_webview_window("quick") {
                let _ = window.set_focus();
            }
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
    // backend stdout/stderr lands in apeireth-backend.log. A persisted config
    // file that is malformed / foreign / version-unsupported is **rejected**
    // here (no silent fallback to defaults): refuse to start with a typed error
    // that names the offending file.
    let supervisor = match BackendSupervisor::with_logger(logger.clone()) {
        Ok(supervisor) => Arc::new(supervisor),
        Err(error) => {
            logger.log_desktop(
                LogLevel::Error,
                &format!("desktop.config rejected, refusing to start: {error}"),
            );
            eprintln!("Persisted config rejected: {error}");
            std::process::exit(1);
        }
    };

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
            close_quick_window,
            get_backend_status,
            start_backend,
            stop_backend,
            restart_backend,
            apply_backend_provider_env,
            apply_backend_config,
            get_gateway_effective_config,
            get_provider_key,
            set_provider_key,
            delete_provider_key,
            has_provider_key,
            get_workspace_dir,
            set_workspace_dir,
            list_workspace_suggestions,
            get_log_directory,
            open_log_directory,
            read_tuning_log,
            provider_proxy::provider_request,
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
                        logger_clone.log_desktop(
                            LogLevel::Info,
                            &format!("Backend auto-start success: {}", msg),
                        );
                    }
                    Err(e) => {
                        eprintln!("Backend auto-start failed: {}", e);
                        logger_clone.log_desktop(
                            LogLevel::Error,
                            &format!("Backend auto-start failed: {}", e),
                        );
                    }
                }
            });

            // 主窗口由 tauri.conf.json 声明 (app.windows[0] label=main), 这里不再重复创建.

            // 快捷窗 (启动预建、初始隐藏; 托盘「快捷窗口」新开/唤起/关闭)
            let _ = build_quick_window(app.handle(), false);

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

#[cfg(test)]
mod tests {
    use super::{quick_window_action, QuickWindowAction};

    /// 快捷窗生命周期决策表：可见 = 销毁关闭、存在不可见 = 唤起、不存在 = 新开。
    #[test]
    fn quick_window_toggle_decision_table() {
        assert_eq!(quick_window_action(true, true), QuickWindowAction::Close);
        assert_eq!(quick_window_action(true, false), QuickWindowAction::Show);
        assert_eq!(quick_window_action(false, false), QuickWindowAction::Create);
    }

    /// 开→关→开→关 与 开→托盘关→托盘开 两条路径走同一状态机：关闭即销毁，
    /// 任何时刻都不存在「隐藏中的透明窗」可残留成关不掉的幽灵窗。
    #[test]
    fn quick_window_open_close_cycles_leave_no_hidden_window() {
        let mut exists = false;
        let mut visible = false;
        let mut trace = Vec::new();
        for _ in 0..4 {
            match quick_window_action(exists, visible) {
                QuickWindowAction::Create => {
                    exists = true;
                    visible = true;
                    trace.push("create");
                }
                QuickWindowAction::Show => {
                    visible = true;
                    trace.push("show");
                }
                QuickWindowAction::Close => {
                    // 关闭 = 销毁：窗口对象不复存在，隐藏态无从残留。
                    exists = false;
                    visible = false;
                    trace.push("close");
                }
            }
        }
        assert_eq!(trace, ["create", "close", "create", "close"]);
        assert!(!exists);
    }
}
