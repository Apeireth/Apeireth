//! Apeireth R19 TUI — ratatui 终端版, 后端全接
//!
//! **R11 LOCKED 边界** (omnibus §6): 本文件只调后端 crate 公开 API,
//! 不修改任何 R11 LOCKED enum / 转换矩阵 / 8 项不修改承诺.
//!
//! **5 nav 顺序** (主人 R19 决定):
//! - 0 舰桥 (Bridge, ΣΚΟΠΗ)  — 默认首页
//! - 1 对话 (Dialogue, ΔΙΑΛΟΓΟΣ)
//! - 2 生长 (Growth, ΑΥΞΗΣΙΣ)
//! - 3 历史 (History, ΙΣΤΟΡΙΑ)
//! - 4 设置 (Settings, ΤΑΞΙΣ)
//!
//! **键位**:
//! - `q` 退出
//! - `0/1/2/3/4` 直接跳
//! - `Tab` / `BackTab` 顺序切
//! - `i` 或 `Enter` (舰桥页) → 跳对话
//! - `PageUp/PageDown` 滚对话/历史, `Home/End` 跳顶/底 (R26 新增)

// R30 U6: notify multi-config watcher
mod app;
mod backend;
mod cognition_live;
mod config_watcher;
mod http_llm;
mod observability;
mod organ;
mod pages;

// sister #1 — 9 器官 × 6 command dispatcher (借鉴 Golutra #1)
// 0 触碰 organ 子树, 独立登记在 crate 根, 跟 organ/ 同级 (R23 P3 迁移)
mod command;

// R22 ST-A1.2: eye 真接 keystrokes (handle_key 处 hook)
use crate::organ::ear;
use crate::organ::eye;
use crate::organ::heart;
mod llm_config;
mod onboarding;
mod persistence;
mod theme;

// R155 TUI × runtime bridge
mod runtime_bridge;

use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use cognition_live::{CognitionLiveTracker, LiveEvent};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Alignment;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;

use crate::app::{App, ChatMessage, NavPage};
use crate::theme::{Theme, ThemeStyle};

type Tui = Terminal<CrosstermBackend<Stdout>>;

fn main() -> Result<()> {
    // R139-1-retry 2026-08-11: --help / -h 选项, 打印帮助后退出 (跟 R144-1 baseline 不一致 — 修 8 步 verify 第 4 步)
    // 0 改 24 LOCKED 入口签名 (TUI 是 binary, 不在 24 LOCKED lib.rs list, per 决策 #74 B1)
    // 兼容: `apeireth-tui --help` / `apeireth-tui -h` / `apeireth-tui 0 --help` (任意位置 --help / -h 优先)
    let args: Vec<String> = std::env::args().collect();
    if args.iter().skip(1).any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }
    // 调试模式: --snapshot <0-4> 渲染指定 nav 一次, dump ANSI 到 stdout 后退出
    if args.len() >= 3 && args[1] == "--snapshot" {
        if let Ok(n) = args[2].parse::<u8>() {
            return snapshot_mode(n);
        }
    }

    // R17 战役 4-5 (2026-08-04): 1.0 release splash 标题 — 编译期 hardcode 编译进 binary
    // (改 Cargo.toml workspace version 后自动穿透, 不再 hardcode "0.14.0" 字符串)
    eprintln!(
        "apeireth-tui v{} (R26-2 全中文化 — 2026-08-07) — 按 q 退出",
        env!("CARGO_PKG_VERSION")
    );

    // R26-3: 首启 / 无 llm.json → 走 onboarding wizard (纯文本 stdin)
    // setup_terminal 之前 → stdin 是 cooked mode (不会跟 ratatui raw mode 冲突)
    if llm_config::load().is_none() {
        if let Err(e) = onboarding::run() {
            eprintln!("[onboarding] failed: {e}; TUI 退出, 下次启动会重跑 onboarding");
            std::process::exit(1);
        }
    }

    let mut terminal = setup_terminal().context("setup terminal")?;
    // W3.5 设置页持久化: 启动时从磁盘 load, 找不到 / 解析失败 → 用默认
    let loaded = persistence::load();
    let mut app = App::with_loaded_settings(loaded);
    // R27 C 方案: 同步当前 API base_url + 连通状态 (status_bar 显示)
    if let Some(cfg) = llm_config::load() {
        app.api_url = cfg.base_url.clone();
        app.api_online = probe_api_online(&cfg.base_url);
    }
    let res = run_app(&mut terminal, &mut app);
    // 退出前再 save 一次 (兜底: 即时 save 已经覆盖 5 字段, 这里是 double-safety)
    if let Err(e) = persistence::save(&app.to_settings()) {
        eprintln!("[settings] save on quit failed: {e}");
    }
    restore_terminal(&mut terminal).ok();
    res
}

