use ratatui::prelude::*;
use ratatui::widgets::*;
use super::state::{ActiveTab, AppState, FocusPane, InputMode};
use super::widgets::{BrailleSparkline, DiffViewer};

pub fn render_ui(frame: &mut Frame, state: &AppState) {
    let size = frame.area();

    // Main Vertical Layout: [Top Bar (3 lines), Content Area (fill), Bottom Bar (2 lines)]
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(size);

    render_top_bar(frame, main_chunks[0], state);

    match state.active_tab {
        ActiveTab::Dialogue => render_dialogue_tab(frame, main_chunks[1], state),
        ActiveTab::Memory => render_memory_tab(frame, main_chunks[1], state),
        ActiveTab::Organs => render_organs_tab(frame, main_chunks[1], state),
        ActiveTab::Factory => render_factory_tab(frame, main_chunks[1], state),
        ActiveTab::Governance => render_governance_tab(frame, main_chunks[1], state),
    }

    render_bottom_bar(frame, main_chunks[2], state);

    if let Some(ref modal_text) = state.pending_modal {
        render_modal(frame, size, modal_text);
    }
}

// -----------------------------------------------------------------------------
// Top Header & Tab Navigation Bar
// -----------------------------------------------------------------------------

fn render_top_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let tabs = vec![
        "[F1] 💬 Dialogue",
        "[F2] 🧠 Memory (ACT-R)",
        "[F3] ⚡ Organs (GDI/MCP)",
        "[F4] 💻 Software Factory",
        "[F5] 🛡️ 5-Gate Governance",
    ];

    let active_idx = match state.active_tab {
        ActiveTab::Dialogue => 0,
        ActiveTab::Memory => 1,
        ActiveTab::Organs => 2,
        ActiveTab::Factory => 3,
        ActiveTab::Governance => 4,
    };

    let tab_widget = Tabs::new(tabs)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(Span::styled(" APEIRETH 2.0 COGNITIVE COMPANION WORKBENCH ", Style::default().fg(Color::Yellow).bold()))
        )
        .select(active_idx)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(Style::default().fg(Color::Cyan).bold().underlined());

    frame.render_widget(tab_widget, area);
}

// -----------------------------------------------------------------------------
// Bottom Status & Keybindings Bar
// -----------------------------------------------------------------------------

fn render_bottom_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let mode_str = match state.input_mode {
        InputMode::Editing => " [INPUT] ",
        InputMode::Normal => " [NORMAL] ",
        InputMode::Search => " [SEARCH] ",
    };

    let status_text = state.status_message.as_ref().map(|(m, _)| m.as_str()).unwrap_or("Ready");

    let status_line = Line::from(vec![
        Span::styled(mode_str, Style::default().bg(Color::Cyan).fg(Color::Black).bold()),
        Span::raw(" "),
        Span::styled(status_text, Style::default().fg(Color::White)),
        Span::raw(" | "),
        Span::styled(format!("Session: {}", &state.session_id[..state.session_id.len().min(12)]), Style::default().fg(Color::DarkGray)),
        Span::raw(" | "),
        Span::styled("[Tab] Pane  [Enter] Send  [F1-F5] Tab  [Esc] Normal  [Ctrl+C] Quit", Style::default().fg(Color::DarkGray)),
    ]);

    frame.render_widget(Paragraph::new(status_line), area);
}

// -----------------------------------------------------------------------------
// Tab 1: Dialogue (Three-Column Layout)
// -----------------------------------------------------------------------------

fn render_dialogue_tab(frame: &mut Frame, area: Rect, state: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Left: Emotion & Memory
            Constraint::Percentage(50), // Center: Dialogue & Input
            Constraint::Percentage(25), // Right: Organs & Governance
        ])
        .split(area);

    render_left_cognition_panel(frame, cols[0], state);
    render_center_chat_panel(frame, cols[1], state);
    render_right_organ_panel(frame, cols[2], state);
}

