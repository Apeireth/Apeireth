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
mod imap_client;
mod keychain;
mod logging;
mod mysql_client;
mod pg_client;
mod portable_mode;
mod provider_proxy;
mod smtp_send;
mod speech_recognizer;
mod sqlite_workbench;
mod stored_config;
mod terminal_exec;
pub mod workspace;
mod workspace_list;

use backend_supervisor::{
    BackendCapabilityEnv, BackendInfo, BackendProviderEnv, BackendSupervisor, INITIAL_CONFIG_WINDOW,
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

/// The loopback access token the UI must present on the gateway's gated
/// endpoints (whole API face since the token gate went gateway-wide; only
/// `/health` is exempt).
///
/// In-memory only: it is minted per backend spawn (or inherited from an
/// explicit env value), crosses IPC like provider keys do, and is never
/// persisted or logged. Empty string = nothing to present (gateway declares
/// itself unprotected), which the caller treats as "send no Authorization".
#[tauri::command]
async fn get_gateway_token(
    supervisor: State<'_, Arc<BackendSupervisor>>,
) -> Result<String, String> {
    Ok(supervisor.gateway_access_token().await.unwrap_or_default())
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

// ============================================================
// 桌宠常驻窗（presence 第四个投影，00-PHILOSOPHY §5）
// ============================================================

/// 桌宠开关的纯决策（可单测）。与快捷窗同一条销毁纪律：用户关闭 = 先隐
/// 身影再销毁，绝不留「用户以为关掉、实际藏着」的透明窗（幽灵窗教训见
/// `quick_window_action`）。互斥礼让的 hide/show 不走本表——那是常驻面的
/// 视觉礼让，窗体本体一直在场（重建零成本、状态不丢）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PetWindowAction {
    /// 窗口可见：关闭（销毁）。
    Close,
    /// 窗口存在但不可见：唤起。
    Show,
    /// 窗口不存在（已销毁）：新开。
    Create,
}

fn pet_window_action(exists: bool, visible: bool) -> PetWindowAction {
    match (exists, visible) {
        (true, true) => PetWindowAction::Close,
        (true, false) => PetWindowAction::Show,
        (false, _) => PetWindowAction::Create,
    }
}

/// 互斥礼让的纯决策（可单测）：主窗（场景窗）在场呈现时桌宠隐身礼让；
/// 主窗收起（关闭进托盘）时桌宠接管常驻。两份显影不抢资源
/// （00-PHILOSOPHY §5「互斥礼让」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PetLeeway {
    Show,
    Hide,
}

fn pet_leeway(main_window_presenting: bool) -> PetLeeway {
    if main_window_presenting {
        PetLeeway::Hide
    } else {
        PetLeeway::Show
    }
}

/// 桌宠鼠标穿透态。穿透后窗内收不到任何指针事件，所以托盘菜单必须能翻回
/// （窗内右键菜单此时已不可达）。
static PET_CLICK_THROUGH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 桌宠窗统一构建入口（启动预建 + 托盘/互斥唤起共用同一组参数）。
/// 透明、置顶、无边框、不进任务栏、不可缩放；默认落位主显示器右下角
/// （桌宠的常驻位）。加载 `index.html?window=pet`（main.ts 角色路由）。
fn build_pet_window(app: &tauri::AppHandle, visible: bool) -> tauri::Result<tauri::WebviewWindow> {
    let (width, height) = (220.0_f64, 260.0_f64);
    let mut builder =
        WebviewWindowBuilder::new(app, "pet", WebviewUrl::App("index.html?window=pet".into()))
            .title("Apeireth 桌宠")
            .inner_size(width, height)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .visible(visible);
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let work_w = f64::from(size.width) / scale;
        let work_h = f64::from(size.height) / scale;
        builder = builder.position(work_w - width - 24.0, work_h - height - 24.0);
    }
    let window = builder.build()?;
    // 跟随记忆中的穿透态（托盘开关可能先于窗创建）
    let _ = window
        .set_ignore_cursor_events(PET_CLICK_THROUGH.load(std::sync::atomic::Ordering::Relaxed));
    Ok(window)
}

