//! 任务面板 (驾驶舱面板 7, 浮层): 任务列表 (id 短哈希/标题/状态六态/尝试数/
//! 负责人) + 六态分组计数 + 选中详情 (payload 摘要 / 时间线)。
//!
//! 面板边界上屏明示: 只读 + 人判重试 (`r`, 仅 failed), 增删改派走工具面。
//! 诚实纪律: 无数据显式 "—" (与遥测面板组同语法), 合同外状态不猜分组上原样。

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::state::App;
use crate::tasks::{self, TaskRow, TaskStateView, HONEST_PLACEHOLDER};
use crate::theme::CockpitTheme;

/// 列表行标题截断宽度 (标题超出以 `…` 标截断)。
const TITLE_CHARS: usize = 12;

/// 合成任务面板浮层 (整屏居中, 模态)。
pub fn draw_panel(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let popup = super::centered_rect(88, 88, area);
    frame.render_widget(Clear, popup);

    let rows = app.tasks.len();
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.glow_primary))
        .style(Style::default().bg(palette.deep_space))
        .title(
            Line::from(vec![
                Span::styled(
                    " ▐ 任务面板 ",
                    Style::default()
                        .fg(palette.glow_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("· TASK BOARD ▌", Style::default().fg(palette.text_dim)),
            ])
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(Span::styled(
                format!(" TASKS · {rows} 行 · 只读 + 人判重试 "),
                Style::default().fg(palette.text_dim),
            ))
            .alignment(Alignment::Right),
        );
    frame.render_widget(outer, popup);

    let inner = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(6),
        Constraint::Length(2),
    ])
    .split(Rect {
        x: popup.x.saturating_add(1),
        y: popup.y.saturating_add(1),
        width: popup.width.saturating_sub(2),
        height: popup.height.saturating_sub(2),
    });

    render_counts(frame, app, theme, inner[0]);
    let body = Layout::horizontal([Constraint::Length(58), Constraint::Min(20)]).split(inner[1]);
    render_list(frame, app, theme, body[0]);
    render_detail(frame, app, theme, body[1]);
    render_footer(frame, theme, inner[2]);
}

/// 状态色 (未知状态不猜色: 暗色原样上屏)。
fn state_color(state: Option<TaskStateView>, theme: &CockpitTheme) -> Color {
    match state {
        Some(TaskStateView::Failed) => theme.palette.glow_danger,
        Some(TaskStateView::Done) => theme.palette.glow_ok,
        Some(TaskStateView::Running) => theme.palette.glow_primary,
        Some(TaskStateView::Blocked) => theme.palette.glow_warn,
        Some(TaskStateView::Pending) | Some(TaskStateView::Assigned) => theme.palette.text_primary,
        None => theme.palette.text_dim,
    }
}

/// 六态分组计数行 (无快照 = 显式 "—")。
fn render_counts(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.glow_secondary))
        .style(Style::default().bg(palette.hull))
        .title(
            Line::from(vec![
                Span::styled(
                    " 六态分组计数 ",
                    Style::default()
                        .fg(palette.glow_secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("· TASK COUNTS", Style::default().fg(palette.text_dim)),
            ])
            .alignment(Alignment::Left),
        );
    let counts = app.tasks.snapshot.as_ref().map(|snapshot| snapshot.counts);
    let line = tasks::counts_line(counts);
    let style = if counts.is_some() {
        Style::default().fg(palette.digit)
    } else {
        Style::default().fg(palette.text_dim)
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(line, style.bg(palette.hull)))).block(block),
        area,
    );
}

/// 任务列表 (行格式: `▸ #短哈希 · 状态 · 标题 · 尝试 n · 负责人`)。
fn render_list(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.tool_card))
        .style(Style::default().bg(palette.hull))
        .title(
            Line::from(vec![
                Span::styled(
                    " 任务列表 ",
                    Style::default()
                        .fg(palette.tool_card)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("· TASK LIST", Style::default().fg(palette.text_dim)),
            ])
            .alignment(Alignment::Left),
        );
    let mut lines: Vec<Line<'_>> = Vec::new();
    match &app.tasks.snapshot {
        None => {
            lines.push(Line::from(Span::styled(
                format!(" {HONEST_PLACEHOLDER}"),
                Style::default().fg(palette.text_dim),
            )));
            lines.push(Line::from(Span::styled(
                " 未回灌: — = 未探到 (不猜数)",
                Style::default().fg(palette.text_dim),
            )));
        }
        Some(snapshot) if snapshot.rows.is_empty() => {
            lines.push(Line::from(Span::styled(
                " 暂无任务 (0 行, 真实空态)",
                Style::default().fg(palette.text_dim),
            )));
        }
        Some(snapshot) => {
            for (index, row) in snapshot.rows.iter().enumerate() {
                let marker = if index == app.tasks.selected {
                    "▸ "
                } else {
                    "  "
                };
                let state_style = Style::default()
                    .fg(state_color(row.state, theme))
                    .bg(palette.hull);
                lines.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(palette.glow_primary)),
                    Span::styled(
                        tasks::short_hash(&row.id),
                        Style::default().fg(palette.digit).bg(palette.hull),
                    ),
                    Span::styled(format!(" · {} · ", row.state_label), state_style),
                    Span::styled(
                        truncate_title(&row.title, TITLE_CHARS),
                        Style::default().fg(palette.text_primary).bg(palette.hull),
                    ),
                    Span::styled(
                        format!(" · 尝试 {} · {}", row.attempts, owner_label(row)),
                        Style::default().fg(palette.text_dim).bg(palette.hull),
                    ),
                ]));
            }
        }
    }
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// 负责人列 (无负责人 = 显式 "—")。
fn owner_label(row: &TaskRow) -> String {
    row.assignee
        .clone()
        .unwrap_or_else(|| HONEST_PLACEHOLDER.to_string())
}