/// R139-1-retry 2026-08-11: --help 打印帮助信息
///
/// 5 nav 顺序: 0 舰桥 (Bridge) / 1 对话 (Dialogue) / 2 生长 (Growth) / 3 历史 (History) / 4 设置 (Settings)
/// 键位: q 退出, 0/1/2/3/4 直接跳, Tab/BackTab 顺序切, i/Enter (舰桥页) → 跳对话
fn print_help() {
    println!(
        "apeireth-tui v{} (R139-1-retry 2026-08-11 — 8 步 verify 8/8 全 PASS)",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("Apeireth Rust TUI — ratatui 终端版, 后端全接 (R19 阶段 4)");
    println!();
    println!("USAGE:");
    println!("    apeireth-tui [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help           打印本帮助信息并退出");
    println!("    --snapshot <0-4>     调试模式: 渲染指定 nav 一次, dump ANSI 到 stdout 后退出");
    println!("                          0=舰桥(Bridge) 1=对话(Dialogue) 2=生长(Growth) 3=历史(History) 4=设置(Settings)");
    println!();
    println!("5 NAV 顺序 (主人 R19 决定):");
    println!("    0  舰桥 (Bridge, ΣΚΟΠΗ)  — 默认首页");
    println!("    1  对话 (Dialogue, ΔΙΑΛΟΓΟΣ)");
    println!("    2  生长 (Growth, ΑΥΞΗΣΙΣ)");
    println!("    3  历史 (History, ΙΣΤΟΡΙΑ)");
    println!("    4  设置 (Settings, ΤΑΞΙΣ)");
    println!();
    println!("键位:");
    println!("    q              退出");
    println!("    0/1/2/3/4      直接跳 nav");
    println!("    Tab/BackTab    顺序切");
    println!("    i 或 Enter     舰桥页跳对话");
    println!("    PageUp/Down    滚对话/历史");
    println!("    Home/End       跳顶/底");
    println!();
    println!("ENVIRONMENT:");
    println!("    APEIRETH_API_KEY    后端 API key (默认走 onboarding wizard 输入)");
    println!();
    println!("后端: apeireth-api v1.2.0 (8 endpoint + 8 tools + 3 启动模式, per P15-1 baseline)");
    println!();
    println!("更多: docs/conventions/  (8 哲学锚 / V0.5 30 维 / 6 重守门 v7 / 13 键 + PHL-07)");
}

/// R26-2: splash overlay state machine (启动时按 settings.splash_enabled 决定显示)
///
/// - `app.splash_active == true` → 只画 splash, 任何按键退出 (除 `q` 仍走 should_quit 退出 TUI 整体)
/// - 按 `Esc` 也退出 splash 进入 nav
/// - 退出 splash 后 `app.splash_active = false`, 不再触发 (每 session 1 次)
fn render_splash(f: &mut ratatui::Frame, app: &App, style: &ThemeStyle) {
    let area = f.area();
    // 上下各空 35%, 中间 7 行高 splash card
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Length(7),
            Constraint::Min(0),
        ])
        .split(area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(style.border_type)
        .border_style(Style::default().fg(style.accent));
    // 居中 7 行内容 (centered = 行高居中+水平居中)
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "APEIRETH RUST TUI",
            Style::default()
                .fg(style.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("v{}  ·  R26-2 全中文化", env!("CARGO_PKG_VERSION")),
            Style::default().fg(style.primary),
        )),
        Line::from(Span::styled(
            "2026-08-07 内部测试",
            Style::default().fg(style.dim),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "按任意键进入  ·  q 退出",
            Style::default().fg(style.dim),
        )),
    ];
    let para = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    f.render_widget(para, chunks[1]);
    // 记录 splash_active (供 run_app 主循环读, 但 fn 本身不消费)
    let _ = app.splash_active;
}

/// 调试模式: 渲染指定 nav 一次, dump 整个 ANSI frame 到 stdout, 退出
fn snapshot_mode(nav_idx: u8) -> Result<()> {
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    use ratatui::Terminal;

    // W3.5 修: snapshot 也走 loaded settings, 否则调试失真 (settings 页一直显示默认)
    let loaded = persistence::load();
    let mut app = App::with_loaded_settings(loaded);
    // R27 C 方案: snapshot 也填 api_url / api_online, status_bar 才准
    if let Some(cfg) = llm_config::load() {
        app.api_url = cfg.base_url.clone();
        app.api_online = probe_api_online(&cfg.base_url);
    }
    if let Some(p) = NavPage::from_u8(nav_idx) {
        app.nav = p;
    }
    let backend = TestBackend::new(140, 45);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|f| ui(f, &mut app))?;

    // 把 buffer 里的 cells 序列化成 ANSI 写到 stdout
    let buf = terminal.backend().buffer().clone();
    let mut out = String::new();
    let mut last_fg: Option<(u8, u8, u8)> = None;
    let mut last_bg: Option<(u8, u8, u8)> = None;
    let mut last_bold = false;
    let mut cursor = (0u16, 0u16);
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            let (cur_x, cur_y) = (x, y);
            if (cur_x, cur_y) != cursor {
                out.push_str(&format!("\x1b[{};{}H", cur_y + 1, cur_x + 1));
                cursor = (cur_x, cur_y);
            }
            // color & style
            let new_fg = match cell.fg {
                Color::Rgb(r, g, b) => Some((r, g, b)),
                _ => None,
            };
            let new_bg = match cell.bg {
                Color::Rgb(r, g, b) => Some((r, g, b)),
                _ => None,
            };
            let new_bold = cell.modifier.contains(ratatui::style::Modifier::BOLD);
            if (new_fg, new_bg, new_bold) != (last_fg, last_bg, last_bold) {
                out.push_str("\x1b[0");
                if new_bold {
                    out.push_str(";1");
                }
                if let Some((r, g, b)) = new_fg {
                    out.push_str(&format!(";38;2;{};{};{}", r, g, b));
                }
                if let Some((r, g, b)) = new_bg {
                    out.push_str(&format!(";48;2;{};{};{}", r, g, b));
                }
                out.push('m');
                last_fg = new_fg;
                last_bg = new_bg;
                last_bold = new_bold;
            }
            out.push_str(cell.symbol());
            cursor.0 += 1;
        }
    }
    print!("{}", out);
    Ok(())
}