/// 桌宠接管常驻（互斥礼让的 Show 臂）：不存在则建，存在则唤起。
/// 不抢焦点——他是常驻在场，不是弹窗。
fn show_pet_window(app: &tauri::AppHandle) {
    match app.get_webview_window("pet") {
        Some(window) => {
            let _ = window.show();
        }
        None => {
            if let Err(error) = build_pet_window(app, true) {
                eprintln!("pet window create failed: {error}");
            }
        }
    }
}

/// 互斥礼让应用：主窗在场 → 桌宠隐身；主窗收起 → 桌宠接管。
///
/// 隐身前发 `pet-host-visibility` 显式事件（真隐藏口径）：实测 `window.hide()`
/// 不翻 `document.visibilityState`，形象层停帧闩（pet-render-pause）靠本事件
/// 置位——原生节流 + 显式闩双保险，后者是可验收的那道（2026-10-11 真窗实测）。
fn apply_pet_leeway(app: &tauri::AppHandle, main_window_presenting: bool) {
    use tauri::Emitter;
    match pet_leeway(main_window_presenting) {
        PetLeeway::Show => {
            if let Some(pet) = app.get_webview_window("pet") {
                let _ = pet.emit("pet-host-visibility", true);
            }
            show_pet_window(app);
        }
        PetLeeway::Hide => {
            if let Some(pet) = app.get_webview_window("pet") {
                let _ = pet.emit("pet-host-visibility", false);
                let _ = pet.hide();
            }
        }
    }
}

/// 桌宠外部处决命令：「隐藏桌宠」= 真正关闭（先隐身影再销毁），与快捷窗
/// 同一拆除纪律；托盘「桌宠」再点 = 新开。
#[tauri::command]
fn close_pet_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("pet") {
        let _ = window.hide();
        let _ = window.destroy();
    }
}

#[tauri::command]
fn toggle_pet_window(app: tauri::AppHandle) {
    let existing = app.get_webview_window("pet");
    let visible = existing
        .as_ref()
        .map(|window| window.is_visible().unwrap_or(false))
        .unwrap_or(false);
    match pet_window_action(existing.is_some(), visible) {
        PetWindowAction::Close => {
            if let Some(window) = existing {
                let _ = window.hide();
                let _ = window.destroy();
            }
        }
        PetWindowAction::Show => {
            if let Some(window) = existing {
                // 解闩显式事件（与 apply_pet_leeway 同源）：礼让隐藏过的窗别冻着
                {
                    use tauri::Emitter;
                    let _ = window.emit("pet-host-visibility", true);
                }
                let _ = window.show();
            }
        }
        PetWindowAction::Create => {
            if let Err(error) = build_pet_window(&app, true) {
                eprintln!("pet window create failed: {error}");
            }
        }
    }
}

/// 右键卷宗入口（00-PHILOSOPHY §5）：唤起主窗并把导航目标递给壳层
/// （`shell-target` 事件，App.svelte 按 shell-nav 目标表改道）。
#[tauri::command]
fn open_main_target(app: tauri::AppHandle, target: String) {
    use tauri::Emitter;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("shell-target", target);
    }
}

