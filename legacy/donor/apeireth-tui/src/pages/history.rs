//! 3 历史 (History, ΙΣΤΟΡΙΑ)
//!
//! - 顶: 6 流计数 (按 session/continuity 分组)
//! - 中: Episode 时间线
//! - 错误兜底: DB 不存在 → "暂无历史 · ..."

use apeireth_core::Episode;
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
            Constraint::Length(5), // 6 流计数
            Constraint::Min(0),    // Episode 时间线
        ])
        .split(area);

    render_stream_counts(f, chunks[0], style);
    render_timeline(f, chunks[1], app, style);
}

fn render_stream_counts(f: &mut Frame, area: Rect, style: &ThemeStyle) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 6 流 (R18 memory 6 流水) ",
            Style::default().fg(style.dim),
        ))
        .border_style(Style::default().fg(style.dim))
        .border_type(style.border_type);

    let counts = match backend::history_stream_counts() {
        Ok(c) => c,
        Err(e) => {
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  history empty · 启动 apeireth-web 或 apeireth-desktop 后再回",
                    Style::default().fg(style.dim),
                )),
                Line::from(Span::styled(
                    format!("  err: {e}"),
                    Style::default().fg(style.dim),
                )),
            ];
            f.render_widget(Paragraph::new(text).block(block), area);
            return;
        }
    };

    let labels = backend::six_stream_labels();
    let n = labels.len().max(1);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            (0..n)
                .map(|_| Constraint::Ratio(1, n as u32))
                .collect::<Vec<_>>(),
        )
        .split(area);

    // R26: 防御性长度对齐, labels / counts / cols 任何一个更短都截断
    let n_max = labels.len().min(counts.len()).min(cols.len());
    for i in 0..n_max {
        let (zh_label, _en) = &labels[i];
        let cell = cols[i];
        let (sess_name, n) = counts[i].clone();
        let text = vec![
            Line::from(Span::styled(
                format!("L{}", i + 1),
                Style::default().fg(style.dim),
            )),
            Line::from(Span::styled(
                *zh_label,
                Style::default()
                    .fg(style.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("{}", n),
                Style::default()
                    .fg(style.primary)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(sess_name, Style::default().fg(style.dim))),
        ];
        let b = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(style.dim))
            .border_type(style.border_type);
        f.render_widget(Paragraph::new(text).block(b), cell);
    }
}

fn render_timeline(f: &mut Frame, area: Rect, app: &App, style: &ThemeStyle) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Episode 时间线 (最近 30 条) ",
            Style::default().fg(style.dim),
        ))
        .border_style(Style::default().fg(style.dim))
        .border_type(style.border_type);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let episodes = match backend::history_recent(30) {
        Ok(v) => v,
        Err(e) => {
            f.render_widget(
                Paragraph::new(format!("(无 history 或 DB 不可用: {e})"))
                    .style(Style::default().fg(style.dim)),
                inner,
            );
            return;
        }
    };
    if episodes.is_empty() {
        f.render_widget(
            Paragraph::new("(空) 启动 apeireth-web 或 apeireth-desktop 后, dialog 会写入 episode")
                .style(Style::default().fg(style.dim)),
            inner,
        );
        return;
    }
    // R26: 按 timestamp 倒序 (最新在最上面), 用户最关心的最新 episode 在顶部
    let mut episodes_sorted = episodes;
    episodes_sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let lines: Vec<Line> = episodes_sorted
        .iter()
        .map(|ep| format_episode_line(ep, style))
        .collect();
    // R26: PageUp/PageDown 滚动支持
    let inner_h = inner.height as usize;
    let total = lines.len();
    let max_scroll = total.saturating_sub(inner_h) as u16;
    let scroll = if app.scroll_offset > max_scroll {
        max_scroll
    } else {
        app.scroll_offset
    };
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        inner,
    );
}

fn format_episode_line<'a>(ep: &'a Episode, style: &ThemeStyle) -> Line<'a> {
    let ts = chrono::DateTime::from_timestamp(ep.timestamp, 0)
        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| format!("时间戳={}", ep.timestamp));
    // 2026-08-04 修 (chuling via mavis): 之前 `&ep.content[..60]` 是 byte 截断,
    //   中文 content 60 字节落在多字节字符中间会 panic ("字节索引 N 不是字符边界").
    //   改用 char-based take(60) 截断, 中文也安全.
    let preview: String = {
        let chars: Vec<char> = ep.content.chars().collect();
        if chars.len() > 60 {
            let mut s: String = chars.iter().take(60).collect();
            s.push('…');
            s
        } else {
            ep.content.clone()
        }
    };
    let role_color = match ep.role.as_str() {
        "user" => style.primary,
        "助手" => style.accent,
        _ => style.dim,
    };
    Line::from(vec![
        Span::styled(format!("[{}] ", ts), Style::default().fg(style.dim)),
        Span::styled(
            format!("{:<14} ", ep.session_id),
            Style::default().fg(style.dim),
        ),
        Span::styled(
            format!("{:<6} ", ep.role),
            Style::default().fg(role_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(preview, Style::default().fg(style.primary)),
    ])
}

#[cfg(test)]
mod tests {
    /// 2026-08-04 回归测试 (chuling via mavis): episode preview 截断不能 panic.
    /// 修复前: `&ep.content[..60]` 在 byte 60 落在 CJK 字符中间时会 panic.
    /// 修复后: char-based take(60) 永远 char-boundary safe.
    #[test]
    fn preview_truncate_cjk_does_not_panic() {
        // 70 个 CJK 字符 = 210 bytes, byte 60 落在 20th 字符中间
        let content: String = "中".repeat(70);
        let chars: Vec<char> = content.chars().collect();
        let preview: String = if chars.len() > 60 {
            let mut s: String = chars.iter().take(60).collect();
            s.push('…');
            s
        } else {
            content.clone()
        };
        // 60 个中 + 1 个 …
        assert_eq!(preview.chars().count(), 61);
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn preview_truncate_short_no_truncate() {
        let content = "hello world".to_string();
        let chars: Vec<char> = content.chars().collect();
        let preview: String = if chars.len() > 60 {
            let mut s: String = chars.iter().take(60).collect();
            s.push('…');
            s
        } else {
            content.clone()
        };
        assert_eq!(preview, "hello world");
        assert!(!preview.ends_with('…'));
    }
}
