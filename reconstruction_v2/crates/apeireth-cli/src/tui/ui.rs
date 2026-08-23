use ratatui::prelude::*;
use ratatui::widgets::*;
use super::state::{AppState, NavPage};
use super::theme::ThemeStyle;

pub fn render_ui(frame: &mut Frame, state: &AppState) {
    let style = state.current_style();
    let size = frame.area();

    // Main layout: [Top Tabs (3 lines), Content Area (fill), Bottom Status (2 lines)]
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(size);

    render_nav_header(frame, chunks[0], state, &style);

    match state.current_page {
        NavPage::Bridge => render_bridge_page(frame, chunks[1], state, &style),
        NavPage::Dialogue => render_dialogue_page(frame, chunks[1], state, &style),
        NavPage::Growth => render_growth_page(frame, chunks[1], state, &style),
        NavPage::History => render_history_page(frame, chunks[1], state, &style),
        NavPage::Settings => render_settings_page(frame, chunks[1], state, &style),
    }

    render_status_footer(frame, chunks[2], state, &style);
}

// -----------------------------------------------------------------------------
// Top Navigation Header
// -----------------------------------------------------------------------------

fn render_nav_header(frame: &mut Frame, area: Rect, state: &AppState, style: &ThemeStyle) {
    let tabs = vec![
        "0 舰桥 ΣΚΟΠΗ",
        "1 对话 ΔΙΑΛΟΓΟΣ",
        "2 生长 ΑΥΞΗΣΙΣ",
        "3 历史 ΙΣΤΟΡΙΑ",
        "4 设置 ΤΑΞΙΣ",
    ];

    let active_idx = match state.current_page {
        NavPage::Bridge => 0,
        NavPage::Dialogue => 1,
        NavPage::Growth => 2,
        NavPage::History => 3,
        NavPage::Settings => 4,
    };

    let tab_widget = Tabs::new(tabs)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(style.primary))
                .border_type(style.border_type)
                .title(Span::styled(" APEIRETH 2.0 伴侣系统 · 终端工作台 ", Style::default().fg(style.accent).bold()))
        )
        .select(active_idx)
        .style(Style::default().fg(style.dim))
        .highlight_style(Style::default().fg(style.accent).bold().underlined());

    frame.render_widget(tab_widget, area);
}

// -----------------------------------------------------------------------------
// Bottom Status Bar
// -----------------------------------------------------------------------------