fn setup_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Tui) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(terminal: &mut Tui, app: &mut App) -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();
    let mut cognition_live = CognitionLiveTracker::new();
    loop {
        // R26-2 tick: 推进 breath 相位 (250ms + 0.10 ≈ 2.5s 完整呼吸周期)
        // breath_enabled=false 时不累加 (停在当前位置), 直到 settings [b] 重新开
        if app.breath_enabled {
            app.breath_phase = (app.breath_phase + 0.10) % (2.0 * std::f32::consts::TAU);
        }
        // R26-2 splash: 启动时如果 splash_enabled, 先画 splash overlay (按任意键退出)
        // q 仍然退出 TUI 整体, 其余任何键都进 nav
        if app.splash_active {
            let style = app.current_style();
            terminal.draw(|f| render_splash(f, app, &style))?;
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') => {
                                app.splash_active = false;
                                app.should_quit = true;
                                return Ok(());
                            }
                            _ => {
                                app.splash_active = false;
                            }
                        }
                    }
                }
            }
            continue; // splash 期间跳过 nav 渲染
        }
        // (正常 nav 渲染主循环)
        // W3 #1 流式 chat 收尾: tick 时收 LLM chunks
        // 多个 Ok 累加到 streaming_message, Disconnected 时 commit 到 chat_history
        if let Some(rx) = &app.chat_rx {
            // 循环收 chunk (同 1 帧可能多个 chunk 到, 一次性收完)
            loop {
                match rx.try_recv() {
                    Ok(chunk) => {
                        // R30 P4: 检测 ToolCallEvent 前缀, 分流到 tool_events
                        if chunk.starts_with(crate::backend::TOOL_EVT_PREFIX) {
                            let body = chunk
                                .trim_start_matches(crate::backend::TOOL_EVT_PREFIX)
                                .trim();
                            if let Ok(evt) =
                                serde_json::from_str::<crate::backend::ToolCallEvent>(body)
                            {
                                app.tool_events.push(evt);
                            }
                            // tool_event 不进 streaming_message (避免污染 LLM 文本)
                        } else if let Some(ref mut s) = app.streaming_message {
                            // 普通 LLM 文本 chunk, 累加
                            s.push_str(&chunk);
                        }
                        // 注: 不在这里 break, 继续 try_recv 直到 Empty (本帧收完)
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // 还在等, 继续 spinner
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        // thread 已 drop sender, 流结束
                        // commit streaming_message → chat_history
                        if let Some(streamed) = app.streaming_message.take() {
                            if !streamed.is_empty() {
                                app.push_assistant_reply(streamed);
                            }
                        }
                        // R30 P4: commit tool_events → chat_history (作为 system 消息, 灰色行)
                        let events: Vec<crate::backend::ToolCallEvent> =
                            app.tool_events.drain(..).collect();
                        for evt in events {
                            let line = crate::backend::format_tool_event(&evt);
                            app.push_system(line);
                        }
                        app.processing = false;
                        app.chat_rx = None;
                        // R26-3-fixes: AI 完成后自动恢复 focus (用户可立刻输入下一句)
                        app.input_focused = true;
                        // R26-3-fixes: 自动提交 pending_input (用户提前输入等 AI 完成)
                        if let Some(pending) = app.pending_input.take() {
                            app.push_user_input(pending.clone());
                            let (tx, rx) = std::sync::mpsc::channel();
                            app.chat_rx = Some(rx);
                            app.processing = true;
                            app.streaming_message = Some(String::new());
                            let history: Vec<ChatMessage> = app.chat_history.clone();
                            std::thread::spawn(move || {
                                backend::chat_streaming(&pending, &history, &tx);
                            });
                        }
                        break;
                    }
                }
            }
        }
        if cognition_live.is_stale()
            && !matches!(cognition_live.check_for_update(), LiveEvent::NoChange)
        {
            app.render_tick = app.render_tick.wrapping_add(1);
        }

        // spinner 帧递增
        app.spinner_frame = app.spinner_frame.wrapping_add(1);

        // W3.6 主题切换: 200ms 渐变期已结束 → 清掉 start, 后续 ui() 走静态 style
        // (不清的话 current_style() 每帧还会做一遍 float 计算, 性能没影响但语义不干净)
        app.finish_theme_transition_if_done();

        app.render_tick = app.render_tick.wrapping_add(1);
        terminal.draw(|f| ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        if handle_key(app, key) {
                            eye::record_keystroke(); // R22 ST-A1.2 hook: keystrokes counter
                            return Ok(());
                        }
                    }
                }
                // R26-3-fixes: 鼠标事件 -> focus + 选区 + scroll
                // - 滚轮: scroll (像 PageUp/PageDown, 5 行步进)
                // - 单击 / 拖拽 / 释放: focus 切换 + 选区追踪 (handle_dialogue_mouse)
                Event::Mouse(mouse) => {
                    if app.nav == NavPage::Dialogue {
                        match mouse.kind {
                            MouseEventKind::ScrollUp => {
                                app.scroll_to_bottom = false;
                                app.scroll_offset = app.scroll_offset.saturating_add(5);
                            }
                            MouseEventKind::ScrollDown => {
                                if app.scroll_to_bottom {
                                    // 已经在底部, no-op
                                } else if app.scroll_offset < 5 {
                                    // 滚到底, 锁住
                                    app.scroll_to_bottom = true;
                                    app.scroll_offset = 0;
                                } else {
                                    app.scroll_offset -= 5;
                                }
                            }
                            _ => {
                                // R28 mouse 偏移修复: mouse handler 要同 render 一样用 page area,
                                // 不要用全屏幕 area (含 nav bar 上面 1 行).
                                let full_area = terminal.get_frame().area();
                                let (_, page_area, _) = ui_chunks(full_area);
                                handle_dialogue_mouse(app, mouse, page_area);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
        ear::record_system(); // R22 ST-A1.3 hook: system channel (tick)
        heart::record_heartbeat(); // R22 ST-A1.6 hook: beat tick counter
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    // R26-3-fixes: Ctrl+C 优先复制选区 (像 codex 自己)
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        if app.nav == NavPage::Dialogue && app.selection.is_some() {
            copy_selection_to_clipboard(app);
            return false;
        }
        // 无选区: 走退出逻辑 (PowerShell / 终端原生 Ctrl+C 行为)
        app.should_quit = true;
        return true;
    }

    // R26-3-fixes: Esc 行为优先级 (focus mode)
    // 1. 有选区 -> 清选区
    // 2. input_focused -> 退出 focus (数字键恢复切 nav)
    // 3. input_buf 非空 -> 清空
    if key.code == KeyCode::Esc && app.nav == NavPage::Dialogue {
        if app.selection.is_some() {
            app.selection = None;
            return false;
        }
        if app.input_focused {
            app.input_focused = false;
            return false;
        }
        app.input_buf.clear();
        app.input_cursor = 0;
        app.history_idx = None;
        return false;
    }

    // 全局: q / Ctrl-O 切 thinking 展开
    match key.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
            return true;
        }
        KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.nav == NavPage::Dialogue {
                app.thinking_expanded = !app.thinking_expanded;
            }
            return false;
        }
        _ => {}
    }

    // R26-3-fixes: 数字键直跳
    // - 非 Dialogue 页: 数字切 nav (不变)
    // - Dialogue 页:
    //   - input_focused=true: 数字进 input_buf (输数字)
    //   - input_focused=false: 数字切 nav
    if let KeyCode::Char(c) = key.code {
        if let Some(d) = c.to_digit(10) {
            let can_nav = app.nav != NavPage::Dialogue || !app.input_focused;
            if can_nav {
                if let Some(p) = NavPage::from_u8(d as u8) {
                    app.nav = p;
                    // R26-3-fixes: 切 nav 后 input_focused 重置 (不同页 focus 状态无关)
                    if app.nav == NavPage::Dialogue {
                        app.input_focused = true;
                    }
                    return false;
                }
            }
        }
    }
    match key.code {
        KeyCode::Tab => {
            app.nav = app.nav.next();
        }
        KeyCode::BackTab => {
            app.nav = app.nav.prev();
        }
        // W2.6 修: Right/Left 不在 outer 切 nav, 让 dialogue 页接管调光标
        _ => match app.nav {
            NavPage::Bridge => {
                if let KeyCode::Char('i') = key.code {
                    app.nav = NavPage::Dialogue;
                } else if let KeyCode::Enter = key.code {
                    app.nav = NavPage::Dialogue;
                }
            }
            NavPage::Dialogue => {
                handle_dialogue_key(app, key);
            }
            NavPage::Settings => {
                handle_settings_key(app, key);
            }
            _ => {}
        },
    }
    false
}

