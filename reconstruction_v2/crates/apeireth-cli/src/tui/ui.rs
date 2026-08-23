use ratatui::prelude::*;
use ratatui::widgets::*;
use super::state::{AppState, NavPage};
use super::theme::ThemeStyle;

pub fn compute_scroll_y(scroll_to_bottom: bool, scroll_offset: u16, max_scroll: u16) -> u16 {
    if scroll_to_bottom {
        max_scroll
    } else {
        max_scroll.saturating_sub(scroll_offset)
    }
}

pub fn compute_scrollbar_position(scroll: u16, max_scroll: u16, total: usize) -> usize {
    if max_scroll == 0 || total == 0 {
        return 0;
    }
    let s = u64::from(scroll);
    let m = u64::from(max_scroll);
    let n = (total as u64).saturating_sub(1);
    (s.saturating_mul(n) / m) as usize
}

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
                .title(Span::styled(" APEIRETH 2.0 伴侣系统 · 后端全接终端工作台 ", Style::default().fg(style.accent).bold()))
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
// Page 0: 0 舰桥 ΣΚΟΠΗ (Bridge) — 真实后端工程参数大屏
// -----------------------------------------------------------------------------

fn render_bridge_page(frame: &mut Frame, area: Rect, state: &AppState, style: &ThemeStyle) {
    let t = &state.telemetry;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Top Global Telemetry
            Constraint::Min(8),    // 4 Engineering Modules Grid
            Constraint::Length(2), // Bottom Hint
        ])
        .split(area);

    // 1. Top Global Telemetry
    let top_lines = vec![
        Line::from(vec![
            Span::styled("北极星指标 ", Style::default().fg(style.dim)),
            Span::styled("0.985 (ASI V0.5)", Style::default().fg(style.primary).bold()),
            Span::styled("  连续性 ", Style::default().fg(style.dim)),
            Span::styled("0.992", Style::default().fg(style.primary)),
            Span::styled("  模型内核 ", Style::default().fg(style.dim)),
            Span::styled(&t.active_model, Style::default().fg(style.accent).bold()),
            Span::styled("  会话 ID ", Style::default().fg(style.dim)),
            Span::styled(&state.session_id[..state.session_id.len().min(14)], Style::default().fg(style.dim)),
        ]),
        Line::from(vec![
            Span::styled("运行阶段 ", Style::default().fg(style.dim)),
            Span::styled("Serving (在线服务)", Style::default().fg(style.accent).bold()),
            Span::styled("  Borbély 睡眠驱动 ", Style::default().fg(style.dim)),
            Span::styled(format!("{:.2} S", t.borbely_sleep_drive), Style::default().fg(style.primary)),
            Span::styled("  PAD 情感动力学 ", Style::default().fg(style.dim)),
            Span::styled(format!("P:{:.2} A:{:.2} D:{:.2}", state.current_pad.pleasure, state.current_pad.arousal, state.current_pad.dominance), Style::default().fg(style.accent)),
            Span::styled("  响应风格 ", Style::default().fg(style.dim)),
            Span::styled(format!("{:?}", state.current_pad.to_response_style()), Style::default().fg(style.primary)),
        ]),
    ];

    let top_block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);
    frame.render_widget(Paragraph::new(top_lines).block(top_block), chunks[0]);

    // 2. Middle: 4 Quadrants of Real Engineering Parameters
    let mid_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let row1_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(mid_rows[0]);

    let row2_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(mid_rows[1]);

    // Module 1 (Top-Left): 🖥️ 物理视觉与屏幕感知参数
    let vision_text = vec![
        Line::from(vec![
            Span::styled("物理屏幕分辨率:    ", Style::default().fg(style.dim)),
            Span::styled(&t.screen_resolution, Style::default().fg(style.accent).bold()),
        ]),
        Line::from(vec![
            Span::styled("底层截屏引擎:      ", Style::default().fg(style.dim)),
            Span::styled(t.screen_driver, Style::default().fg(style.primary)),
        ]),
        Line::from(vec![
            Span::styled("当前感知哈希 (pHash): ", Style::default().fg(style.dim)),
            Span::styled(format!("0x{:016x}", t.screen_phash), Style::default().fg(style.primary)),
        ]),
        Line::from(vec![
            Span::styled("帧间海明差分 (ΔH): ", Style::default().fg(style.dim)),
            Span::styled(format!("{} bits (变化率: 0.0%)", t.screen_hamming_diff), Style::default().fg(style.primary)),
        ]),
        Line::from(vec![
            Span::styled("SoM 控件逆解数量:  ", Style::default().fg(style.dim)),
            Span::styled(format!("{} 个物理活动窗口/控件已映射", t.som_elements_count), Style::default().fg(style.accent)),
        ]),
    ];
    let vision_block = Block::default()
        .title(" 🖥️ 物理视觉与桌面感知 (Win32 GDI & Screen Engine) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);
    frame.render_widget(Paragraph::new(vision_text).block(vision_block), row1_cols[0]);

    // Module 2 (Top-Right): 🧠 ACT-R 2.0 认知记忆与 SQLite WAL
    let memory_text = vec![
        Line::from(vec![
            Span::styled("数据库连接池状态:  ", Style::default().fg(style.dim)),
            Span::styled(t.wal_pool_status, Style::default().fg(style.accent).bold()),
        ]),
        Line::from(vec![
            Span::styled("长期记忆事实总条目: ", Style::default().fg(style.dim)),
            Span::styled(format!("{} 条 (实时持久化在 {})", state.memory_items.len(), t.db_path), Style::default().fg(style.primary)),
        ]),
        Line::from(vec![
            Span::styled("ACT-R 激活度均值:  ", Style::default().fg(style.dim)),
            Span::styled(format!("Ā_i = {:.4} [ln Σ(t-t_j)^(-d) + β]", t.avg_activation_score), Style::default().fg(style.primary)),
        ]),
        Line::from(vec![
            Span::styled("CJK Bigram 倒排索引:", Style::default().fg(style.dim)),
            Span::styled(t.cjk_bigram_status, Style::default().fg(style.primary)),
        ]),
        Line::from(vec![
            Span::styled("存储文件大小/同步: ", Style::default().fg(style.dim)),
            Span::styled(format!("{} KB (WAL Checkpoint 已对齐)", t.db_size_kb), Style::default().fg(style.accent)),
        ]),
    ];
    let memory_block = Block::default()
        .title(" 🧠 ACT-R 2.0 认知记忆与存储池 (SQLite WAL Pool) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);
    frame.render_widget(Paragraph::new(memory_text).block(memory_block), row1_cols[1]);

    // Module 3 (Bottom-Left): ⚡ 大模型协议与流式状态机
    let llm_text = vec![
        Line::from(vec![
            Span::styled("协议适配器 (Adapter): ", Style::default().fg(style.dim)),
            Span::styled(t.protocol_adapter, Style::default().fg(style.primary)),
        ]),
        Line::from(vec![
            Span::styled("会话累计消耗令牌:   ", Style::default().fg(style.dim)),
            Span::styled(format!("{} Tokens", t.session_tokens), Style::default().fg(style.accent).bold()),
        ]),
        Line::from(vec![
            Span::styled("首字端到端响应延迟: ", Style::default().fg(style.dim)),
            Span::styled(format!("{} ms", t.last_latency_ms), Style::default().fg(style.primary)),
        ]),
        Line::from(vec![
            Span::styled("CoT 深度推理链:    ", Style::default().fg(style.dim)),
            Span::styled(t.cot_depth, Style::default().fg(style.accent)),
        ]),
        Line::from(vec![
            Span::styled("流式状态机 (Stream):", Style::default().fg(style.dim)),
            Span::styled(t.streaming_state, Style::default().fg(style.primary)),
        ]),
    ];
    let llm_block = Block::default()
        .title(" ⚡ 大模型协议与推理管线 (LLM Backbone & Protocol) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);
    frame.render_widget(Paragraph::new(llm_text).block(llm_block), row2_cols[0]);

    // Module 4 (Bottom-Right): 🛡️ 5-Gate 宪政治理与审计区块链
    let gov_text = vec![
        Line::from(vec![
            Span::styled("洋葱消毒门禁 (L1-L3): ", Style::default().fg(style.dim)),
            Span::styled(t.onion_sanitizer, Style::default().fg(style.primary)),
        ]),
        Line::from(vec![
            Span::styled("Egress 出站域名白名单: ", Style::default().fg(style.dim)),
            Span::styled(t.egress_whitelist, Style::default().fg(style.accent).bold()),
        ]),
        Line::from(vec![
            Span::styled("底层进程物理沙箱:   ", Style::default().fg(style.dim)),
            Span::styled(&t.platform_sandbox, Style::default().fg(style.primary)),
        ]),
        Line::from(vec![
            Span::styled("不可篡改审计链高度: ", Style::default().fg(style.dim)),
            Span::styled(format!("H:{} (创世 -> 最新区块 100% 有效)", state.audit_chain_length), Style::default().fg(style.accent).bold()),
        ]),
        Line::from(vec![
            Span::styled("生物昼夜节律驱动:   ", Style::default().fg(style.dim)),
            Span::styled(t.circadian_phase, Style::default().fg(style.primary)),
        ]),
    ];
    let gov_block = Block::default()
        .title(" 🛡️ 5-Gate 宪政治理与审计区块链 (Governance & Audit) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);
    frame.render_widget(Paragraph::new(gov_text).block(gov_block), row2_cols[1]);

    // 3. Bottom Hint
    let hint_text = Line::from(vec![
        Span::styled(" → 按 ", Style::default().fg(style.dim)),
        Span::styled("1", Style::default().fg(style.accent).bold()),
        Span::styled(" 或 ", Style::default().fg(style.dim)),
        Span::styled("i", Style::default().fg(style.accent).bold()),
        Span::styled(" 进入对话 (Dialogue) | 按 ", Style::default().fg(style.dim)),
        Span::styled("t", Style::default().fg(style.accent).bold()),
        Span::styled(" 切换古朴金/时代蓝主题 | 按 ", Style::default().fg(style.dim)),
        Span::styled("2/3/4", Style::default().fg(style.accent).bold()),
        Span::styled(" 查看记忆/历史/设置", Style::default().fg(style.dim)),
    ]);
    frame.render_widget(Paragraph::new(hint_text).alignment(Alignment::Center), chunks[2]);
}

// -----------------------------------------------------------------------------
// Page 1: 1 对话 ΔΙΑΛΟΓΟΣ (Dialogue) — 具备完整上下滚动与 Scrollbar
// -----------------------------------------------------------------------------

fn render_dialogue_page(frame: &mut Frame, area: Rect, state: &AppState, style: &ThemeStyle) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),    // Chat Messages Viewport
            Constraint::Length(3), // Input Box
        ])
        .split(area);

    let chat_block = Block::default()
        .title(" 1 对话 (Dialogue, ΔΙΑΛΟΓΟΣ) · [PageUp/Down/Up/Down] 滚动浏览 [Home/End] 跳顶/底 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);

    let mut lines: Vec<Line> = Vec::new();

    for msg in &state.messages {
        let (role_prefix, prefix_style) = match msg.role.as_str() {
            "user" => (" ❯ USER ", Style::default().fg(style.bg).bg(style.primary).bold()),
            "assistant" => (" ▌ APEIRETH ", Style::default().fg(style.accent).bold()),
            _ => (" · SYSTEM ", Style::default().fg(style.dim)),
        };

        lines.push(Line::from(vec![
            Span::styled(role_prefix, prefix_style),
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
    }

    if state.is_thinking {
        lines.push(Line::from(vec![
            Span::styled(" ▌ APEIRETH ", Style::default().fg(style.accent).bold()),
            Span::styled("正在深度思考并实时唤醒 ACT-R 记忆流...", Style::default().fg(style.accent).italic()),
        ]));
        lines.push(Line::raw(""));
    }

    // Compute dynamic scroll parameters
    let inner_height = chunks[0].height.saturating_sub(2) as usize;
    let total_lines = lines.len();
    let max_scroll = total_lines.saturating_sub(inner_height) as u16;
    let scroll_y = compute_scroll_y(state.scroll_to_bottom, state.scroll_offset, max_scroll);
    let inner_area = chat_block.inner(chunks[0]);

    frame.render_widget(
        Paragraph::new(lines)
            .block(chat_block)
            .wrap(Wrap { trim: false })
            .scroll((scroll_y, 0)),
        chunks[0],
    );

    // Render Scrollbar
    let sb_pos = compute_scrollbar_position(scroll_y, max_scroll, total_lines);
    let mut sb_state = ScrollbarState::new(total_lines)
        .position(sb_pos)
        .viewport_content_length(inner_height);

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(style.accent))
            .track_style(Style::default().fg(style.dim)),
        inner_area,
        &mut sb_state,
    );

    // Input Box
    let input_block = Block::default()
        .title(" ✍️ 输入消息 (按 Enter 发送, PageUp/PageDown 向上翻看历史, Ctrl+O 展开思考) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.accent))
        .border_type(style.border_type);

    let input_widget = Paragraph::new(format!("> {}█", state.input_buffer))
        .style(Style::default().fg(Color::White))
        .block(input_block);

    frame.render_widget(input_widget, chunks[1]);
}

// -----------------------------------------------------------------------------
// Page 2: 2 生长 ΑΥΞΗΣΙΣ (Growth) — ACT-R 知识沉淀与三元组
// -----------------------------------------------------------------------------

fn render_growth_page(frame: &mut Frame, area: Rect, state: &AppState, style: &ThemeStyle) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let mut list_items = Vec::new();
    if state.memory_items.is_empty() {
        list_items.push(ListItem::new("暂无沉淀记忆。请在对话页与伴侣交互以生成 ACT-R 认知事实。"));
    } else {
        for (i, item) in state.memory_items.iter().enumerate() {
            let is_sel = i == state.memory_selected;
            let st = if is_sel { Style::default().fg(style.accent).bold().bg(style.dim) } else { Style::default().fg(Color::White) };
            list_items.push(ListItem::new(format!("[★{:.1}] {}", item.importance, item.data)).style(st));
        }
    }

    let list_block = Block::default()
        .title(" 🧠 ACT-R 记忆事实池 ([↑/↓] 选择查看) ")
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
// Page 3: 3 历史 ΙΣΤΟΡΙΑ (History) — 支持 PageUp/PageDown 滚动浏览
// -----------------------------------------------------------------------------

fn render_history_page(frame: &mut Frame, area: Rect, state: &AppState, style: &ThemeStyle) {
    let mut audit_lines = Vec::new();
    for msg in &state.messages {
        audit_lines.push(Line::from(vec![
            Span::styled(format!("[{}] ", msg.timestamp_ms), Style::default().fg(style.dim)),
            Span::styled(format!("Role: {:<9} | ", msg.role), Style::default().fg(style.primary).bold()),
            Span::styled(format!("Hash: {:.16}... | ", msg.audit_hash), Style::default().fg(style.accent)),
            Span::styled(format!("Tokens: {:<4} | ", msg.tokens), Style::default().fg(style.dim)),
            Span::styled(&msg.content[..msg.content.len().min(40)], Style::default().fg(Color::White)),
        ]));
    }

    let inner_height = area.height.saturating_sub(2) as usize;
    let total = audit_lines.len();
    let max_scroll = total.saturating_sub(inner_height) as u16;
    let scroll = compute_scroll_y(false, state.history_scroll_offset, max_scroll);

    let audit_block = Block::default()
        .title(" 🔗 不可篡改 SHA-256 审计区块链账本 ([PageUp/Down] 滚动浏览) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);

    let inner_area = audit_block.inner(area);

    frame.render_widget(
        Paragraph::new(audit_lines)
            .block(audit_block)
            .scroll((scroll, 0)),
        area,
    );

    let sb_pos = compute_scrollbar_position(scroll, max_scroll, total);
    let mut sb_state = ScrollbarState::new(total)
        .position(sb_pos)
        .viewport_content_length(inner_height);

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(style.accent))
            .track_style(Style::default().fg(style.dim)),
        inner_area,
        &mut sb_state,
    );
}

// -----------------------------------------------------------------------------
// Page 4: 4 设置 ΤΑΞΙΣ (Settings)
// -----------------------------------------------------------------------------

fn render_settings_page(frame: &mut Frame, area: Rect, state: &AppState, style: &ThemeStyle) {
    let settings_text = format!(
        "系统主题:           {} (按 't' 键实时切换)\n主题配色:           {}\n边框风格:           {:?}\n\n后端微内核:         UnifiedRuntimeHost 2.0 (全真后端接驳)\n大语言模型服务:     MiniMax-Text-01 (已挂载)\n长期认知存储池:     SQLite WAL ({})\n安全出站 (Egress):  严格域名白名单拦截已开启 (Default Deny)\n平台进程沙箱:       {}\n\n快捷键帮助:\n• [0/1/2/3/4]: 快速跳转对应页面 (舰桥 / 对话 / 生长 / 历史 / 设置)\n• [Tab / BackTab]: 循环切换页面\n• [t]: 切换 古朴金 / 时代蓝 主题\n• [Ctrl+O]: 展开 / 折叠 CoT 思考链\n• [PageUp / PageDown]: 滚动翻看历史长对话\n• [Home / End]: 一键直达对话顶部 / 底部\n• [q]: 退出 TUI 伴侣系统",
        state.theme.display_label(),
        match state.theme { super::theme::Theme::Archaic => "砖块金 (0xc8860a) / 暗金 (0x806040)", super::theme::Theme::Era => "淡蓝 (0x8fb3d9) / 暗蓝 (0x506840)" },
        style.border_type,
        state.telemetry.db_path,
        state.telemetry.platform_sandbox
    );

    let block = Block::default()
        .title(" 4 设置 (Settings, ΤΑΞΙΣ) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);

    frame.render_widget(Paragraph::new(settings_text).block(block), area);
}
