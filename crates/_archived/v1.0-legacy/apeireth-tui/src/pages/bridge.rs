//! 0 舰桥 (Bridge, ΣΚΟΠΗ) — 默认首页
//!
//! **R26 升级** (8/7 主审):
//! - 顶: 3 行拆分 (Line1 ASI V0.5/continuity/philosophy + 5-Self)
//!        (Line2 4 阶段工程用语 Init/Bootstrap/Serving/Saturated + 反思/endurance)
//!        (Line3 cycle/token/eps 计数)
//!        (旧: 7 数字硬塞 2 行, 过窄屏截断)
//! - 中: 9 器官心跳卡 (左 3x3) + 30 crate 极坐标星图 (右, ASCII aspect 1:2 修正)
//! - 底: "→ press i to enter dialogue" 大字提示
//!
//! **R11 LOCKED 边界**: 0 触. 仅 TUI 层 pages/bridge.rs 渲染调整.

use apeireth_value::ValueDimension;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::backend;
use crate::theme::ThemeStyle;

pub fn render(f: &mut Frame, area: Rect, app: &App, style: &ThemeStyle) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // R26: 3 行 (ASI / stage / cycle) + 2 border
            Constraint::Min(0),    // 中间: 器官 + 星图
            Constraint::Length(2), // 提示
        ])
        .split(area);

    render_top_status(f, chunks[0], app, style);
    render_middle(f, chunks[1], app, style);
    render_hint(f, chunks[2], app, style);
}

fn render_top_status(f: &mut Frame, area: Rect, _app: &App, style: &ThemeStyle) {
    let status = backend::compute_main_ai_status().unwrap_or_else(|e| backend::MainAiStatus {
        asi_v05: 0.0,
        asi_continuity: 0.0,
        asi_philosophy: 0.0,
        life_stage: format!("错误: {e}"),
        life_stage_idx: 0,
        reflection_status: "?".into(),
        endurance: 0.0,
        episode_count: 0,
        cycle_count: 0,
        token_used: 0,
        token_r19: 0,
        five_self: "?".into(),
    });

    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // R26 升级: 7 数字拆 3 行, 每行 3-4 字段, 不再硬塞 (过窄屏不再截断)
    // Line 1: ASI V0.5 / continuity / philosophy / 5-Self
    // Line 2: 阶段 (R26 工程用语 badge) / 反思 / endurance
    // Line 3: cycle / token LLM / token R19 / eps
    let stage_badge = backend::stage_badge(status.life_stage_idx);
    let text = vec![
        Line::from(vec![
            Span::styled("北极星 ", Style::default().fg(style.dim)),
            Span::styled(
                format!("{:.3}", status.asi_v05),
                Style::default()
                    .fg(style.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  连续 ", Style::default().fg(style.dim)),
            Span::styled(
                format!("{:.3}", status.asi_continuity),
                Style::default().fg(style.primary),
            ),
            Span::styled("  哲学 ", Style::default().fg(style.dim)),
            Span::styled(
                format!("{:.3}", status.asi_philosophy),
                Style::default().fg(style.primary),
            ),
            Span::styled("  5-我 ", Style::default().fg(style.dim)),
            Span::styled(status.five_self.clone(), Style::default().fg(style.accent)),
        ]),
        Line::from(vec![
            Span::styled("阶段 ", Style::default().fg(style.dim)),
            Span::styled(
                format!("{}({})", stage_badge, status.life_stage_idx),
                Style::default()
                    .fg(style.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  反思 ", Style::default().fg(style.dim)),
            Span::styled(
                status.reflection_status.clone(),
                Style::default().fg(style.primary),
            ),
            Span::styled("  耐力 ", Style::default().fg(style.dim)),
            Span::styled(
                format!("{:.3}", status.endurance),
                Style::default().fg(style.primary),
            ),
        ]),
        Line::from(vec![
            Span::styled("循环 ", Style::default().fg(style.dim)),
            Span::styled(
                format!("{}", status.cycle_count),
                Style::default().fg(style.primary),
            ),
            Span::styled("  LLM 令牌 ", Style::default().fg(style.dim)),
            Span::styled(
                format!("{}", status.token_used),
                Style::default().fg(style.primary),
            ),
            Span::styled("  R19 令牌 ", Style::default().fg(style.dim)),
            Span::styled(
                format!("{}", status.token_r19),
                Style::default().fg(style.primary),
            ),
            Span::styled("  片段 ", Style::default().fg(style.dim)),
            Span::styled(
                format!("{}", status.episode_count),
                Style::default().fg(style.primary),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(text), inner);
}

fn render_middle(f: &mut Frame, area: Rect, _app: &App, style: &ThemeStyle) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    render_organs(f, chunks[0], style);
    render_star_chart(f, chunks[1], style);
}

fn render_organs(f: &mut Frame, area: Rect, style: &ThemeStyle) {
    let organs = backend::snapshot_all_organs().unwrap_or_default();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(area);

    for (i, row_area) in rows.iter().enumerate() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
            ])
            .split(*row_area);
        for (j, col_area) in cols.iter().enumerate() {
            let idx = i * 3 + j;
            if idx < organs.len() {
                let o = &organs[idx];
                let bar_w = (o.health * 8.0) as usize;
                let bar = format!(
                    "{}{}",
                    style.bar_full.to_string().repeat(bar_w),
                    style.bar_empty.to_string().repeat(8 - bar_w)
                );
                let text = vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{}", o.display),
                            Style::default()
                                .fg(style.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(format!(" {}", o.metaphor), Style::default().fg(style.dim)),
                    ]),
                    Line::from(Span::styled(bar, Style::default().fg(style.accent))),
                    Line::from(Span::styled(
                        format!("{:.2}", o.health),
                        Style::default().fg(style.primary),
                    )),
                    Line::from(Span::styled(&o.primary, Style::default().fg(style.dim))),
                ];
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(style.dim))
                    .border_type(style.border_type);
                f.render_widget(
                    Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
                    *col_area,
                );
            }
        }
    }
}