/// 标题截断 (超出以 `…` 标截断)。
fn truncate_title(title: &str, max_chars: usize) -> String {
    if title.chars().count() <= max_chars {
        return title.to_string();
    }
    let truncated: String = title.chars().take(max_chars).collect();
    format!("{truncated}…")
}

/// 选中详情 (payload 摘要 + 时间线)。
fn render_detail(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.glow_secondary))
        .style(Style::default().bg(palette.hull))
        .title(
            Line::from(vec![
                Span::styled(
                    " 选中详情 ",
                    Style::default()
                        .fg(palette.glow_secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "· PAYLOAD / TIMELINE",
                    Style::default().fg(palette.text_dim),
                ),
            ])
            .alignment(Alignment::Left),
        );
    let mut lines: Vec<Line<'_>> = Vec::new();
    match app.tasks.selected_row() {
        None => {
            lines.push(Line::from(Span::styled(
                format!(" {HONEST_PLACEHOLDER} 未选中任务"),
                Style::default().fg(palette.text_dim),
            )));
        }
        Some(row) => {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", tasks::short_hash(&row.id)),
                    Style::default().fg(palette.digit).bg(palette.hull),
                ),
                Span::styled(
                    format!("id: {}", row.id),
                    Style::default().fg(palette.text_dim).bg(palette.hull),
                ),
            ]));
            lines.push(Line::from(Span::styled(
                format!(
                    " 状态 {} · 尝试 {} · 负责人 {}",
                    row.state_label,
                    row.attempts,
                    owner_label(row)
                ),
                Style::default()
                    .fg(state_color(row.state, theme))
                    .bg(palette.hull),
            )));
            lines.push(Line::from(Span::styled(
                format!(" payload 摘要: {}", row.payload_summary),
                Style::default().fg(palette.text_primary).bg(palette.hull),
            )));
            lines.push(Line::from(Span::styled(
                format!(" 时间线 ({}):", row.timeline.len()),
                Style::default().fg(palette.glow_secondary).bg(palette.hull),
            )));
            if row.timeline.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("   {HONEST_PLACEHOLDER} 无事件"),
                    Style::default().fg(palette.text_dim).bg(palette.hull),
                )));
            }
            for entry in &row.timeline {
                let detail = entry
                    .detail
                    .as_deref()
                    .map(|detail| format!(" {detail}"))
                    .unwrap_or_default();
                lines.push(Line::from(Span::styled(
                    format!("   [{}] {}{}", entry.at_ms, entry.kind, detail),
                    Style::default().fg(palette.text_dim).bg(palette.hull),
                )));
            }
        }
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