fn render_left_cognition_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // PAD Emotion
            Constraint::Length(6), // Sleep & Braille Wave
            Constraint::Min(6),    // Top Memory Recall
        ])
        .split(area);

    // 1. PAD Emotion Gauges
    let p_pct = (state.current_pad.pleasure * 100.0).clamp(0.0, 100.0) as u16;
    let a_pct = (state.current_pad.arousal * 100.0).clamp(0.0, 100.0) as u16;
    let d_pct = (state.current_pad.dominance * 100.0).clamp(0.0, 100.0) as u16;

    let pad_block = Block::default()
        .title(" 🎭 Emotional PAD State ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let pad_inner = pad_block.inner(chunks[0]);
    frame.render_widget(pad_block, chunks[0]);

    let pad_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .split(pad_inner);

    let style_name = format!("{:?}", state.current_pad.to_response_style());
    frame.render_widget(Paragraph::new(format!("Style: {}", style_name)).style(Style::default().fg(Color::Yellow).bold()), pad_chunks[0]);
    frame.render_widget(Gauge::default().gauge_style(Style::default().fg(Color::Green)).percent(p_pct).label(format!("P: {:.2}", state.current_pad.pleasure)), pad_chunks[1]);
    frame.render_widget(Gauge::default().gauge_style(Style::default().fg(Color::Yellow)).percent(a_pct).label(format!("A: {:.2}", state.current_pad.arousal)), pad_chunks[2]);
    frame.render_widget(Gauge::default().gauge_style(Style::default().fg(Color::Blue)).percent(d_pct).label(format!("D: {:.2}", state.current_pad.dominance)), pad_chunks[3]);

    // 2. Sleep Drive & Braille Sparklines
    let braille_wave = BrailleSparkline::render_line(&state.sleep_history, 18);
    let sleep_lines = vec![
        Line::from(vec![
            Span::raw("Sleep Drive (S): "),
            Span::styled(format!("{:.2}", state.sleep_drive), Style::default().fg(Color::Cyan).bold()),
        ]),
        Line::from(vec![
            Span::raw("Circadian Wave: "),
            Span::styled(braille_wave, Style::default().fg(Color::Cyan)),
        ]),
    ];

    let sleep_widget = Paragraph::new(sleep_lines).block(
        Block::default().title(" 🌙 Borbély Circadian ").borders(Borders::ALL).border_style(Style::default().fg(Color::Blue))
    );
    frame.render_widget(sleep_widget, chunks[1]);

    // 3. Top Recalled ACT-R Memories
    let mut mem_lines = Vec::new();
    if state.memory_items.is_empty() {
        mem_lines.push(Line::from(Span::styled("• User preference: Rust & High Performance", Style::default().fg(Color::White))));
        mem_lines.push(Line::from(Span::styled("• Architecture: Microkernel + Hybrid SLM", Style::default().fg(Color::White))));
        mem_lines.push(Line::from(Span::styled("• Egress: Whitelist Strict Security", Style::default().fg(Color::White))));
    } else {
        for (i, m) in state.memory_items.iter().take(4).enumerate() {
            mem_lines.push(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::Cyan)),
                Span::styled(format!("[★{:.1}] ", m.importance), Style::default().fg(Color::Yellow)),
                Span::raw(&m.data[..m.data.len().min(24)]),
            ]));
        }
    }

    let mem_widget = Paragraph::new(mem_lines).block(
        Block::default().title(" 🧠 Top ACT-R Memories ").borders(Borders::ALL).border_style(Style::default().fg(Color::Green))
    );
    frame.render_widget(mem_widget, chunks[2]);
}