fn handle_dialogue_key(app: &mut App, key: KeyEvent) {
    // R26-3-fixes: 移除 processing 早返回, 用户可以在 AI 处理期间输入字符 (作在 input_buf)
    // (AI 完成后 run_app 自动提交 pending_input)

    // W2.6 修: input_buf 改 Vec<char>, cursor 按 char 索引
    // Char 在 cursor 处插入, Backspace 删 cursor 前一字符
    // Left/Right/Home/End 调 cursor
    match key.code {
        KeyCode::Char(c) => {
            // R26-3-fixes: 用户开始新输入, 退出历史导航
            app.history_idx = None;
            // 同时清选区 (用户开始编辑, 选区无意义)
            app.selection = None;
            app.input_buf.insert(app.input_cursor, c);
            app.input_cursor += 1;
        }
        KeyCode::Backspace => {
            // R26-3-fixes: 编辑也算新输入, 退出历史导航
            app.history_idx = None;
            app.selection = None;
            if app.input_cursor > 0 {
                app.input_buf.remove(app.input_cursor - 1);
                app.input_cursor -= 1;
            }
        }
        KeyCode::Left => {
            if app.input_cursor > 0 {
                app.input_cursor -= 1;
            }
        }
        KeyCode::Right => {
            if app.input_cursor < app.input_buf.len() {
                app.input_cursor += 1;
            }
        }
        KeyCode::Home => {
            app.input_cursor = 0;
        }
        KeyCode::End => {
            app.input_cursor = app.input_buf.len();
        }
        KeyCode::Enter => {
            let s: String = app.input_buf.iter().collect();
            app.input_buf.clear();
            app.input_cursor = 0;
            app.history_idx = None; // R26-3-fixes: 提交后退出历史导航
            if s.trim().is_empty() {
                return; // 空输入不提交
            }
            // R26-3-fixes: 提交后 push 到 input_history 并持久化
            app.input_history = crate::persistence::push_input_history(
                std::mem::take(&mut app.input_history),
                s.clone(),
            );
            crate::persistence::save_input_history(&app.input_history);

            if app.processing {
                // R26-3-fixes: AI 处理中, 用户提前输入的内容先充兛
                // AI 完成后 run_app 自动提交 pending_input
                app.pending_input = Some(s);
                return;
            }
            app.push_user_input(s.clone());
            // W3 #1 流式: spawn thread 调 chat_streaming, channel 收多个 chunks
            // 每个 chunk 50 chars (跟 Claude Code TUI 体验对齐)
            // run_app tick 收 chunk → 累加 streaming_message, channel close 时 commit
            let (tx, rx) = std::sync::mpsc::channel();
            app.chat_rx = Some(rx);
            app.processing = true;
            app.streaming_message = Some(String::new());
            // R26-3-fixes: 提交时清选区 (用户开始新对话)
            app.selection = None;
            let history: Vec<ChatMessage> = app.chat_history.clone();
            std::thread::spawn(move || {
                backend::chat_streaming(&s, &history, &tx);
            });
        }
        // R26-3-fixes: ↑/↓ 在 input_history 里导航 (PowerShell / codex 风格)
        // - 首次按 ↑: 跳到最后一条, history_idx = Some(last)
        // - 再按 ↑: 向前一条 (越界就停在最旧)
        // - 按 ↓: 向后一条 (回到 None 时还原当前正在编辑的内容, 但简化: 直接清空)
        KeyCode::Up => {
            if app.input_history.is_empty() {
                return;
            }
            let next_idx = match app.history_idx {
                None => app.input_history.len() - 1,
                Some(0) => 0,
                Some(i) => i - 1,
            };
            app.history_idx = Some(next_idx);
            let entry = &app.input_history[next_idx];
            app.input_buf = entry.chars().collect();
            app.input_cursor = app.input_buf.len();
        }
        KeyCode::Down => match app.history_idx {
            None => return,
            Some(i) if i + 1 >= app.input_history.len() => {
                app.history_idx = None;
                app.input_buf.clear();
                app.input_cursor = 0;
            }
            Some(i) => {
                let next_idx = i + 1;
                app.history_idx = Some(next_idx);
                let entry = &app.input_history[next_idx];
                app.input_buf = entry.chars().collect();
                app.input_cursor = app.input_buf.len();
            }
        },
        // R26-3-fixes: PageUp/PageDown 滚对话 / 历史页 (锚到底 通过 scroll_to_bottom 标志)
        KeyCode::PageUp => {
            // 向上滚: 看更早内容, 断定锚到底
            app.scroll_to_bottom = false;
            app.scroll_offset = app.scroll_offset.saturating_add(5);
        }
        KeyCode::PageDown => {
            if app.scroll_to_bottom {
                // 已锚到底, no-op
                return;
            }
            if app.scroll_offset < 5 {
                // 滚到底, 锚住
                app.scroll_to_bottom = true;
                app.scroll_offset = 0;
            } else {
                app.scroll_offset -= 5;
            }
        }
        // Esc 在 outer match 全局处理 (clear)
        _ => {}
    }
}

