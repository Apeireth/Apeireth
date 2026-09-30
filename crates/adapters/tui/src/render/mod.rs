//! 面板布局与合成 (全屏驾驶舱语法: 发光描边 / 扫描线氛围 / 正文可读优先)。
//!
//! 布局 (自上而下):
//! 1. 多会话页签行 (面板 1 的会话切换);
//! 2. 面板 1 主对话区 (左/中) + 面板 3 内部过程频道 (右窄栏, 本批接口桩);
//! 3. 输入行 (命令补全浮层叠在其上方);
//! 4. 面板 2 实时状态条。
//!
//! 面板 5 时间倒带走双 Esc / `/rewind` (接口桩, 见 [`crate::state`])。

pub mod statusbar;

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap};
use ratatui::Frame;

use crate::command::COMMAND_SPECS;
use crate::keys::KEY_TABLE;
use crate::markdown;
use crate::state::{App, ConnectionState, Message, NoteLevel, Overlay};
use crate::theme::CockpitTheme;

/// 合成一整帧 (无条件可渲染: 连接失败也第一帧上屏, 不白屏)。
pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let theme = CockpitTheme::new(app.motion);
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.palette.deep_space)),
        area,
    );

    let root = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(6),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);

    render_tabs(frame, app, &theme, root[0]);
    let body = Layout::horizontal([Constraint::Min(30), Constraint::Length(32)]).split(root[1]);
    render_conversation(frame, app, &theme, body[0]);
    render_channel(frame, app, &theme, body[1]);
    render_composer(frame, app, &theme, root[2]);
    frame.render_widget(
        Paragraph::new(statusbar::status_line(app, &theme))
            .style(Style::default().bg(theme.palette.hull)),
        root[3],
    );
    if app.overlay == Some(Overlay::Help) {
        render_help(frame, &theme, area);
    }
}

fn glow_border(theme: &CockpitTheme) -> Style {
    if theme.motion.glow_pulse_enabled {
        Style::default().fg(theme.palette.glow_primary)
    } else {
        Style::default().fg(theme.palette.panel_edge)
    }
}

fn render_tabs(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let titles: Vec<Line<'_>> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            let marker = if index == app.active { "▸ " } else { "  " };
            Line::from(vec![
                Span::styled(marker, Style::default().fg(palette.glow_primary)),
                Span::styled(tab.title.clone(), Style::default()),
            ])
        })
        .collect();
    let tabs = Tabs::new(titles)
        .select(app.active)
        .style(Style::default().fg(palette.text_dim).bg(palette.hull))
        .highlight_style(
            Style::default()
                .fg(palette.glow_primary)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" ┆ ", Style::default().fg(palette.panel_edge)));
    frame.render_widget(tabs, area);
}