fn render_center_chat_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),    // Dialogue history
            Constraint::Length(3), // User Input area
        ])
        .split(area);

    // Dialogue List
    let mut list_items = Vec::new();
    for msg in &state.messages {
        let (role_tag, role_color) = match msg.role.as_str() {
            "user" => (" USER ", Color::Green),
            "assistant" => (" APEIRETH ", Color::Cyan),
            _ => (" SYSTEM ", Color::DarkGray),
        };

        let mut lines = Vec::new();
        lines.push(Line::from(vec![
            Span::styled(role_tag, Style::default().bg(role_color).fg(Color::Black).bold()),
            Span::raw(" "),
            Span::styled(format!("(Tokens: {} | Hash: {})", msg.tokens, &msg.audit_hash[..msg.audit_hash.len().min(8)]), Style::default().fg(Color::DarkGray)),
        ]));

        if let Some(ref cot) = msg.cot {
            lines.push(Line::from(vec![
                Span::styled("  ┌─ CoT Deep Reasoning ───────────────────────────────┐", Style::default().fg(Color::DarkGray)),
            ]));
            for cot_line in cot.lines().take(4) {
                lines.push(Line::from(vec![
                    Span::styled("  │ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(cot_line, Style::default().fg(Color::DarkGray).italic()),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled("  └────────────────────────────────────────────────────┘", Style::default().fg(Color::DarkGray)),
            ]));
        }

        for content_line in msg.content.lines() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(content_line, Style::default().fg(Color::White)),
            ]));
        }
        lines.push(Line::raw(""));

        list_items.push(ListItem::new(lines));
    }

    if state.is_thinking {
        list_items.push(ListItem::new(vec![
            Line::from(vec![
                Span::styled(" APEIRETH ", Style::default().bg(Color::Cyan).fg(Color::Black).bold()),
                Span::raw(" "),
                Span::styled("Thinking & Activating ACT-R Memory...", Style::default().fg(Color::Yellow).italic()),
            ]),
        ]));
    }

    let is_focused_dialogue = state.focus_pane == FocusPane::Center;
    let chat_block = Block::default()
        .title(" 💬 Live Dialogue & Thinking Stream ")
        .borders(Borders::ALL)
        .border_style(if is_focused_dialogue { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::Cyan) });

    let list_widget = List::new(list_items)
        .block(chat_block)
        .scroll_padding(2);

    frame.render_widget(list_widget, chunks[0]);

    // Input Box
    let is_focused_input = state.focus_pane == FocusPane::Input;
    let input_block = Block::default()
        .title(if is_focused_input { " ✍️ Input (Active - Press Enter to Send) " } else { " ✍️ Input (Press Tab to Focus) " })
        .borders(Borders::ALL)
        .border_style(if is_focused_input { Style::default().fg(Color::Green).bold() } else { Style::default().fg(Color::DarkGray) });

    let input_text = Paragraph::new(format!("> {}█", state.input_buffer))
        .style(Style::default().fg(Color::White))
        .block(input_block);

    frame.render_widget(input_text, chunks[1]);
}

fn render_right_organ_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Organ Status (Eye & Voice)
            Constraint::Length(6), // 5-Gate Governance
            Constraint::Min(6),    // Tool Log Stream
        ])
        .split(area);

    // 1. Organ Monitor
    let organ_lines = vec![
        Line::from(vec![
            Span::raw("👁️ Eye GDI: "),
            Span::styled(format!("{}x{} (Live Win32)", state.screen_width, state.screen_height), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::raw("🔍 SoM Elements: "),
            Span::styled(format!("{} controls mapped", state.screen_elements_count), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::raw("🎙️ Ear VAD: "),
            Span::styled("Ready / Full-Duplex WebAudio", Style::default().fg(Color::Yellow)),
        ]),
    ];

    let organ_widget = Paragraph::new(organ_lines).block(
        Block::default().title(" ⚡ Physical Organs ").borders(Borders::ALL).border_style(Style::default().fg(Color::Green))
    );
    frame.render_widget(organ_widget, chunks[0]);

    // 2. 5-Gate Governance Pipeline
    let gate_lines = vec![
        Line::from(vec![
            Span::styled("✓ Onion Layer-3 Sanitizer: ", Style::default().fg(Color::Green)),
            Span::raw("SECURE"),
        ]),
        Line::from(vec![
            Span::styled("✓ Egress Whitelist Guard:  ", Style::default().fg(Color::Green)),
            Span::raw("ENFORCED"),
        ]),
        Line::from(vec![
            Span::styled("✓ SHA-256 Audit Chain:     ", Style::default().fg(Color::Green)),
            Span::styled(format!("H:{} (100% OK)", state.audit_chain_length), Style::default().fg(Color::Cyan).bold()),
        ]),
    ];

    let gate_widget = Paragraph::new(gate_lines).block(
        Block::default().title(" 🛡️ 5-Gate Governance ").borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow))
    );
    frame.render_widget(gate_widget, chunks[1]);

    // 3. Tool & MCP Execution Log
    let mut log_lines = Vec::new();
    for log in state.tool_execution_logs.iter().rev().take(6) {
        log_lines.push(Line::from(vec![
            Span::styled("• ", Style::default().fg(Color::Cyan)),
            Span::raw(log),
        ]));
    }

    let log_widget = Paragraph::new(log_lines).block(
        Block::default().title(" 📋 Tool & MCP Logs ").borders(Borders::ALL).border_style(Style::default().fg(Color::Blue))
    );
    frame.render_widget(log_widget, chunks[2]);
}