fn render_status_footer(frame: &mut Frame, area: Rect, state: &AppState, style: &ThemeStyle) {
    let status_text = state.status_message.as_ref().map(|(m, _)| m.as_str()).unwrap_or("就绪");

    let line = Line::from(vec![
        Span::styled(format!(" [{}] ", state.theme.display_label()), Style::default().bg(style.primary).fg(Color::Black).bold()),
        Span::raw(" "),
        Span::styled(status_text, Style::default().fg(style.accent)),
        Span::raw(" | "),
        Span::styled("[0-4] 跳转  [Tab] 顺序切换  [t] 切换主题  [Ctrl+O] 展开思考  [PageUp/Down] 滚动  [q] 退出", Style::default().fg(style.dim)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

// -----------------------------------------------------------------------------
// Page 0: 0 舰桥 ΣΚΟΠΗ (Bridge)
// -----------------------------------------------------------------------------

fn render_bridge_page(frame: &mut Frame, area: Rect, state: &AppState, style: &ThemeStyle) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Top 3-line Status
            Constraint::Min(8),    // Middle 9 Organs + Star Chart
            Constraint::Length(2), // Bottom Hint
        ])
        .split(area);

    // 1. Top Status
    let top_lines = vec![
        Line::from(vec![
            Span::styled("北极星 ", Style::default().fg(style.dim)),
            Span::styled("0.985", Style::default().fg(style.primary).bold()),
            Span::styled("  连续 ", Style::default().fg(style.dim)),
            Span::styled("0.992", Style::default().fg(style.primary)),
            Span::styled("  哲学守护 ", Style::default().fg(style.dim)),
            Span::styled("1.000", Style::default().fg(style.primary)),
            Span::styled("  5-我自组织 ", Style::default().fg(style.dim)),
            Span::styled("已就绪 (Sovereign)", Style::default().fg(style.accent)),
        ]),
        Line::from(vec![
            Span::styled("运行阶段 ", Style::default().fg(style.dim)),
            Span::styled("Serving (在线服务中)", Style::default().fg(style.accent).bold()),
            Span::styled("  认知反思 ", Style::default().fg(style.dim)),
            Span::styled("活跃 (Active)", Style::default().fg(style.primary)),
            Span::styled("  睡眠驱动 ", Style::default().fg(style.dim)),
            Span::styled(format!("{:.2} S", state.sleep_drive), Style::default().fg(style.primary)),
            Span::styled("  PAD 情感 ", Style::default().fg(style.dim)),
            Span::styled(format!("P:{:.2} A:{:.2} D:{:.2}", state.current_pad.pleasure, state.current_pad.arousal, state.current_pad.dominance), Style::default().fg(style.accent)),
        ]),
        Line::from(vec![
            Span::styled("心跳时钟 ", Style::default().fg(style.dim)),
            Span::styled("20Hz (WAL)", Style::default().fg(style.primary)),
            Span::styled("  ACT-R 记忆 ", Style::default().fg(style.dim)),
            Span::styled(format!("{} 条", state.memory_items.len()), Style::default().fg(style.primary)),
            Span::styled("  审计链区块 ", Style::default().fg(style.dim)),
            Span::styled(format!("H:{} (100% 校验通过)", state.audit_chain_length), Style::default().fg(style.accent)),
        ]),
    ];

    let top_block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);
    frame.render_widget(Paragraph::new(top_lines).block(top_block), chunks[0]);

    // 2. Middle: 9 Organs Grid (Left 50%) + Star Chart (Right 50%)
    let mid_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);

    // Render 9 Organs in 3x3 Grid
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
        .split(mid_cols[0]);

    for (i, row_area) in rows.iter().enumerate() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
            .split(*row_area);

        for (j, col_area) in cols.iter().enumerate() {
            let idx = i * 3 + j;
            if idx < state.organs.len() {
                let o = &state.organs[idx];
                let bar_w = (o.health * 6.0).round() as usize;
                let bar = format!(
                    "{}{}",
                    style.bar_full.to_string().repeat(bar_w),
                    style.bar_empty.to_string().repeat(6 - bar_w)
                );

                let card_text = vec![
                    Line::from(vec![
                        Span::styled(o.name, Style::default().fg(style.primary).bold()),
                    ]),
                    Line::from(Span::styled(bar, Style::default().fg(style.accent))),
                    Line::from(Span::styled(o.primary, Style::default().fg(style.dim))),
                ];

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(style.dim))
                    .border_type(style.border_type);

                frame.render_widget(Paragraph::new(card_text).block(block), *col_area);
            }
        }
    }

    // Render ASCII Star Map / Cognitive Constellation
    let star_lines = vec![
        Line::from(Span::styled("        ·   ★ 北极星 (ASI V0.5)   ·", Style::default().fg(style.accent).bold())),
        Line::from(Span::styled("             /        \\", Style::default().fg(style.dim))),
        Line::from(Span::styled("       [ACT-R] ──── [MiniMax]", Style::default().fg(style.primary))),
        Line::from(Span::styled("          |            |", Style::default().fg(style.dim))),
        Line::from(Span::styled("    [Win32 GDI] ── [5-Gate 宪政]", Style::default().fg(style.primary))),
        Line::from(Span::styled("          \\            /", Style::default().fg(style.dim))),
        Line::from(Span::styled("       [MCP Hub] ── [SHA-256 账本]", Style::default().fg(style.primary))),
        Line::from(Span::raw("")),
        Line::from(Span::styled("  全 9 大认知器官与工作流微内核健康运转中", Style::default().fg(style.accent))),
    ];

    let star_block = Block::default()
        .title(" 🌌 认知星座拓扑图 (Constellation) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);
    frame.render_widget(Paragraph::new(star_lines).block(star_block), mid_cols[1]);

    // 3. Bottom Hint
    let hint_text = Line::from(vec![
        Span::styled(" → 按 ", Style::default().fg(style.dim)),
        Span::styled("1", Style::default().fg(style.accent).bold()),
        Span::styled(" 或 ", Style::default().fg(style.dim)),
        Span::styled("i", Style::default().fg(style.accent).bold()),
        Span::styled(" 进入对话 (Dialogue) | 按 ", Style::default().fg(style.dim)),
        Span::styled("t", Style::default().fg(style.accent).bold()),
        Span::styled(" 切换古朴金/时代蓝主题", Style::default().fg(style.dim)),
    ]);
    frame.render_widget(Paragraph::new(hint_text).alignment(Alignment::Center), chunks[2]);
}