/// 底部动作提示 (面板边界明示: 只读 + 人判重试, 增删改派走工具面)。
fn render_footer(frame: &mut Frame<'_>, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let lines = vec![
        Line::from(Span::styled(
            " ↑↓/jk 选择 · r 人判重试 (仅 failed) · Ctrl+T / Esc 关闭",
            Style::default().fg(palette.glow_primary).bg(palette.hull),
        )),
        Line::from(Span::styled(
            format!(" {}", crate::state::TASK_PANEL_BOUNDARY_NOTE),
            Style::default().fg(palette.text_dim).bg(palette.hull),
        )),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;
    use crate::state::{App, Input, Overlay};
    use crate::tasks::{snapshot_from_json, TaskBoardSnapshot, TaskCounts, TaskRow, TaskStateView};
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

    fn draw_panel_frame(app: &App) -> String {
        let backend = TestBackend::new(140, 42);
        let mut terminal = Terminal::new(backend).expect("终端");
        terminal
            .draw(|frame| crate::render::draw(frame, app))
            .expect("渲染");
        buffer_text(&terminal)
    }

    fn sample_rows() -> Vec<TaskRow> {
        [
            "pending", "assigned", "running", "done", "failed", "blocked",
        ]
        .iter()
        .enumerate()
        .map(|(index, state)| TaskRow {
            id: format!("task-{index}"),
            title: format!("任务标题{index}"),
            state: TaskStateView::parse(state),
            state_label: (*state).to_string(),
            attempts: index as u32,
            assignee: (index % 2 == 0).then(|| "worker-a".to_string()),
            payload_summary: "{\"step\":3}".to_string(),
            timeline: vec![crate::tasks::TaskTimelineEntry {
                kind: "enqueued".to_string(),
                at_ms: 1_700_000_000_000 + index as i64,
                detail: None,
            }],
        })
        .collect()
    }

    fn panel_app(rows: Vec<TaskRow>) -> App {
        let mut app = App::new("http://127.0.0.1:8080", MotionMode::Full);
        app.feed(Input::ConnectionOk);
        app.tasks.absorb(TaskBoardSnapshot {
            counts: TaskCounts {
                pending: 1,
                assigned: 1,
                running: 1,
                done: 1,
                failed: 1,
                blocked: 1,
                total: rows.len() as u64,
            },
            rows,
        });
        app.overlay = Some(Overlay::Tasks);
        app
    }

    /// 六态分组计数 + 列表 (短哈希/标题/状态/尝试数/负责人) 全上屏。
    #[test]
    fn task_panel_renders_six_state_counts_and_list_columns() {
        let mut app = panel_app(sample_rows());
        app.tasks.selected = 4;
        let text = draw_panel_frame(&app);

        // 六态分组计数 (零也是真数, 每态一行字都在)。
        assert_shown(&text, "六态分组计数");
        for label in ["待派 1", "已派 1", "执行 1", "完成 1", "失败 1", "阻塞 1"] {
            assert_shown(&text, label);
        }
        assert_shown(&text, "合计 6");
        // 列表列: id 短哈希 / 标题 / 状态六态 / 尝试数 / 负责人 (含 "—" 诚实位)。
        assert_shown(&text, &tasks::short_hash("task-0"));
        assert_shown(&text, &tasks::short_hash("task-4"));
        assert_shown(&text, "任务标题4");
        assert_shown(&text, "尝试 3");
        assert_shown(&text, "worker-a");
        assert_shown(&text, &format!("尝试 5 · {}", tasks::HONEST_PLACEHOLDER));
        // 选中详情: payload 摘要 + 时间线。
        assert_shown(&text, "选中详情");
        assert_shown(&text, "payload 摘要:");
        assert_shown(&text, "{\"step\":3}");
        assert_shown(&text, "时间线 (1)");
        assert_shown(&text, "enqueued");
        // 面板边界明示 (防越权: 增删改派走工具面)。
        assert_shown(&text, "只读 + 人判重试");
    }

    /// 空态 "—" 语义: 未回灌显式 "—", 真实空态如实报 0 行。
    #[test]
    fn task_panel_renders_honest_empty_states() {
        let mut app = App::new("http://127.0.0.1:8080", MotionMode::Full);
        app.feed(Input::ConnectionOk);
        app.overlay = Some(Overlay::Tasks);
        let text = draw_panel_frame(&app);
        assert_shown(&text, "六态分组计数");
        assert_shown(&text, tasks::HONEST_PLACEHOLDER);
        assert_shown(&text, "未回灌");
        assert_shown(&text, "未选中任务");

        app.tasks.absorb(TaskBoardSnapshot::default());
        let text = draw_panel_frame(&app);
        assert_shown(&text, "暂无任务 (0 行, 真实空态)");
        assert_shown(&text, "合计 0");
    }

    /// 合同外状态不猜分组: 原样标签 + 暗色 (渲染不造六态)。
    #[test]
    fn unknown_states_are_shown_verbatim_without_guessing() {
        let value = serde_json::json!({
            "tasks": [{
                "id": "task-9", "title": "怪态", "state": "maybe", "attempts": 0,
                "assignee": null, "created_at_ms": 1, "updated_at_ms": 1,
                "payload": null, "timeline": []
            }],
            "counts": {"pending": 0, "assigned": 0, "running": 0, "done": 0,
                       "failed": 0, "blocked": 0, "total": 1}
        });
        let mut app = App::new("http://127.0.0.1:8080", MotionMode::Full);
        app.feed(Input::ConnectionOk);
        app.tasks.absorb(snapshot_from_json(&value));
        app.overlay = Some(Overlay::Tasks);
        let text = draw_panel_frame(&app);
        assert_shown(&text, "maybe");
        assert_shown(&text, "状态 maybe");
    }
}