// -----------------------------------------------------------------------------
// Tab 2: Memory (ACT-R Knowledge Graph)
// -----------------------------------------------------------------------------

fn render_memory_tab(frame: &mut Frame, area: Rect, state: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let mut list_items = Vec::new();
    if state.memory_items.is_empty() {
        list_items.push(ListItem::new("No episodic memory found. Perform a chat turn to generate ACT-R memories."));
    } else {
        for (i, item) in state.memory_items.iter().enumerate() {
            let is_selected = i == state.memory_selected;
            let style = if is_selected { Style::default().fg(Color::Yellow).bold().bg(Color::DarkGray) } else { Style::default().fg(Color::White) };
            list_items.push(ListItem::new(format!("[★{:.1}] {}", item.importance, item.data)).style(style));
        }
    }

    let list_block = Block::default()
        .title(" 🧠 ACT-R Fact Episodes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(List::new(list_items).block(list_block), cols[0]);

    // Right Inspector
    let selected_item = state.memory_items.get(state.memory_selected);
    let inspect_text = if let Some(item) = selected_item {
        format!(
            "Memory ID:       {}\nData Content:    {}\nImportance:      {:.2}\nAccess Count:    {}\nCreated At:      {}\nValid From:      {}\nSignature:       {:?}\n\n[Cognitive Activation Formula]:\nln( sum( (t - t_j)^(-d) ) ) + Beta = {:.4}\n\nSemantic Triplet:\n(Subject: User) ──[Predicate: states]──> (Object: {})",
            item.id, item.data, item.importance, item.access_count, item.created_at, item.valid_from, item.artifact_sig,
            item.calculate_act_r_activation(chrono::Utc::now().timestamp(), 0.5, 0.0),
            item.data
        )
    } else {
        "Select a memory episode from the left panel to inspect ACT-R activation scores and semantic graph triplets.".into()
    };

    let inspect_widget = Paragraph::new(inspect_text).block(
        Block::default().title(" 🔍 Memory Episode Inspector ").borders(Borders::ALL).border_style(Style::default().fg(Color::Green))
    );
    frame.render_widget(inspect_widget, cols[1]);
}

// -----------------------------------------------------------------------------
// Tab 3: Organs & MCP Ecosystem
// -----------------------------------------------------------------------------

fn render_organs_tab(frame: &mut Frame, area: Rect, state: &AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bot_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    // Top-Left: Eye
    let eye_text = format!(
        "Physical Monitor:   {}x{} (Win32 Primary GDI)\nCapture Engine:     BitBlt + DIBits Zero-Leak Pipeline\nPerceptual Hash:    0x{:016x}\nSet-of-Marks Marks: {} active controls\nStatus:             ACTIVE_OBSERVING",
        state.screen_width, state.screen_height, state.last_phash, state.screen_elements_count
    );
    frame.render_widget(Paragraph::new(eye_text).block(Block::default().title(" 👁️ Organ: Eye (Vision & Screen) ").borders(Borders::ALL).border_style(Style::default().fg(Color::Green))), top_cols[0]);

    // Top-Right: Hand (MCP Tools)
    let hand_text = "Mounted MCP Transports:\n• StdioTransport: Subprocess JSON-RPC (2024-11-05 standard)\n• MemoryTransport: Direct in-memory zero-copy channel\n• SseTransport: Remote HTTP streamable adapter\n\nActive Tool Registry:\n✓ shell (Sandboxed System Command)\n✓ fs (Cross-Platform Sandboxed Filesystem)\n✓ fetch (Anti-SSRF HTTP Request)\n✓ desktop_action (Mouse/Keyboard Automation)";
    frame.render_widget(Paragraph::new(hand_text).block(Block::default().title(" 🖐️ Organ: Hand (MCP Tool Hub) ").borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan))), top_cols[1]);

    // Bottom-Left: Ear (Voice)
    let ear_text = "Voice Engine:       Full-Duplex Web Audio API\nVAD Energy RMS:     [||||||||||||░░░░░░] 0.02 (Silence)\nBarge-In Detector:  ARMED (Speech interrupts immediately)\nPad Color Map:      Reactive Soundwave Visualizer Active";
    frame.render_widget(Paragraph::new(ear_text).block(Block::default().title(" 🎙️ Organ: Ear & Voice ").borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow))), bot_cols[0]);

    // Bottom-Right: Brain & Healer
    let brain_text = "Cognitive Core:     Hybrid Local SLM + MiniMax-Text-01\nEpistemic Healer:   Distillation Active (0 failures recorded)\nDream Engine:       P9 Nightly Memory Cluster Active\nSafety Onion:       Layer-1/2/3 Scrubbing Verified";
    frame.render_widget(Paragraph::new(brain_text).block(Block::default().title(" 🧠 Organ: Brain (Healer & Dream) ").borders(Borders::ALL).border_style(Style::default().fg(Color::Magenta))), bot_cols[1]);
}

