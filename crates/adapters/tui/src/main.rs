//! 终端驾驶舱入口: 一条命令拉起全屏 TUI (`cargo run -p apeireth-tui`)。
//!
//! 终端生命周期: 进入备用屏 + 原始模式, 退出时无条件还原。回合在工作线程
//! 执行 (UI 线程零阻塞), 增量经消息通道回流主循环逐帧渲染。
//!
//! 重绘纪律 (P2): 重绘只在数据变化 (脏帧) 或动画帧到点时发生; 动画帧预算
//! ≤5fps ([`AnimClock`]), `--motion reduced` 档无动画帧 —— 空闲近乎零 CPU。

#![forbid(unsafe_code)]

use std::io::{self, Stdout};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use apeireth_tui::backend::CockpitBackend;
use apeireth_tui::effects;
use apeireth_tui::gateway_http::HttpGatewayBackend;
use apeireth_tui::render;
use apeireth_tui::state::{App, ConnectionState, Effect, ExitStage, Input};
use apeireth_tui::theme::{AnimClock, CockpitTheme, MotionMode};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

/// 缺省端点 (与既有 gateway/CLI 后端的本机回环地址一致)。
const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:8080";

/// 遥测数据轮询间隔 (毫秒): 数据拉取节奏, 不是动画节奏。
const TELEMETRY_POLL_MS: u64 = 5_000;

/// 输入/通道轮询上限 (毫秒): 唤醒检查不等于重绘 —— 没有事件就没有重绘,
/// 空闲 CPU 近乎为零; 上限只为工作线程增量的回流延迟兜底。
const INPUT_POLL_CAP_MS: u64 = 200;

/// 用法文本。
const USAGE: &str = "apeireth-tui — 终端驾驶舱\n\n用法:\n  apeireth-tui [--endpoint URL] [--token TOKEN] [--motion full|reduced]\n\n环境变量:\n  APEIRETH_GATEWAY_URL     后端端点 (缺省 http://127.0.0.1:8080)\n  APEIRETH_GATEWAY_TOKEN   回环访问令牌 (敏感端点令牌门; --token 优先; 缺省回落常驻服务令牌面, 无令牌=三态门照旧)\n  APEIRETH_TUI_MOTION      动效档 (full / reduced)\n\n键位: / 命令  ? 帮助  Ctrl+C 两段式退出  Esc Esc 时间倒带(接口桩)";

/// 启动选项。
struct Options {
    /// 后端端点。
    endpoint: String,
    /// 回环访问令牌 (可选; 有则随请求呈现 Bearer 头)。
    token: Option<String>,
    /// 动效档。
    motion: MotionMode,
}

fn parse_args() -> Result<Options, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    parse_args_from(&args, &|key| std::env::var(key).ok())
}

/// 参数解析主体 (env 取值注入, 便于钉行为): `--token` 优先于 env
/// `APEIRETH_GATEWAY_TOKEN`, 空白令牌按未配置处理。
fn parse_args_from(
    args: &[String],
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Options, String> {
    let mut endpoint = env("APEIRETH_GATEWAY_URL")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string());
    let mut token = env("APEIRETH_GATEWAY_TOKEN")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let mut motion = env("APEIRETH_TUI_MOTION")
        .and_then(|value| MotionMode::parse(&value))
        .unwrap_or_default();

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--endpoint" => {
                index += 1;
                let value = args.get(index).ok_or("--endpoint 需要一个 URL")?;
                endpoint = value.clone();
            }
            "--token" => {
                index += 1;
                let value = args.get(index).ok_or("--token 需要一个令牌值")?;
                token = Some(value.trim().to_string()).filter(|value| !value.is_empty());
            }
            "--motion" => {
                index += 1;
                let value = args.get(index).ok_or("--motion 需要 full 或 reduced")?;
                motion = MotionMode::parse(value)
                    .ok_or_else(|| format!("未知动效档: {value} (可选 full / reduced)"))?;
            }
            other => return Err(format!("未知参数: {other} (用 --help 查看用法)")),
        }
        index += 1;
    }
    Ok(Options {
        endpoint,
        token,
        motion,
    })
}