fn render_conversation(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(glow_border(theme))
        .style(Style::default().bg(palette.hull))
        .title(
            Line::from(vec![
                Span::styled(
                    " ▐ 对话 ",
                    Style::default()
                        .fg(palette.glow_secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("· MAIN CHANNEL ▌", Style::default().fg(palette.text_dim)),
            ])
            .alignment(Alignment::Left),
        );
    let tab = &app.tabs[app.active.min(app.tabs.len().saturating_sub(1))];
    let mut lines = Vec::new();
    for message in &tab.messages {
        match message {
            Message::User(text) => lines.push(Line::from(vec![
                Span::styled(
                    "▌YOU ▸ ",
                    Style::default()
                        .fg(palette.glow_secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(text.clone(), Style::default().fg(palette.text_primary)),
            ])),
            Message::Assistant(assistant) => {
                lines.push(Line::from(Span::styled(
                    "▌AI  ▸ ",
                    Style::default()
                        .fg(palette.glow_primary)
                        .add_modifier(Modifier::BOLD),
                )));
                for tool in &assistant.tools {
                    lines.extend(markdown::tool_card_lines(tool, &palette));
                }
                lines.extend(markdown::render_markdown(&assistant.text, &palette));
                if assistant.streaming {
                    lines.push(Line::from(Span::styled(
                        "▍",
                        Style::default().fg(palette.digit),
                    )));
                }
            }
            Message::System(note) => {
                let color = match note.level {
                    NoteLevel::Info => palette.text_dim,
                    NoteLevel::Warn => palette.glow_warn,
                    NoteLevel::Error => palette.glow_danger,
                };
                lines.push(Line::from(Span::styled(
                    format!("· {}", note.text),
                    Style::default().fg(color),
                )));
            }
        }
    }
    // 连接失败即帧: 错误卡直接进主对话区 (不白屏)。
    if let ConnectionState::Failed { detail } = &app.status.connection {
        lines.push(Line::from(Span::styled(
            "┌─ ⚠ 连接失败 ──────────────────────────",
            Style::default().fg(palette.glow_danger),
        )));
        lines.push(Line::from(Span::styled(
            format!("│ 后端 {detail}"),
            Style::default().fg(palette.glow_danger),
        )));
        lines.push(Line::from(Span::styled(
            "└─ 驾驶舱继续运行; 后端恢复后用 /resume 续接",
            Style::default().fg(palette.glow_danger),
        )));
    }
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((tab.scroll, 0));
    frame.render_widget(paragraph, area);
}

fn render_channel(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.tool_card))
        .style(Style::default().bg(palette.hull))
        .title(Line::from(vec![
            Span::styled(
                " 频道 ",
                Style::default()
                    .fg(palette.tool_card)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("· PROCESS FEED", Style::default().fg(palette.text_dim)),
        ]));
    let mut lines = vec![
        Line::from(Span::styled(
            " [未接线] ",
            Style::default()
                .fg(palette.glow_warn)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "本批为接口桩:",
            Style::default().fg(palette.text_dim),
        )),
        Line::from(Span::styled(
            "下批消费 events 面",
            Style::default().fg(palette.text_dim),
        )),
        Line::from(Span::styled(
            "只读投影 (治理判定 /",
            Style::default().fg(palette.text_dim),
        )),
        Line::from(Span::styled(
            "工具调用 / 器官活动)",
            Style::default().fg(palette.text_dim),
        )),
    ];
    // 扫描线氛围: 只铺装饰行, 不覆盖正文 (reduced 档整段关闭)。
    if theme.motion.scanline_enabled {
        let stride = theme.motion.scanline_stride.max(1) as usize;
        let inner_height = area.height.saturating_sub(2) as usize;
        while lines.len() < inner_height {
            if lines.len() % stride == 0 {
                lines.push(Line::from(Span::styled(
                    "░".repeat(area.width.saturating_sub(2) as usize),
                    Style::default().fg(palette.scanline),
                )));
            } else {
                lines.push(Line::from(""));
            }
        }
    }
    let _ = app;
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_composer(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let mode_label = if app.composer.command_mode {
        Span::styled(
            "CMD",
            Style::default()
                .fg(palette.glow_secondary)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("MSG", Style::default().fg(palette.text_dim))
    };
    let line = Line::from(vec![
        Span::styled("▌ ", Style::default().fg(palette.glow_primary)),
        mode_label,
        Span::styled(" ▸ ", Style::default().fg(palette.panel_edge)),
        Span::styled(
            app.composer.text.clone(),
            Style::default().fg(palette.text_primary),
        ),
        Span::styled("▍", Style::default().fg(palette.digit)),
    ]);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(palette.hull)),
        area,
    );

    // 命令补全浮层 (`/` 触发)。
    if app.composer.command_mode {
        let matches = crate::command::complete_command(&app.composer.text);
        if !matches.is_empty() {
            let popup = Rect {
                x: area.x,
                y: area.y.saturating_sub(6),
                width: area.width.min(56),
                height: 6,
            };
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.glow_secondary))
                .style(Style::default().bg(palette.deep_space))
                .title(Span::styled(
                    " 命令补全 ",
                    Style::default().fg(palette.glow_secondary),
                ));
            let rows: Vec<Line<'_>> = matches
                .iter()
                .map(|spec| {
                    Line::from(vec![
                        Span::styled(
                            format!(" {:<10}", spec.name),
                            Style::default()
                                .fg(palette.glow_primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{:<18}", spec.usage),
                            Style::default().fg(palette.text_dim),
                        ),
                        Span::styled(spec.summary, Style::default().fg(palette.text_primary)),
                    ])
                })
                .collect();
            frame.render_widget(Paragraph::new(rows).block(block), popup);
        }
    }
}

fn render_help(frame: &mut Frame<'_>, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let popup = centered_rect(72, 80, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.glow_primary))
        .style(Style::default().bg(palette.deep_space))
        .title(Line::from(vec![
            Span::styled(
                " ▐ 帮助 ",
                Style::default()
                    .fg(palette.glow_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("· KEY MAP ▌", Style::default().fg(palette.text_dim)),
        ]));
    let mut lines = vec![Line::from(Span::styled(
        "── 键位 ──────────────────────────────────",
        Style::default().fg(palette.glow_secondary),
    ))];
    for row in KEY_TABLE {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {:<28}", row.keys),
                Style::default()
                    .fg(palette.glow_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<18}", row.action),
                Style::default().fg(palette.text_primary),
            ),
            Span::styled(row.note, Style::default().fg(palette.text_dim)),
        ]));
    }
    lines.push(Line::from(Span::styled(
        "── 命令 ──────────────────────────────────",
        Style::default().fg(palette.glow_secondary),
    )));
    for spec in COMMAND_SPECS {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {:<22}", spec.usage),
                Style::default()
                    .fg(palette.glow_secondary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(spec.summary, Style::default().fg(palette.text_primary)),
        ]));
    }
    lines.push(Line::from(Span::styled(
        "Esc 关闭浮层",
        Style::default().fg(palette.text_dim),
    )));
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;
    use crate::backend::UsageSnapshot;
    use crate::state::{AssistantMessage, Message, SystemNote, ToolCard, ToolStatus};
    use crate::theme::MotionMode;

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        let area = buffer.area();
        let mut out = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buffer.cell((x, y)) {
                    out.push_str(cell.symbol());
                }
            }
            out.push('\n');
        }
        out
    }

    /// 宽字符占双格, 格间填充会拆开子串: 断言按「去空白稠密串」比对。
    fn assert_shown(text: &str, needle: &str) {
        let dense =
            |value: &str| -> String { value.chars().filter(|ch| !ch.is_whitespace()).collect() };
        assert!(
            dense(text).contains(&dense(needle)),
            "帧里缺少 {needle:?}\n--- 帧 ---\n{text}"
        );
    }

    fn populated_app(motion: MotionMode) -> App {
        let mut app = App::new("http://127.0.0.1:8080", motion);
        app.feed(crate::state::Input::ConnectionOk);
        app.status.model = Some("demo-model".to_string());
        app.status.context_window = Some(128_000);
        app.status.usage = Some(UsageSnapshot {
            prompt_tokens: 12_300,
            completion_tokens: 4_560,
            total_tokens: 16_860,
            cache_hit_rate: None,
        });
        app.status.turn_latency_ms = Some(3_200);
        let tab = &mut app.tabs[0];
        tab.messages.push(Message::User("跑一个工具".to_string()));
        tab.messages.push(Message::Assistant(AssistantMessage {
            text: "# 结果\n- 完成\n```rust\nfn done() { return 1; }\n```".to_string(),
            streaming: false,
            tools: vec![ToolCard {
                name: "tool.demo".to_string(),
                status: ToolStatus::Ok,
                started_ms: 0,
                duration_ms: Some(342),
            }],
            usage: None,
            finish: None,
        }));
        tab.messages.push(Message::System(SystemNote {
            level: crate::state::NoteLevel::Info,
            text: "面板 3 内部过程频道: 未接线 (接口桩)".to_string(),
        }));
        app
    }

    /// 面板状态渲染单测: 对话 / 状态条 / 页签 / 频道桩全部上屏。
    #[test]
    fn panels_render_full_cockpit_state() {
        let app = populated_app(MotionMode::Full);
        let backend = TestBackend::new(160, 42);
        let mut terminal = Terminal::new(backend).expect("终端");
        terminal.draw(|frame| draw(frame, &app)).expect("渲染");

        let text = buffer_text(&terminal);
        // 面板 1: 用户 / 助手 / 工具卡片 / 代码高亮 / 流式标记。
        assert_shown(&text, "YOU");
        assert_shown(&text, "tool.demo");
        assert_shown(&text, "OK");
        assert_shown(&text, "342ms");
        assert_shown(&text, "code · rust");
        assert_shown(&text, "▸ 完成");
        // 页签。
        assert_shown(&text, "会话 1");
        // 面板 2: 上下文条 / token / 回合耗时 / 缓存诚实占位 / 活动指示。
        assert_shown(&text, "LINK OK");
        assert_shown(&text, "MODEL demo-model");
        assert_shown(&text, "CTX");
        assert_shown(&text, "TOK");
        assert_shown(&text, "TURN");
        assert_shown(&text, "CACHE —");
        assert_shown(&text, "MOTION FULL");
        // 面板 3: 显式未接线。
        assert_shown(&text, "未接线");
    }

    /// 连接失败即帧: 第一帧就是带错误卡的完整驾驶舱 (不白屏)。
    #[test]
    fn connection_failure_framed_on_first_frame() {
        let mut app = App::new("http://127.0.0.1:8080", MotionMode::Full);
        app.feed(crate::state::Input::ConnectionFailed {
            detail: "connection refused".to_string(),
        });
        let backend = TestBackend::new(140, 42);
        let mut terminal = Terminal::new(backend).expect("终端");
        terminal.draw(|frame| draw(frame, &app)).expect("渲染");
        let text = buffer_text(&terminal);

        assert_shown(&text, "LINK DOWN");
        assert_shown(&text, "连接失败");
        assert_shown(&text, "connection refused");
        // 不白屏: 面板骨架照常在。
        assert_shown(&text, "MAIN CHANNEL");
        assert_shown(&text, "PROCESS FEED");
        assert_shown(&text, "MOTION");
    }

    /// reduced-motion 档: 扫描线氛围关闭, full 档开启。
    #[test]
    fn reduced_motion_drops_scanline_texture() {
        let full_app = populated_app(MotionMode::Full);
        let backend = TestBackend::new(140, 42);
        let mut terminal = Terminal::new(backend).expect("终端");
        terminal.draw(|frame| draw(frame, &full_app)).expect("渲染");
        assert!(buffer_text(&terminal).contains("░"));

        let reduced_app = populated_app(MotionMode::Reduced);
        let backend = TestBackend::new(140, 42);
        let mut terminal = Terminal::new(backend).expect("终端");
        terminal
            .draw(|frame| draw(frame, &reduced_app))
            .expect("渲染");
        assert!(!buffer_text(&terminal).contains("░"));
    }

    /// 帮助浮层: 键位表 + 命令表齐上, Esc 关闭。
    #[test]
    fn help_overlay_lists_keys_and_commands() {
        let mut app = populated_app(MotionMode::Full);
        app.overlay = Some(Overlay::Help);
        let backend = TestBackend::new(140, 42);
        let mut terminal = Terminal::new(backend).expect("终端");
        terminal.draw(|frame| draw(frame, &app)).expect("渲染");
        let text = buffer_text(&terminal);
        assert_shown(&text, "KEY MAP");
        assert_shown(&text, "Ctrl+C");
        assert_shown(&text, "Esc Esc");
        assert_shown(&text, "/resume [会话id]");
    }
}