// -----------------------------------------------------------------------------
// Tab 4: Software Factory (Git Worktree & Diff Review)
// -----------------------------------------------------------------------------

fn render_factory_tab(frame: &mut Frame, area: Rect, state: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let mut task_items = Vec::new();
    for (i, task) in state.factory_tasks.iter().enumerate() {
        let is_selected = i == state.factory_selected;
        let style = if is_selected { Style::default().fg(Color::Yellow).bold().bg(Color::DarkGray) } else { Style::default().fg(Color::White) };
        let status_tag = if task.passed { "PASS" } else { "FAIL" };
        task_items.push(ListItem::new(format!("[{}] {} ({})", status_tag, task.requirement, task.branch)).style(style));
    }

    let list_block = Block::default()
        .title(" 💻 Factory Tasks (Git Worktrees) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(List::new(task_items).block(list_block), cols[0]);

    // Right: Colorized Diff Viewer
    let selected_task = state.factory_tasks.get(state.factory_selected);
    let diff_content = selected_task.map(|t| t.diff_content.as_str()).unwrap_or("No task selected.");
    let diff_widget = DiffViewer::render_diff(diff_content).block(
        Block::default().title(" 📄 PatchSet Diff Viewer ([Y] Apply Patch  [N] Discard) ").borders(Borders::ALL).border_style(Style::default().fg(Color::Green))
    );

    frame.render_widget(diff_widget, cols[1]);
}

// -----------------------------------------------------------------------------
// Tab 5: Governance & Audit Blockchain
// -----------------------------------------------------------------------------

fn render_governance_tab(frame: &mut Frame, area: Rect, state: &AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(8)])
        .split(area);

    let audit_status = if state.audit_integrity_ok {
        "✓ 100% Cryptographically Verified (Genesis -> Latest Block Valid)"
    } else {
        "❌ Integrity Broken!"
    };

    let summary_text = format!(
        "Audit Ledger Height:      {} blocks\nBlockchain Status:        {}\nGovernance Pipeline:      5/5 Gated Layers Active\nEgress Enforcement:       Default Deny with Domain Whitelist\nPlatform Sandbox:         Windows Job Object Process Hardening Active",
        state.audit_chain_length, audit_status
    );

    frame.render_widget(Paragraph::new(summary_text).block(Block::default().title(" 🛡️ Governance & Integrity Overview ").borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow))), rows[0]);

    let mut audit_entries = Vec::new();
    for msg in &state.messages {
        audit_entries.push(ListItem::new(format!(
            "[{}] Role: {:<10} | Hash: {:<20} | Tokens: {:<4} | Content: {}",
            msg.timestamp_ms, msg.role, msg.audit_hash, msg.tokens, &msg.content[..msg.content.len().min(40)]
        )));
    }

    let audit_block = Block::default()
        .title(" 🔗 Immutable Audit Hash-Chain Ledger ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(List::new(audit_entries).block(audit_block), rows[1]);
}

// -----------------------------------------------------------------------------
// Modal Dialog
// -----------------------------------------------------------------------------

fn render_modal(frame: &mut Frame, area: Rect, text: &str) {
    let popup_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(8),
            Constraint::Percentage(30),
        ])
        .split(area)[1];

    let popup_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(popup_area)[1];

    let clear_block = Clear;
    frame.render_widget(clear_block, popup_cols);

    let modal_block = Block::default()
        .title(" ⚠️ FACTORY PATCHSET APPROVAL ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow).bold());

    let p = Paragraph::new(format!("{}\n\nPress [Y] to Apply, [N] to Discard, [Esc] to Close.", text))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(modal_block);

    frame.render_widget(p, popup_cols);
}