/// 令牌解析收口 (G10 常驻协同): 显式值 (`--token` / env) 优先, 缺省回落
/// **服务令牌面** (同源令牌, 见 `apeireth_credentials::service_token`)。
/// 令牌面只在常驻服务安装期间存在 —— 即「探测到常驻 = 读服务令牌面」的
/// 落地口径; 若所连端点并非该常驻网关 (自定义 `--endpoint` 连侧车的边角),
/// 敏感端点会显式 401 而非静默放行。两者皆无 = None —— 三态门警告帧语义
/// 照旧 (不发空 Bearer, 不自造第四种口径)。
fn resolve_access_token(explicit: Option<String>, service_face: Option<String>) -> Option<String> {
    explicit
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or(service_face)
        .filter(|value| !value.trim().is_empty())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{USAGE}");
        return;
    }
    let options = match parse_args() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    // 令牌三源收口: 显式值优先, 缺省读常驻服务令牌面 (同源), 无令牌照旧。
    let access_token = resolve_access_token(
        options.token.clone(),
        apeireth_credentials::service_token::read().map(|secret| secret.expose().to_string()),
    );
    let mut app = App::new(&options.endpoint, options.motion);
    let backend: BackendHandle =
        match HttpGatewayBackend::with_token(&options.endpoint, access_token.as_deref()) {
            Ok(backend) => Arc::new(Mutex::new(Box::new(backend))),
            Err(error) => {
                // 端点构造失败 = 配置错误, 明确报错退出 (不白屏装跑)。
                eprintln!("端点不可用: {error}");
                std::process::exit(2);
            }
        };

    // 启动即探活: 连接失败 → 错误帧第一帧上屏。
    let mut startup_effects = Vec::new();
    {
        let mut guard = lock_backend(&backend);
        match guard.health() {
            Ok(()) => {
                startup_effects = app.feed(Input::ConnectionOk);
            }
            Err(error) => {
                app.feed(Input::ConnectionFailed {
                    detail: error.to_string(),
                });
            }
        }
    }

    if let Err(error) = run(backend, &mut app, startup_effects) {
        eprintln!("终端驾驶舱异常退出: {error}");
        std::process::exit(1);
    }
}

type BackendHandle = Arc<Mutex<Box<dyn CockpitBackend>>>;

fn lock_backend(backend: &BackendHandle) -> std::sync::MutexGuard<'_, Box<dyn CockpitBackend>> {
    backend
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn run(backend: BackendHandle, app: &mut App, startup_effects: Vec<Effect>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let (tx, rx) = mpsc::channel::<Input>();
    let result = event_loop(&mut terminal, app, &backend, &tx, &rx, startup_effects);
    // 无条件还原终端。
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    backend: &BackendHandle,
    tx: &Sender<Input>,
    rx: &Receiver<Input>,
    startup_effects: Vec<Effect>,
) -> io::Result<()> {
    let started = Instant::now();
    let now_ms = || started.elapsed().as_millis() as u64;
    // 动画帧预算调度: full 档 ≥200ms/帧 (≤5fps), reduced 档无动画帧。
    let mut anim = AnimClock::new(&CockpitTheme::new(app.motion).motion);
    let mut dirty = true;
    let mut next_telemetry_ms = 0u64;
    dispatch(startup_effects, backend, tx);
    loop {
        let now = now_ms();

        // 动画帧到点才推进动效 (呼吸/波形/跳动数字); 数据轮询是另一条节奏。
        if anim.due(now) {
            let effects = app.feed(Input::Tick);
            dispatch(effects, backend, tx);
            dirty = true;
        }
        // 重绘只在数据变化 (脏帧) 或动画帧需要时发生。
        if dirty {
            terminal.draw(|frame| render::draw(frame, app))?;
            dirty = false;
        }
        // 遥测数据轮询 (不是动画): 定期把真实读数回灌面板组。
        if now >= next_telemetry_ms {
            next_telemetry_ms = now + TELEMETRY_POLL_MS;
            if matches!(app.status.connection, ConnectionState::Connected) {
                dispatch(vec![Effect::RefreshTelemetry], backend, tx);
            }
        }

        // 空闲等待: 下一个动画帧 / 数据轮询到点 / 轮询上限兜底 (唤醒 ≠ 重绘)。
        let mut wait_ms = next_telemetry_ms.saturating_sub(now).min(INPUT_POLL_CAP_MS);
        if let Some(next_anim) = anim.next_due_in(now) {
            wait_ms = wait_ms.min(next_anim);
        }
        if event::poll(Duration::from_millis(wait_ms))? {
            if let Event::Key(key) = event::read()? {
                if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    let effects = app.handle_key(key);
                    dispatch(effects, backend, tx);
                    dirty = true;
                }
            }
        }
        while let Ok(input) = rx.try_recv() {
            let effects = app.feed(input);
            dispatch(effects, backend, tx);
            dirty = true;
        }

        if app.exit == ExitStage::Exiting {
            break;
        }
    }
    Ok(())
}

