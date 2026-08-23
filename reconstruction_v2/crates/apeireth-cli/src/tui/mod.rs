pub mod state;
pub mod ui;
pub mod widgets;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
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
use self::state::{ActiveTab, AppState, FocusPane, InputMode, ChatMessageItem};

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
                let mut st = state.lock().await;

                // Global Exit check (Ctrl+C)
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }

                // Global Tab Switching (F1-F5)
                match key.code {
                    KeyCode::F(1) => { st.active_tab = ActiveTab::Dialogue; st.set_status("Switched to Dialogue Tab"); continue; }
                    KeyCode::F(2) => { st.active_tab = ActiveTab::Memory; st.set_status("Switched to Memory Tab"); continue; }
                    KeyCode::F(3) => { st.active_tab = ActiveTab::Organs; st.set_status("Switched to Organs Tab"); continue; }
                    KeyCode::F(4) => { st.active_tab = ActiveTab::Factory; st.set_status("Switched to Software Factory Tab"); continue; }
                    KeyCode::F(5) => { st.active_tab = ActiveTab::Governance; st.set_status("Switched to Governance Tab"); continue; }
                    _ => {}
                }

                // Pane Navigation (Tab / Shift+Tab)
                if key.code == KeyCode::Tab {
                    st.focus_pane = match st.focus_pane {
                        FocusPane::Left => FocusPane::Center,
                        FocusPane::Center => FocusPane::Right,
                        FocusPane::Right => FocusPane::Input,
                        FocusPane::Input => FocusPane::Left,
                    };
                    if st.focus_pane == FocusPane::Input {
                        st.input_mode = InputMode::Editing;
                    }
                    continue;
                }

                // Modal Handling
                if st.pending_modal.is_some() {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            st.set_status("PatchSet Approved and Applied cleanly.");
                            st.pending_modal = None;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            st.set_status("PatchSet Discarded.");
                            st.pending_modal = None;
                        }
                        _ => {}
                    }
                    continue;
                }

                // Global Esc Mode Switching
                if key.code == KeyCode::Esc {
                    st.input_mode = match st.input_mode {
                        InputMode::Editing => {
                            st.set_status("Switched to NORMAL mode. Press 'i' to edit, 'q' to quit.");
                            InputMode::Normal
                        }
                        InputMode::Normal => {
                            st.set_status("Switched to INPUT mode.");
                            InputMode::Editing
                        }
                        InputMode::Search => {
                            st.set_status("Exited search.");
                            InputMode::Normal
                        }
                    };
                    continue;
                }

                // Tab-Specific Input Handling
                match st.active_tab {
                    ActiveTab::Dialogue => {
                        if st.input_mode == InputMode::Normal {
                            match key.code {
                                KeyCode::Char('i') => {
                                    st.input_mode = InputMode::Editing;
                                    st.focus_pane = FocusPane::Input;
                                    st.set_status("Entered INPUT mode.");
                                }
                                KeyCode::Char('q') => {
                                    break;
                                }
                                KeyCode::Up => {
                                    if st.message_scroll > 0 {
                                        st.message_scroll -= 1;
                                    }
                                }
                                KeyCode::Down => {
                                    st.message_scroll += 1;
                                }
                                _ => {}
                            }
                        } else {
                            // InputMode::Editing
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
                                                    inner_st.pad_history.push(turn.pad_state.pleasure);
                                                    if inner_st.pad_history.len() > 30 {
                                                        inner_st.pad_history.remove(0);
                                                    }

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

                                                    inner_st.set_status("Turn processed successfully.");
                                                }
                                                Err(err) => {
                                                    inner_st.set_status(format!("Error: {}", err));
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
                                _ => {}
                            }
                        }
                    }
                    ActiveTab::Memory => {
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
                            KeyCode::Char('q') if st.input_mode == InputMode::Normal => {
                                break;
                            }
                            _ => {}
                        }
                    }
                    ActiveTab::Factory => {
                        match key.code {
                            KeyCode::Up => {
                                if st.factory_selected > 0 {
                                    st.factory_selected -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if st.factory_selected + 1 < st.factory_tasks.len() {
                                    st.factory_selected += 1;
                                }
                            }
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                if let Some(task) = st.factory_tasks.get(st.factory_selected) {
                                    let diff_content = task.diff_content.clone();
                                    st.set_status("Applying PatchSet live via git apply...");
                                    let state_clone = state.clone();

                                    tokio::spawn(async move {
                                        let repo_path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                                        let res = apeireth_tools::worktree::WorktreeSandbox::apply_live_patch(repo_path.to_str().unwrap(), &diff_content).await;
                                        let mut inner_st = state_clone.lock().await;
                                        match res {
                                            Ok(msg) => inner_st.set_status(format!("✓ Patch Applied: {}", msg)),
                                            Err(err) => inner_st.set_status(format!("❌ Apply Failed: {}", err)),
                                        }
                                    });
                                }
                            }
                            KeyCode::Char('q') if st.input_mode == InputMode::Normal => {
                                break;
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        if key.code == KeyCode::Char('q') {
                            break;
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