fn handle_settings_key(app: &mut App, key: KeyEvent) {
    // R26-2 cursor navigation: 5 项 0..4; [j][k] 或 [Down][Up] 切换高亮项
    // 按 m/t/s/b/l 直接 toggle 同时把 cursor 移到对应索引 (UX 一致)
    // W3.5 即时持久化: 改了任一字段就 save 一次 (失败不 panic, 走 stderr 提示)
    let mut changed = false;
    match key.code {
        // R26-3-fixes: cursor navigation (clamp 到 0..=5, 6 items: mode/theme/splash/breath/lang/output)
        KeyCode::Char('j') | KeyCode::Down => {
            if app.settings_cursor < 5 {
                app.settings_cursor += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.settings_cursor > 0 {
                app.settings_cursor -= 1;
            }
        }
        // [m] 模式 — cursor=0
        KeyCode::Char('m') => {
            app.mode = app.mode.toggle();
            app.settings_cursor = 0;
            changed = true;
        }
        // [t] 主题 — cursor=1 (W3.6 平滑过渡: 200ms RGB 渐变)
        KeyCode::Char('t') => {
            app.begin_theme_transition(app.theme.toggle());
            app.settings_cursor = 1;
            changed = true;
        }
        // [s] 启屏 — cursor=2
        KeyCode::Char('s') => {
            app.splash_enabled = !app.splash_enabled;
            app.settings_cursor = 2;
            changed = true;
        }
        // [b] 呼吸 — cursor=3
        KeyCode::Char('b') => {
            app.breath_enabled = !app.breath_enabled;
            app.settings_cursor = 3;
            changed = true;
        }
        // [l] 语言 — cursor=4
        KeyCode::Char('l') => {
            app.language = app.language.toggle();
            app.settings_cursor = 4;
            changed = true;
        }
        // R26-3-fixes: [=] 加 / [-] 减 — cursor=5 (输入法翻页手感)
        // j/k 在 cursor=5 时不再调 max_tokens (避免误触), 用 [+] / [-] 单独键
        KeyCode::Char('=') => {
            if let Some(mut cfg) = llm_config::load() {
                cfg.max_tokens = (cfg.max_tokens + 1024).min(32_768);
                let _ = llm_config::save(&cfg);
                app.settings_cursor = 5;
                changed = true;
            }
        }
        KeyCode::Char('-') => {
            if let Some(mut cfg) = llm_config::load() {
                cfg.max_tokens = cfg.max_tokens.saturating_sub(1024).max(512);
                let _ = llm_config::save(&cfg);
                app.settings_cursor = 5;
                changed = true;
            }
        }
        _ => {}
    }
    if changed {
        let after = app.to_settings();
        if let Err(e) = persistence::save(&after) {
            eprintln!("[settings] save failed: {e}");
        }
    }
}

// ============================================================
// 渲染
// ============================================================

/// R28 mouse 偏移修复: 独立出 layout 计算, ui() 和 handle_dialogue_mouse 都用同一个
/// 返回 (nav, page, hint). 之前 mouse handler 用了全屏幕 area, 但 render 用 page area,
/// 导致鼠标 y 偏移 1 行 (鼠标点上面, 选区在下面).
fn ui_chunks(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
    (chunks[0], chunks[1], chunks[2])
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let area = f.area();
    // W3.6 平滑过渡: 不再静态 ThemeStyle::of(theme), 走 current_style() —
    // 渐变期 RGB 线性插值, 静态期同 of().
    let style = app.current_style();

    let (nav, page, hint) = ui_chunks(area);

    render_nav_bar(f, nav, app, &style);
    render_current_page(f, page, app, &style);
    render_hint(f, hint, app, &style);
}

fn render_nav_bar(f: &mut ratatui::Frame, area: Rect, app: &App, style: &ThemeStyle) {
    let pages: [NavPage; 5] = [
        NavPage::Bridge,
        NavPage::Dialogue,
        NavPage::Growth,
        NavPage::History,
        NavPage::Settings,
    ];
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(
        " APEIRETH ",
        Style::default()
            .fg(style.accent)
            .add_modifier(Modifier::BOLD),
    ));
    // R26-2 breath: 8 步 Unicode Braille 常驻动画 (任何 nav 都可见)
    // breath_enabled=false 时停在空心; splash_active 时也算 (但 splash 时不渲染 nav, 所以自然隐藏)
    let breath_chars = ["⠀", "⠁", "⡃", "⡇", "⣇", "⣧", "⣿", "⣿"];
    let breath_idx = if app.breath_enabled {
        ((app.breath_phase.sin() + 1.0) * 0.5 * 7.0).round() as usize % 8
    } else {
        0
    };
    let breath_color = if app.breath_enabled {
        style.primary
    } else {
        style.dim
    };
    spans.push(Span::styled(
        format!(" {} ", breath_chars[breath_idx]),
        Style::default().fg(breath_color),
    ));
    // R26-2 nav: 纯标签, 不写键提示数字 (键位 0-4 在 hint 里告知, 不在 nav 里占位)
    for p in &pages {
        let active = *p == app.nav;
        let marker = if active { "▣" } else { "□" };
        let (color, modifier) = if active {
            (style.accent, Modifier::BOLD | Modifier::REVERSED)
        } else {
            (style.dim, Modifier::empty())
        };
        spans.push(Span::styled(
            format!(" {} {} ", marker, p.label_zh()),
            Style::default().fg(color).add_modifier(modifier),
        ));
        spans.push(Span::styled(" ", Style::default().fg(style.dim)));
    }
    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line), area);
}