fn render_star_chart(f: &mut Frame, area: Rect, style: &ThemeStyle) {
    let nodes = backend::topology();
    let w = i32::from(area.width);
    let h = i32::from(area.height);
    let ax = i32::from(area.x);
    let ay = i32::from(area.y);
    // R26: ASCII cell 是 2:1 (height:width), 圆 y 方向要乘 2 否则椭圆被压扁
    let cx = ax + w / 2;
    let cy = ay + h / 2;
    let radius = f64::from((w.min(h * 2) / 2 - 3).max(3));

    let mut canvas: Vec<Vec<char>> = vec![vec![' '; w as usize]; h as usize];

    for n in &nodes {
        let x = cx + (n.r * radius * n.theta.cos()) as i32;
        // R26: y 乘 2 抵消 ASCII cell 2:1 aspect, 让圆更像圆
        let y = cy + ((n.r * radius * n.theta.sin()) * 2.0) as i32;
        if x >= ax && x < ax + w && y >= ay && y < ay + h {
            let ch = if n.active >= 0.9 {
                '*'
            } else if n.active >= 0.75 {
                '×'
            } else {
                style.star
            };
            let row = (y - ay) as usize;
            let col = (x - ax) as usize;
            if row < canvas.len() && col < canvas[row].len() {
                canvas[row][col] = ch;
            }
        }
    }

    let center_row = (cy - ay) as usize;
    let center_col = (cx - ax) as usize;
    if center_row < canvas.len() && center_col < canvas[0].len() {
        canvas[center_row][center_col] = '◆';
    }

    let lines: Vec<Line> = canvas
        .iter()
        .map(|row| {
            Line::from(
                row.iter()
                    .map(|&c| {
                        let color = match c {
                            '*' => style.accent,
                            '×' => style.primary,
                            '◆' => style.accent,
                            _ => style.dim,
                        };
                        Span::styled(c.to_string(), Style::default().fg(color))
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 30 crate 极坐标星图 (5 大组 × 6) ",
            Style::default().fg(style.dim),
        ))
        .border_style(Style::default().fg(style.dim))
        .border_type(style.border_type);
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_hint(f: &mut Frame, area: Rect, _app: &App, style: &ThemeStyle) {
    let text = vec![Line::from(vec![
        Span::styled(
            "→ ",
            Style::default()
                .fg(style.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("按 ", Style::default().fg(style.dim)),
        Span::styled(
            "i",
            Style::default()
                .fg(style.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 或 ", Style::default().fg(style.dim)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(style.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 进入对话", Style::default().fg(style.primary)),
    ])];
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(style.primary))
        .border_type(style.border_type);
    f.render_widget(Paragraph::new(text).block(block), area);
}

#[allow(dead_code)]
fn _value_dims_count() -> usize {
    ValueDimension::ALL.len()
}