/// 把效果派发到工作线程 (UI 线程零阻塞)。
fn dispatch(effects: Vec<Effect>, backend: &BackendHandle, tx: &Sender<Input>) {
    for effect in effects {
        let backend = Arc::clone(backend);
        let tx = tx.clone();
        std::thread::spawn(move || {
            let mut sink = |input: Input| {
                let _ = tx.send(input);
            };
            let mut guard = lock_backend(&backend);
            effects::run(effect, guard.as_mut(), &mut sink);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    /// 令牌注入面 (G10 对齐): `--token` 显式给值 / env `APEIRETH_GATEWAY_TOKEN`
    /// 同源兜底 / 空白按未配置 (不发空 Bearer)。
    #[test]
    fn token_comes_from_flag_or_env_and_blank_means_none() {
        // env 兜底。
        let env = |key: &str| (key == "APEIRETH_GATEWAY_TOKEN").then(|| "env-token".to_string());
        let options = parse_args_from(&[], &env).expect("parse");
        assert_eq!(options.token.as_deref(), Some("env-token"));

        // --token 显式覆盖 env。
        let options = parse_args_from(&args(&["--token", "flag-token"]), &env).expect("parse");
        assert_eq!(options.token.as_deref(), Some("flag-token"), "旗标优先");

        // 空白 = 无令牌 (兼容路径: 不发鉴权头)。
        let options = parse_args_from(&args(&["--token", "  "]), &no_env).expect("parse");
        assert_eq!(options.token, None);
        let options = parse_args_from(&[], &no_env).expect("parse");
        assert_eq!(options.token, None);
    }

    /// 服务令牌面回落 (G10 常驻协同): 显式值赢过服务面; 无显式值才用服务面
    /// (探测到常驻 = 读同源令牌); 两者皆无 = None = 三态门警告帧语义照旧。
    #[test]
    fn access_token_prefers_explicit_then_service_face_then_none() {
        // 显式值优先 (含空白显式值按未配置)。
        assert_eq!(
            resolve_access_token(Some("flag-token".into()), Some("face-token".into())).as_deref(),
            Some("flag-token")
        );
        assert_eq!(
            resolve_access_token(Some("   ".into()), Some("face-token".into())).as_deref(),
            Some("face-token"),
            "空白显式值读作未配置, 回落服务令牌面"
        );

        // 无显式值 → 服务令牌面 (同源令牌)。
        assert_eq!(
            resolve_access_token(None, Some("face-token".into())).as_deref(),
            Some("face-token")
        );

        // 两者皆无 → None (不发 Bearer, 三态门照旧)。
        assert_eq!(resolve_access_token(None, None), None);
        assert_eq!(resolve_access_token(None, Some("   ".into())), None);
    }

    /// 端点/动效既有语义零回归 (令牌面只增不改)。
    #[test]
    fn endpoint_and_motion_keep_their_existing_semantics() {
        let env = |key: &str| match key {
            "APEIRETH_GATEWAY_URL" => Some("http://127.0.0.1:9000".to_string()),
            "APEIRETH_TUI_MOTION" => Some("reduced".to_string()),
            _ => None,
        };
        let options = parse_args_from(&[], &env).expect("parse");
        assert_eq!(options.endpoint, "http://127.0.0.1:9000");
        assert!(matches!(options.motion, MotionMode::Reduced));

        let options = parse_args_from(
            &args(&["--endpoint", "http://127.0.0.1:7000", "--motion", "full"]),
            &env,
        )
        .expect("parse");
        assert_eq!(options.endpoint, "http://127.0.0.1:7000");
        assert!(matches!(options.motion, MotionMode::Full));
    }
}