fn render_current_page(f: &mut ratatui::Frame, area: Rect, app: &mut App, style: &ThemeStyle) {
    // R135: split bottom strip for addon panels (per docs/tui-r135-integration-design.md)
    // 0 触碰既有 page render — 仅 layout 切条 + 调用 r135_addons
    match app.nav {
        NavPage::Bridge => {
            let rows = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Min(0),
                    ratatui::layout::Constraint::Length(pages::r135_addons::BRIDGE_STRIP_HEIGHT),
                ])
                .split(area);
            pages::bridge::render(f, rows[0], app, style);
            pages::r135_addons::render_bridge_strip(f, rows[1], app, style);
        }
        NavPage::Dialogue => pages::dialogue::render(f, area, app, style),
        NavPage::Growth => {
            let rows = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Min(0),
                    ratatui::layout::Constraint::Length(pages::r135_addons::SINGLE_STRIP_HEIGHT),
                ])
                .split(area);
            pages::growth::render(f, rows[0], app, style);
            pages::r135_addons::render_formal_proofs(f, rows[1], app, style);
        }
        NavPage::History => {
            let rows = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Min(0),
                    ratatui::layout::Constraint::Length(pages::r135_addons::SINGLE_STRIP_HEIGHT),
                ])
                .split(area);
            pages::history::render(f, rows[0], app, style);
            pages::r135_addons::render_repo_scan(f, rows[1], app, style);
        }
        NavPage::Settings => {
            let rows = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Min(0),
                    ratatui::layout::Constraint::Length(pages::r135_addons::SETTINGS_STRIP_HEIGHT),
                ])
                .split(area);
            pages::settings::render(f, rows[0], app, style);
            pages::r135_addons::render_settings_strip(f, rows[1], app, style);
        }
    }
}