/// 桌宠鼠标穿透开关。穿透态由托盘「桌宠穿透」兜底翻回（穿透后窗内点不到
/// 任何东西，右键菜单不可达）。
#[tauri::command]
fn set_pet_click_through(app: tauri::AppHandle, enabled: bool) {
    PET_CLICK_THROUGH.store(enabled, std::sync::atomic::Ordering::Relaxed);
    if let Some(window) = app.get_webview_window("pet") {
        let _ = window.set_ignore_cursor_events(enabled);
    }
}

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "打开主窗", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quick", "快捷窗口", true, None::<&str>)?;
    let pet = MenuItem::with_id(app, "pet", "桌宠", true, None::<&str>)?;
    let pet_through = MenuItem::with_id(app, "pet_through", "桌宠鼠标穿透", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    Menu::with_items(app, &[&show, &quick, &pet, &pet_through, &settings, &quit])
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
            // 互斥礼让: 主窗被唤到前台, 桌宠隐身
            apply_pet_leeway(app, true);
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
            backend_supervisor::get_resident_probe,
            start_backend,
            stop_backend,
            restart_backend,
            apply_backend_provider_env,
            apply_backend_config,
            get_gateway_effective_config,
            get_gateway_token,
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
            terminal_exec::run_workspace_command,
            portable_mode::get_portable_mode,
            sqlite_workbench::query_sqlite_file,
            workspace_list::list_workspace_dir,
            smtp_send::send_smtp,
            pg_client::query_postgres,
            imap_client::imap_fetch,
            mysql_client::query_mysql,
            speech_recognizer::detect_speech_engine,
            speech_recognizer::recognize_speech,
            workspace_list::create_dir_entry,
            workspace_list::rename_entry,
            workspace_list::delete_entry,
            open_settings,
            toggle_quick_window,
            close_pet_window,
            toggle_pet_window,
            open_main_target,
            set_pet_click_through
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            // Auto-start backend on app launch. The launch spawn is held until
            // the Settings-UI's initial config push resolves (bounded by
            // INITIAL_CONFIG_WINDOW), so model/capability-family changes land
            // in the FIRST spawn instead of bouncing a just-ready sidecar
            // (E-07: one spawn straight to ready).
            let supervisor_clone = supervisor.clone();
            let logger_clone = logger.clone();
            tauri::async_runtime::spawn(async move {
                logger_clone.log_desktop(LogLevel::Info, "Backend auto-start initiated");
                match supervisor_clone
                    .start_after_initial_config(INITIAL_CONFIG_WINDOW)
                    .await
                {
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

            // 桌宠常驻窗 (启动预建、初始隐藏; 主窗在场即互斥礼让, 主窗收起接管)
            let _ = build_pet_window(app.handle(), false);

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
                        // 互斥礼让: 主窗在场, 桌宠隐身
                        apply_pet_leeway(app, true);
                    }
                    "quick" => toggle_quick_window(app.clone()),
                    "pet" => toggle_pet_window(app.clone()),
                    "pet_through" => {
                        let next = !PET_CLICK_THROUGH.load(std::sync::atomic::Ordering::Relaxed);
                        set_pet_click_through(app.clone(), next);
                    }
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
                        // 互斥礼让: 主窗在场, 桌宠隐身
                        apply_pet_leeway(app, true);
                    }
                })
                .build(&handle)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // 关闭主窗时隐藏到托盘, 不退出 (桌面伴随体常驻)
            if window.label() == "main" {
                match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        let _ = window.hide();
                        api.prevent_close();
                        // 互斥礼让: 主窗收起, 桌宠接管常驻 (00-PHILOSOPHY §5)
                        apply_pet_leeway(window.app_handle(), false);
                    }
                    tauri::WindowEvent::Focused(true) => {
                        // 互斥礼让: 主窗回到前台, 桌宠隐身 (两份显影不抢资源)
                        apply_pet_leeway(window.app_handle(), true);
                    }
                    _ => {}
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
    use super::{
        pet_leeway, pet_window_action, quick_window_action, PetLeeway, PetWindowAction,
        QuickWindowAction,
    };

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

    /// 桌宠开关决策表：与快捷窗同一条销毁纪律（可见 = 销毁关闭、存在不可见 =
    /// 唤起、不存在 = 新开），用户关闭路径绝不存在「隐藏中的透明窗」。
    #[test]
    fn pet_window_toggle_decision_table() {
        assert_eq!(pet_window_action(true, true), PetWindowAction::Close);
        assert_eq!(pet_window_action(true, false), PetWindowAction::Show);
        assert_eq!(pet_window_action(false, false), PetWindowAction::Create);
    }

    /// 互斥礼让决策表：主窗在场 → 桌宠隐身；主窗收起 → 桌宠接管常驻。
    #[test]
    fn pet_leeway_follows_main_window_presence() {
        assert_eq!(pet_leeway(true), PetLeeway::Hide);
        assert_eq!(pet_leeway(false), PetLeeway::Show);
    }
}
