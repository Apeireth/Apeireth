pub mod state;
pub mod theme;
pub mod ui;
pub mod widgets;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use chrono::Utc;

use apeireth_runtime::UnifiedRuntimeHost;
use apeireth_storage::memory_v2::QueryMode;
use self::state::{AppState, NavPage, ChatMessageItem};

/// RAII Terminal Cleanup Guard to guarantee terminal reset on normal exit, error, or panic
struct TerminalGuard;

impl TerminalGuard {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }
}

/// Runs the full-screen Ratatui TUI Cognitive Companion Workbench
pub async fn run_tui(api_key: String, db_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Initialize Host & Database first (fails cleanly to stderr without touching terminal if error occurs)
    let session_id = format!("tui_{}", uuid::Uuid::new_v4());
    let host = Arc::new(UnifiedRuntimeHost::new(api_key, db_path).await?);

    let state = Arc::new(Mutex::new(AppState::new(session_id.clone())));

    // Load initial memories into state
    {
        let mut st = state.lock().await;
        if let Ok(memories) = host.memory_store.query(Utc::now(), QueryMode::All).await {
            st.memory_items = memories;
        }
        if let Ok(audit) = host.audit_chain.try_lock() {
            st.audit_chain_length = audit.records().len();
        }
    }

    // 2. Setup Terminal and RAII Guard
    let _guard = TerminalGuard::new()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 3. Setup Panic Hook for backup safety
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        default_hook(panic_info);
    }));

    // 4. Main Event & Render Loop
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = std::time::Instant::now();

    loop {
        // Draw Frame
        {
            let st = state.lock().await;
            terminal.draw(|f| ui::render_ui(f, &st))?;
        }

        // Poll Events with timeout
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // IMPORTANT: Filter out Release and Repeat events on Windows/crossterm to prevent double typing!
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                let mut st = state.lock().await;

                // Global Exit check (Ctrl+C)
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }

                // Global Thinking fold toggle (Ctrl+O)
                if key.modifiers.contains(KeyModifiers::CONTROL) && (key.code == KeyCode::Char('o') || key.code == KeyCode::Char('O')) {
                    st.thinking_expanded = !st.thinking_expanded;
                    let msg = if st.thinking_expanded { "已展开思考链" } else { "已折叠思考链" };
                    st.set_status(msg);
                    continue;
                }

                // Global Page Navigation via 0-4
                match key.code {
                    KeyCode::Char('0') => { st.current_page = NavPage::Bridge; st.set_status("已跳转至 0 舰桥 (ΣΚΟΠΗ)"); continue; }
                    KeyCode::Char('1') if st.current_page != NavPage::Dialogue => { st.current_page = NavPage::Dialogue; st.set_status("已跳转至 1 对话 (ΔΙΑΛΟΓΟΣ)"); continue; }
                    KeyCode::Char('2') if st.current_page != NavPage::Dialogue => { st.current_page = NavPage::Growth; st.set_status("已跳转至 2 生长 (ΑΥΞΗΣΙΣ)"); continue; }
                    KeyCode::Char('3') if st.current_page != NavPage::Dialogue => { st.current_page = NavPage::History; st.set_status("已跳转至 3 历史 (ΙΣΤΟΡΙΑ)"); continue; }
                    KeyCode::Char('4') if st.current_page != NavPage::Dialogue => { st.current_page = NavPage::Settings; st.set_status("已跳转至 4 设置 (ΤΑΞΙΣ)"); continue; }
                    _ => {}
                }

                // Global Tab / BackTab Navigation
                if key.code == KeyCode::Tab {
                    let next = st.current_page.next();
                    st.current_page = next;
                    let title = next.title();
                    st.set_status(format!("已切换至 {}", title));
                    continue;
                }
                if key.code == KeyCode::BackTab {
                    let prev = st.current_page.prev();
                    st.current_page = prev;
                    let title = prev.title();
                    st.set_status(format!("已切换至 {}", title));
                    continue;
                }

                // Page-Specific Key Handling
                match st.current_page {
                    NavPage::Bridge => {
                        match key.code {
                            KeyCode::Char('t') | KeyCode::Char('T') => {
                                st.toggle_theme();
                            }
                            KeyCode::Char('i') | KeyCode::Char('I') | KeyCode::Enter => {
                                st.current_page = NavPage::Dialogue;
                                st.set_status("进入对话 (Dialogue)");
                            }
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                break;
                            }
                            _ => {}
                        }
                    }
                    NavPage::Dialogue => {
                        match key.code {
                            KeyCode::Enter => {
                                let input_text = st.input_buffer.trim().to_string();
                                if !input_text.is_empty() && !st.is_thinking {
                                    let current_pad = st.current_pad.clone();
                                    let sess_id = st.session_id.clone();
                                    st.input_buffer.clear();
                                    st.is_thinking = true;
                                    st.messages.push(ChatMessageItem {
                                        role: "user".into(),
                                        content: input_text.clone(),
                                        cot: None,
                                        pad: current_pad,
                                        tokens: input_text.len() / 4 + 1,
                                        audit_hash: format!("user_{:x}", Utc::now().timestamp()),
                                        timestamp_ms: Utc::now().timestamp_millis(),
                                    });

                                    let host_clone = host.clone();
                                    let state_clone = state.clone();

                                    tokio::spawn(async move {
                                        let res = host_clone.handle_chat_turn(&sess_id, &input_text).await;
                                        let mut inner_st = state_clone.lock().await;
                                        inner_st.is_thinking = false;

                                        match res {
                                            Ok(turn) => {
                                                inner_st.current_pad = turn.pad_state.clone();
                                                inner_st.messages.push(ChatMessageItem {
                                                    role: "assistant".into(),
                                                    content: turn.assistant_text,
                                                    cot: turn.reasoning_cot,
                                                    pad: turn.pad_state,
                                                    tokens: turn.token_usage.total_tokens as usize,
                                                    audit_hash: turn.audit_hash,
                                                    timestamp_ms: turn.timestamp * 1000,
                                                });

                                                // Update memories & audit height
                                                if let Ok(mems) = host_clone.memory_store.query(Utc::now(), QueryMode::All).await {
                                                    inner_st.memory_items = mems;
                                                }
                                                if let Ok(audit) = host_clone.audit_chain.try_lock() {
                                                    inner_st.audit_chain_length = audit.records().len();
                                                }

                                                inner_st.set_status("消息已送达并完成记忆沉淀。");
                                            }
                                            Err(err) => {
                                                inner_st.set_status(format!("对话异常: {}", err));
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Backspace => {
                                st.input_buffer.pop();
                            }
                            KeyCode::Char(c) => {
                                st.input_buffer.push(c);
                            }
                            KeyCode::PageUp => {
                                if st.message_scroll > 0 {
                                    st.message_scroll -= 5;
                                }
                            }
                            KeyCode::PageDown => {
                                st.message_scroll += 5;
                            }
                            _ => {}
                        }
                    }
                    NavPage::Growth => {
                        match key.code {
                            KeyCode::Up => {
                                if st.memory_selected > 0 {
                                    st.memory_selected -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if st.memory_selected + 1 < st.memory_items.len() {
                                    st.memory_selected += 1;
                                }
                            }
                            KeyCode::Char('t') | KeyCode::Char('T') => {
                                st.toggle_theme();
                            }
                            KeyCode::Char('q') => {
                                break;
                            }
                            _ => {}
                        }
                    }
                    NavPage::History => {
                        match key.code {
                            KeyCode::Char('t') | KeyCode::Char('T') => {
                                st.toggle_theme();
                            }
                            KeyCode::Char('q') => {
                                break;
                            }
                            _ => {}
                        }
                    }
                    NavPage::Settings => {
                        match key.code {
                            KeyCode::Char('t') | KeyCode::Char('T') => {
                                st.toggle_theme();
                            }
                            KeyCode::Char('q') => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = std::time::Instant::now();
        }
    }

    println!("Apeireth 2.0 TUI Workbench closed cleanly. Cognitive session preserved.");
    Ok(())
}