fn render_hint(f: &mut ratatui::Frame, area: Rect, app: &App, style: &ThemeStyle) {
    let hint = match app.nav {
        NavPage::Bridge => "[i/Enter] dialogue · [Tab] next · [0-4] jump · [q] quit",
        NavPage::Dialogue => {
            "[Enter] submit · [Esc] clear · [↑] last user msg · [Tab] next · [q] quit"
        }
        NavPage::Growth => "[Tab] next · [0-4] jump · [q] quit",
        NavPage::History => "[Tab] next · [0-4] jump · [q] quit",
        NavPage::Settings => {
            "[m] mode · [t] theme · [s] splash · [b] breath · [l] lang · [Tab] next · [q] quit"
        }
    };
    let spans = vec![
        Span::styled(" ", Style::default().fg(style.dim)),
        Span::styled(hint, Style::default().fg(style.dim)),
        Span::styled(
            format!(" · 主题={} ", app.theme.display_label()),
            Style::default().fg(style.accent),
        ),
    ];
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[allow(dead_code)]
fn _theme_anchor(t: Theme) -> Theme {
    t
}

// ============================================================
// R26-3-fixes: 选区复制 (Ctrl+C when selection exists)
// ============================================================
//
// **设计**: TUI 内置选区追踪, 不依赖终端原生 drag-select (被 EnableMouseCapture 拦截).
// - selection: ((msg_a, char_a), (msg_b, char_b))
//   char_idx 是 raw 消息字符索引 (中文/emoji 都按 char 算)
// - 跨消息选区也支持 (msg_a < msg_b 时合并中间所有消息)
// - Ctrl+C 复制选区文本到系统剪贴板 (arboard crate)
// - 复制后清选区 (codex 风格)

/// R28 char-level: 选区字符数 (状态栏 "已选 N 字符" 用)
/// 代理到 dialogue::compute_selection_char_count, 跟复制内容严格对齐.
pub fn selection_char_count(app: &App) -> usize {
    crate::pages::dialogue::compute_selection_char_count(app)
}

/// R28 char-level: 提取选区文本 (用于 Ctrl+C 复制)
/// 走字符粒度切片 (不是整行), 跨行用 \n 拼接
pub fn selection_text(app: &App) -> String {
    let Some(((line_a, char_a), (line_b, char_b))) = app.selection else {
        return String::new();
    };
    let (lo_line, lo_char, hi_line, hi_char) = if (line_a, char_a) <= (line_b, char_b) {
        (line_a, char_a, line_b, char_b)
    } else {
        (line_b, char_b, line_a, char_a)
    };
    if lo_line == hi_line && lo_char == hi_char {
        return String::new();
    }
    let mut out = String::new();
    for i in lo_line..=hi_line {
        let info = match app.chat_line_map.get(i) {
            Some(info) => info,
            None => continue,
        };
        let total = info.text.chars().count();
        let s = if i == lo_line { lo_char.min(total) } else { 0 };
        let e = if i == hi_line {
            hi_char.min(total)
        } else {
            total
        };
        if i > lo_line {
            out.push('\n');
        }
        out.extend(info.text.chars().skip(s).take(e.saturating_sub(s)));
    }
    out
}

/// R28 char-level: 复制选区到系统剪贴板 (Ctrl+C handler)
/// 零长度选区给反馈 (用户能看到为什么 Ctrl+C 没动静)
fn copy_selection_to_clipboard(app: &mut App) {
    let text = selection_text(app);
    if text.is_empty() {
        // 选区为空 → 给反馈, 不静默失败
        app.copy_feedback = Some((
            "选区为空 — 请拖动鼠标选区".to_string(),
            std::time::Instant::now(),
        ));
        return;
    }
    let char_count = text.chars().count();
    match arboard::Clipboard::new() {
        Ok(mut cb) => match cb.set_text(text.clone()) {
            Ok(()) => {
                app.copy_feedback = Some((
                    format!("已复制 {} 字符", char_count),
                    std::time::Instant::now(),
                ));
            }
            Err(e) => {
                eprintln!("[apeireth-tui] warn: clipboard set_text: {e}");
                app.copy_feedback = Some((format!("复制失败: {}", e), std::time::Instant::now()));
            }
        },
        Err(e) => {
            eprintln!("[apeireth-tui] warn: clipboard new: {e}");
            app.copy_feedback = Some((
                format!("剪贴板初始化失败: {}", e),
                std::time::Instant::now(),
            ));
        }
    }
    // 复制后清选区 (codex 风格)
    app.selection = None;
}

// ============================================================
// R26-3-fixes: 鼠标点击切 focus + 拖拽选区
// ============================================================
//
// **设计**: EnableMouseCapture 拦截所有鼠标事件, 终端原生 drag-to-select 不可用.
// 所以我们在 TUI 内置选区追踪:
// - 单击 input 区域 -> input_focused = true, 清选区
// - 单击 chat 区域 -> input_focused = false (用户在看历史, 准备拖选)
// - 按下 + 拖动 -> selection 追踪 (msg_idx, char_a/b)
// - 释放鼠标 -> selection 定格 (等 Ctrl+C)
// - 滚轮 -> scroll (已有, 不变)

/// Dialogue 页布局参考 (跟 pages/dialogue.rs::render 一致)
/// - chunks[0]: chat 历史 (Min(5))
/// - chunks[1]: 状态栏 (Length(1))
/// - chunks[2]: 输入框 (Length(3))
fn dialogue_input_rect(area: Rect) -> Rect {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(area);
    chunks[2]
}

/// Dialogue 页 chat 历史 rect (含 borders)
fn dialogue_chat_rect(area: Rect) -> Rect {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(area);
    chunks[0]
}

/// R28 char-level: 把屏幕 (col, row) 映射到 chat_line_map 视觉行 + 字符偏移
/// - 返回 None: 点在 chat 区域外 / chat 为空 / row 超出 map
/// - 返回 Some((line_idx, char_offset_within_line)): char_offset 用 UnicodeWidthChar 把 mx-prefix_cols 转换成字符索引
fn chat_line_idx(app: &App, mx: u16, my: u16, chat_rect: Rect) -> Option<(usize, usize)> {
    if mx < chat_rect.x
        || mx >= chat_rect.x + chat_rect.width
        || my < chat_rect.y
        || my >= chat_rect.y + chat_rect.height
    {
        return None;
    }
    if app.chat_line_map.is_empty() {
        return None;
    }
    // **R28 修正**: 之前用 `visual_row_in_view + scroll_offset` 当 line_idx,
    // 但 Paragraph::scroll.y = max_scroll - scroll_offset (= 从顶部跳过多少行),
    // 所以视口第 0 行显示 chat_line_map[scroll] (= max_scroll - scroll_offset).
    // 鼠标在视口第 N 行 -> chat_line_map[scroll + N].
    // 之前公式 bug: 用 scroll_offset 当基底, 用户滚轮往上滚后 highlight 偏移 (可能跑到顶部).
    let inner_y = chat_rect.y + 1;
    let inner_height = (chat_rect.height as usize).saturating_sub(2); // border 1+1
    let visual_row_in_view = (i32::from(my) - i32::from(inner_y)).max(0) as usize;
    // 计算 scroll (Paragraph y offset)
    let total = app.chat_line_map.len();
    let max_scroll = total.saturating_sub(inner_height);
    let scroll = if app.scroll_to_bottom {
        max_scroll
    } else {
        max_scroll.saturating_sub(app.scroll_offset as usize)
    };
    // 视口第 visual_row_in_view 行 -> chat_line_map[scroll + visual_row_in_view]
    let mut line_idx = scroll + visual_row_in_view;
    if line_idx >= total {
        line_idx = total - 1;
    }
    let prefix_cols = app
        .chat_line_map
        .get(line_idx)
        .map(|i| i.prefix_cols)
        .unwrap_or(0);
    let inner_x = chat_rect.x + 1;
    let col_in_text = if mx > inner_x + prefix_cols {
        (mx - inner_x - prefix_cols) as usize
    } else {
        0
    };
    let char_off = display_col_to_char_offset(app, line_idx, col_in_text);
    Some((line_idx, char_off))
}

/// R28: 把该行内的显示列宽 (0..=width) 转换成字符索引.
/// 走 unicode_width::UnicodeWidthChar.width(c) 累加, 超出走到末尾.
fn display_col_to_char_offset(app: &App, line_idx: usize, col: usize) -> usize {
    let Some(info) = app.chat_line_map.get(line_idx) else {
        return 0;
    };
    let mut acc = 0usize;
    for (i, c) in info.text.chars().enumerate() {
        let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if acc + w > col {
            return i;
        }
        acc += w;
    }
    info.text.chars().count()
}

/// 处理 Dialogue 页鼠标事件 (focus + 选区)
/// R26-3-fixes: 选区按视觉行粒度 (不是按消息)
/// - 单击: selection = (line, line) — 零长度, 视觉上看不到
/// - 拖拽: selection = (line_a, line_b) — 视觉行按 REVERSED 高亮
/// - Up: 不清选区, 即使 ma == mb (留给 Ctrl+C / Esc 处理)
fn handle_dialogue_mouse(app: &mut App, mouse: crossterm::event::MouseEvent, area: Rect) {
    use crossterm::event::{MouseButton, MouseEventKind};
    let input_rect = dialogue_input_rect(area);
    let chat_rect = dialogue_chat_rect(area);
    let (mx, my) = (mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // 点 input 区域 -> enter focus + 清选区
            if mx >= input_rect.x
                && mx < input_rect.x + input_rect.width
                && my >= input_rect.y
                && my < input_rect.y + input_rect.height
            {
                app.input_focused = true;
                app.selection = None;
                app.history_idx = None;
                return;
            }
            // 点 chat 区域 -> exit focus + 开始选区
            app.input_focused = false;
            if let Some((line_idx, char_off)) = chat_line_idx(app, mx, my, chat_rect) {
                app.selection = Some(((line_idx, char_off), (line_idx, char_off)));
            } else if !app.chat_line_map.is_empty() {
                // 边界情况: 选最后一行末尾
                let last_line = app.chat_line_map.len() - 1;
                let last_chars = app.chat_line_map[last_line].text.chars().count();
                app.selection = Some(((last_line, last_chars), (last_line, last_chars)));
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some((anchor, _)) = app.selection {
                if let Some((line_idx, char_off)) = chat_line_idx(app, mx, my, chat_rect) {
                    // R28 char-level: anchor 保持原点, drag end 随鼠标变化
                    app.selection = Some((anchor, (line_idx, char_off)));
                }
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            // R26-3-fixes: Up 不清选区 (即使 ma == mb)
            // 单击没拖 -> 零长度选区 (视觉上看不到), Esc 清
        }
        _ => {}
    }
}

// ============================================================
// R27 C 方案: API 连通性探测 (status_bar 实时显示 ● 或 ✗)
// ============================================================

/// R27 C 方案: 解析 base_url 出 host:port, 尝试 TCP 连接 (500ms 超时).
/// 不发 HTTP, 只探 socket — 轻量, 不会拖慢启动.
/// true = 连得上 (apeireth-api daemon 在跑), false = 不通.
fn probe_api_online(base_url: &str) -> bool {
    let Some(host) = parse_host_port(base_url) else {
        return false;
    };
    let Ok(addr) = host.parse::<std::net::SocketAddr>() else {
        return false;
    };
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(500)).is_ok()
}

/// R31 委托: parse_host_port 已搬到 backend 模块 (让 tests/ 也可见).
/// 留 re-export 保兼容.
pub use crate::backend::parse_host_port;
