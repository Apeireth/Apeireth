//! 2 生长 (Growth, ΑΥΞΗΣΙΣ)
//!
//! **R26 升级** (8/7 主审):
//! - 顶: 4 阶段工程用语 (Init/Bootstrap/Serving/Saturated) 进度条横向
//!   (砍 Birth / Reproduction / Migration / Rebirth / Decline / Death, 8 阶段 → 4 阶段)
//! - 中: 5 层洋葱 (E/S/A/M/O) + ValueDimension letters (保留 R19 设计)
//! - 右: 反思期 72h 环 (R26 真接 backend `compute_reflection_progress` = SqliteMemoryStore
//!   最近 72h episode / 1000 阈值, 旧实现写死 identity.birth_time 导致永远空圆, 修)
//!
//! **R11 LOCKED 边界**: `apeireth-core::LifeStage` 10 变体 enum 0 触, `LEGAL_TRANSITIONS` 12 条 0 触,
/// 仅 TUI 层 pages/growth.rs + backend.rs 调整.
use apeireth_value::ValueDimension;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::backend;
use crate::theme::ThemeStyle;

pub fn render(f: &mut Frame, area: Rect, _app: &App, style: &ThemeStyle) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // 8 阶段
            Constraint::Min(0),    // 洋葱 + 反思环
        ])
        .split(area);

    render_stages(f, chunks[0], style);
    render_onion_and_reflection(f, chunks[1], style);
}

fn render_stages(f: &mut Frame, area: Rect, style: &ThemeStyle) {
    let stages = backend::compute_life_stages_info().unwrap_or_default();
    let n = stages.len().max(1);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            (0..n)
                .map(|_| Constraint::Ratio(1, n as u32))
                .collect::<Vec<_>>(),
        )
        .split(area);

    for (i, stage) in stages.iter().enumerate() {
        if i >= cols.len() {
            break;
        }
        let cell = cols[i];
        let active = stage.active;
        let bar_w = 8;
        let filled = if active {
            bar_w
        } else {
            (bar_w as f64 * 0.4) as usize
        };
        let bar = format!(
            "{}{}",
            style.bar_full.to_string().repeat(filled),
            style.bar_empty.to_string().repeat(bar_w - filled)
        );
        let fg = if active { style.accent } else { style.dim };
        let text = vec![
            Line::from(Span::styled(
                format!("{}.{}", stage.idx, stage.zh),
                Style::default().fg(fg).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(&stage.en, Style::default().fg(fg))),
            Line::from(Span::styled(bar, Style::default().fg(fg))),
            Line::from(Span::styled(
                format!("[{}]", stage.r11_enum),
                Style::default().fg(style.dim),
            )),
        ];
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if active { style.accent } else { style.dim }))
            .border_type(style.border_type);
        f.render_widget(
            Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
            cell,
        );
    }
}

fn render_onion_and_reflection(f: &mut Frame, area: Rect, style: &ThemeStyle) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    render_onion(f, chunks[0], style);
    render_reflection_ring(f, chunks[1], style);
}

fn render_onion(f: &mut Frame, area: Rect, style: &ThemeStyle) {
    let _dims = ValueDimension::ALL; // 引用确保 R11 LOCKED 边界
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 5 层洋葱 (E / S / A / M / O) ",
            Style::default().fg(style.dim),
        ))
        .border_style(Style::default().fg(style.dim))
        .border_type(style.border_type);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let letters: Vec<(&str, &str)> = vec![
        ("O", "操作 (AI 主动 + 9 器官)"),
        ("M", "方法论"),
        ("A", "艺术性"),
        ("S", "叙事"),
        ("E", "原则 (R14-D7 LOCKED)"),
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    for (i, (l, desc)) in letters.iter().enumerate() {
        let layer_n = 5 - i;
        let indent = "  ".repeat(i);
        let bar = format!(
            "{}{}",
            style.bar_full.to_string().repeat(layer_n * 2),
            style.bar_empty.to_string().repeat((5 - layer_n) * 2)
        );
        let fg = if i == 0 { style.accent } else { style.primary };
        lines.push(Line::from(vec![
            Span::styled(format!("第{}层 ", layer_n), Style::default().fg(style.dim)),
            Span::styled(indent.clone(), Style::default().fg(style.dim)),
            Span::styled(
                format!("[{}] ", l),
                Style::default().fg(fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(bar, Style::default().fg(fg)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(format!("    {}", indent), Style::default().fg(style.dim)),
            Span::styled(*desc, Style::default().fg(style.dim)),
        ]));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_reflection_ring(f: &mut Frame, area: Rect, style: &ThemeStyle) {
    let progress = backend::compute_reflection_progress();
    let status = backend::compute_reflection_status();

    // R26 真接: progress 来自 backend 真实 SqliteMemoryStore 最近 72h episode / 1000 阈值
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 反思环 (R26 真接, 最近 72h episode / 1000 阈值) ",
            Style::default().fg(style.dim),
        ))
        .border_style(Style::default().fg(style.dim))
        .border_type(style.border_type);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let w = i32::from(inner.width);
    let h = i32::from(inner.height);
    let cx = w / 2;
    let cy = h / 2 - 2;
    let radius = (w.min(h) / 2 - 2).max(3) as f64;

    let mut lines_buf: Vec<String> = vec![String::new(); h as usize];
    for y in 0..h {
        for x in 0..w {
            let dx = f64::from(x - cx);
            let dy = f64::from(y - cy);
            let d = (dx * dx + dy * dy).sqrt();
            let ch = if (d - radius).abs() < 0.7 {
                let angle = dy.atan2(dx);
                let frac = (angle + std::f64::consts::PI) / (2.0 * std::f64::consts::PI);
                if frac < progress {
                    if progress >= 0.99 {
                        '●'
                    } else if progress >= 0.75 {
                        '◓'
                    } else if progress >= 0.5 {
                        '◑'
                    } else if progress >= 0.25 {
                        '◐'
                    } else {
                        '◔'
                    }
                } else {
                    '○'
                }
            } else {
                ' '
            };
            lines_buf[y as usize].push(ch);
        }
    }

    let mut lines: Vec<Line> = Vec::new();
    for (i, row) in lines_buf.iter().enumerate() {
        let color = if i == cy as usize {
            style.accent
        } else {
            style.primary
        };
        lines.push(Line::from(Span::styled(
            row.clone(),
            Style::default().fg(color),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("状态: ", Style::default().fg(style.dim)),
        Span::styled(&status, Style::default().fg(style.accent)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("进度: ", Style::default().fg(style.dim)),
        Span::styled(
            format!("{:.1}%", progress * 100.0),
            Style::default().fg(style.accent),
        ),
    ]));
    // R26: 0% = "无反思" (透明), 100% = "反思充分"
    let hint = if progress >= 1.0 {
        " 反思充分 (>= 1000 episodes in 72h)"
    } else if progress <= 0.0 {
        " 无反思 (0 episodes in 72h)"
    } else {
        " 进行中"
    };
    lines.push(Line::from(Span::styled(
        hint,
        Style::default().fg(style.dim),
    )));
    f.render_widget(Paragraph::new(lines), inner);
}