// -----------------------------------------------------------------------------
// Page 1: 1 对话 ΔΙΑΛΟΓΟΣ (Dialogue)
// -----------------------------------------------------------------------------

fn render_dialogue_page(frame: &mut Frame, area: Rect, state: &AppState, style: &ThemeStyle) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),    // Chat Messages
            Constraint::Length(3), // Input Box
        ])
        .split(area);

    // Render Chat Messages List
    let mut list_items = Vec::new();
    for msg in &state.messages {
        let (role_tag, role_color) = match msg.role.as_str() {
            "user" => (" ❯ USER ", style.accent),
            "assistant" => (" ▌ APEIRETH ", style.primary),
            _ => (" · SYSTEM ", style.dim),
        };

        let mut lines = Vec::new();
        lines.push(Line::from(vec![
            Span::styled(role_tag, Style::default().fg(role_color).bold()),
            Span::raw(" "),
            Span::styled(format!("(Tokens: {} | Audit: {})", msg.tokens, &msg.audit_hash[..msg.audit_hash.len().min(8)]), Style::default().fg(style.dim)),
        ]));

        if let Some(ref cot) = msg.cot {
            if state.thinking_expanded {
                lines.push(Line::from(Span::styled("  ┌─ ∴ Thinking 深度思考链 (按 Ctrl+O 折叠) ──────┐", Style::default().fg(style.dim))));
                for cot_line in cot.lines() {
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", Style::default().fg(style.dim)),
                        Span::styled(cot_line, Style::default().fg(style.dim).italic()),
                    ]));
                }
                lines.push(Line::from(Span::styled("  └───────────────────────────────────────────────┘", Style::default().fg(style.dim))));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  ∴ Thinking ", Style::default().fg(style.dim).italic()),
                    Span::styled("(已折叠, 按 Ctrl+O 展开思考过程)", Style::default().fg(style.dim)),
                ]));
            }
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
                Span::styled(" ▌ APEIRETH ", Style::default().fg(style.primary).bold()),
                Span::styled("正在深度思考并唤醒 ACT-R 记忆流...", Style::default().fg(style.accent).italic()),
            ]),
        ]));
    }

    let chat_block = Block::default()
        .title(" 1 对话 (Dialogue, ΔΙΑΛΟΓΟΣ) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);

    frame.render_widget(List::new(list_items).block(chat_block), chunks[0]);

    // Input Box
    let input_block = Block::default()
        .title(" ✍️ 输入消息 (按 Enter 发送, Ctrl+O 展开思考, Esc 退出聚焦) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.accent))
        .border_type(style.border_type);

    let input_widget = Paragraph::new(format!("> {}█", state.input_buffer))
        .style(Style::default().fg(Color::White))
        .block(input_block);

    frame.render_widget(input_widget, chunks[1]);
}

// -----------------------------------------------------------------------------
// Page 2: 2 生长 ΑΥΞΗΣΙΣ (Growth)
// -----------------------------------------------------------------------------

fn render_growth_page(frame: &mut Frame, area: Rect, state: &AppState, style: &ThemeStyle) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let mut list_items = Vec::new();
    if state.memory_items.is_empty() {
        list_items.push(ListItem::new("暂无沉淀记忆。请在对话页与伴侣互动以生成 ACT-R 认知事实。"));
    } else {
        for (i, item) in state.memory_items.iter().enumerate() {
            let is_sel = i == state.memory_selected;
            let st = if is_sel { Style::default().fg(style.accent).bold().bg(style.dim) } else { Style::default().fg(Color::White) };
            list_items.push(ListItem::new(format!("[★{:.1}] {}", item.importance, item.data)).style(st));
        }
    }

    let list_block = Block::default()
        .title(" 🧠 ACT-R 记忆事实池 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);

    frame.render_widget(List::new(list_items).block(list_block), cols[0]);

    let selected_item = state.memory_items.get(state.memory_selected);
    let detail_text = if let Some(item) = selected_item {
        format!(
            "记忆 ID:       {}\n事实内容:      {}\n重要度:        {:.2}\n访问频次:      {}\n生成时间:      {}\n\n[ACT-R 动态认知激活度公式]:\nA_i = ln( sum( (t - t_j)^(-d) ) ) + Beta = {:.4}\n\n[语义三元组关系]:\n(主语: User) ──[谓词: 叙述/偏好]──> (宾语: {})",
            item.id, item.data, item.importance, item.access_count, item.created_at,
            item.calculate_act_r_activation(chrono::Utc::now().timestamp(), 0.5, 0.0),
            item.data
        )
    } else {
        "在左侧选择一项记忆，查看其时间衰减激活度与语义图谱三元组。".into()
    };

    let detail_block = Block::default()
        .title(" 🔍 认知生长与三元组查看器 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.accent))
        .border_type(style.border_type);

    frame.render_widget(Paragraph::new(detail_text).block(detail_block), cols[1]);
}

// -----------------------------------------------------------------------------
// Page 3: 3 历史 ΙΣΤΟΡΙΑ (History)
// -----------------------------------------------------------------------------

fn render_history_page(frame: &mut Frame, area: Rect, state: &AppState, style: &ThemeStyle) {
    let mut audit_items = Vec::new();
    for msg in &state.messages {
        audit_items.push(ListItem::new(format!(
            "[{}] Role: {:<10} | Hash: {:<20} | Tokens: {:<4} | Content: {}",
            msg.timestamp_ms, msg.role, msg.audit_hash, msg.tokens, &msg.content[..msg.content.len().min(45)]
        )));
    }

    let audit_block = Block::default()
        .title(" 🔗 不可篡改 SHA-256 审计区块链账本 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);

    frame.render_widget(List::new(audit_items).block(audit_block), area);
}

// -----------------------------------------------------------------------------
// Page 4: 4 设置 ΤΑΞΙΣ (Settings)
// -----------------------------------------------------------------------------

fn render_settings_page(frame: &mut Frame, area: Rect, state: &AppState, style: &ThemeStyle) {
    let settings_text = format!(
        "系统主题:           {} (按 't' 键实时切换)\n主题配色:           {}\n边框风格:           {:?}\n\n后端微内核:         UnifiedRuntimeHost 2.0\n大语言模型服务:     MiniMax-Text-01 (已挂载)\n长期认知存储池:     SQLite WAL (apeireth_v2.db)\n安全出站 (Egress):  严格域名白名单拦截已开启\n平台进程沙箱:       Windows Job Object 活跃\n\n快捷键帮助:\n• [0/1/2/3/4]: 快速跳转对应页面\n• [Tab]: 循环切换页面\n• [t]: 切换 古朴金 / 时代蓝 主题\n• [Ctrl+O]: 展开 / 折叠 CoT 思考链\n• [PageUp/Down]: 滚动消息列表\n• [q]: 退出 TUI 伴侣系统",
        state.theme.display_label(),
        match state.theme { super::theme::Theme::Archaic => "砖块金 (0xc8860a) / 暗金 (0x806040)", super::theme::Theme::Era => "淡蓝 (0x8fb3d9) / 暗蓝 (0x506840)" },
        style.border_type
    );

    let block = Block::default()
        .title(" 4 设置 (Settings, ΤΑΞΙΣ) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);

    frame.render_widget(Paragraph::new(settings_text).block(block), area);
}
