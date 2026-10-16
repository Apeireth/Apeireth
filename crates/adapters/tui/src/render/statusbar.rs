//! 状态条 (面板 2): 上下文用量条 / token 计数 / 回合耗时 / 缓存命中率 /
//! 活动指示 / 氛围跳动数字。
//!
//! 诚实占位纪律: 接口没有的字段 (如缓存命中率) 一律显示 "—", 不猜不装。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::state::{ActivityState, App, ConnectionState};
use crate::theme::CockpitTheme;

/// 千分位收敛 token 数 (等宽观感: 12.3k / 2.30M)。
pub fn format_tokens(value: u32) -> String {
    if value < 1_000 {
        value.to_string()
    } else if value < 1_000_000 {
        format!("{:.1}k", f64::from(value) / 1_000.0)
    } else {
        format!("{:.2}M", f64::from(value) / 1_000_000.0)
    }
}

/// 回合耗时上屏 (毫秒 → 342ms / 3.2s)。
pub fn format_latency(ms: u64) -> String {
    if ms >= 1_000 {
        format!("{:.1}s", ms as f64 / 1_000.0)
    } else {
        format!("{ms}ms")
    }
}

/// 上下文用量条: `▰▰▰▱▱▱▱▱`; 窗口未知时只给占用量。
pub fn context_bar(used: u32, window: Option<u32>, width: u16) -> String {
    let width = width.max(1);
    let Some(window) = window.filter(|value| *value > 0) else {
        return "▱".repeat(width as usize);
    };
    let ratio = (f64::from(used) / f64::from(window)).clamp(0.0, 1.0);
    let filled = (ratio * f64::from(width)).round() as u16;
    let filled = filled.min(width);
    format!(
        "{}{}",
        "▰".repeat(filled as usize),
        "▱".repeat((width - filled) as usize)
    )
}

/// 状态条一整行 spans。
pub fn status_line<'a>(app: &'a App, theme: &CockpitTheme) -> Line<'a> {
    let palette = theme.palette;
    let mut spans: Vec<Span<'a>> = Vec::new();
    let mut push = |spans: &mut Vec<Span<'a>>, text: String, style: Style| {
        spans.push(Span::styled(text, style));
        spans.push(Span::styled(" │ ", Style::default().fg(palette.panel_edge)));
    };

    // 连接段。
    match &app.status.connection {
        ConnectionState::Connected => push(
            &mut spans,
            format!("LINK OK {}", app.endpoint),
            Style::default().fg(palette.glow_ok),
        ),
        ConnectionState::Checking => push(
            &mut spans,
            format!("LINK … {}", app.endpoint),
            Style::default().fg(palette.glow_warn),
        ),
        ConnectionState::Failed { .. } => push(
            &mut spans,
            "LINK DOWN 连接失败".to_string(),
            Style::default()
                .fg(palette.glow_danger)
                .add_modifier(Modifier::BOLD),
        ),
    }

    // 模型段。
    let model = app.status.model.as_deref().unwrap_or("默认");
    push(
        &mut spans,
        format!("MODEL {model}"),
        Style::default().fg(palette.glow_secondary),
    );

    // 上下文用量条。
    let used = app
        .status
        .usage
        .map(|usage| usage.total_tokens)
        .unwrap_or(0);
    let window_label = match app.status.context_window {
        Some(window) => format_tokens(window),
        None => "?".to_string(),
    };
    push(
        &mut spans,
        format!(
            "CTX {} {}/{}",
            context_bar(used, app.status.context_window, 8),
            format_tokens(used),
            window_label
        ),
        Style::default().fg(palette.glow_primary),
    );

    // token 计数。
    let (prompt, completion) = app
        .status
        .usage
        .map(|usage| (usage.prompt_tokens, usage.completion_tokens))
        .unwrap_or((0, 0));
    push(
        &mut spans,
        format!(
            "TOK ↑{} ↓{}",
            format_tokens(prompt),
            format_tokens(completion)
        ),
        Style::default().fg(palette.text_primary),
    );

    // 回合耗时。
    let latency = app
        .status
        .turn_latency_ms
        .map(format_latency)
        .unwrap_or_else(|| "—".to_string());
    push(
        &mut spans,
        format!("TURN {latency}"),
        Style::default().fg(palette.digit),
    );

    // 缓存命中率: 接口无此字段 → 诚实 "—"。
    let cache = match app.status.usage.and_then(|usage| usage.cache_hit_rate) {
        Some(rate) => format!("{rate}%"),
        None => "—".to_string(),
    };
    push(
        &mut spans,
        format!("CACHE {cache}"),
        Style::default().fg(palette.text_dim),
    );

    // 活动指示。
    let (glyph, label, color) = match app.status.activity {
        ActivityState::Streaming => (
            theme.spinner_glyph(app.frame),
            "STREAMING",
            palette.glow_warn,
        ),
        ActivityState::Idle => ("◆", "IDLE", palette.text_dim),
    };
    push(
        &mut spans,
        format!("{glyph} {label}"),
        Style::default().fg(color),
    );

    // 氛围跳动数字 + 动效档。
    spans.push(Span::styled(
        app.ticker.render(&theme.motion),
        Style::default().fg(palette.digit),
    ));
    spans.push(Span::styled(" │ ", Style::default().fg(palette.panel_edge)));
    spans.push(Span::styled(
        format!("MOTION {}", theme.mode.label()),
        Style::default().fg(palette.text_dim),
    ));
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// token 计数收敛: 个位 / k / M 三段。
    #[test]
    fn token_counts_collapse_to_kilo_units() {
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(12_345), "12.3k");
        assert_eq!(format_tokens(2_300_000), "2.30M");
    }

    /// 上下文用量条: 比例填充, 窗口未知时诚实空条。
    #[test]
    fn context_bar_fills_by_ratio() {
        assert_eq!(context_bar(0, Some(100), 8), "▱▱▱▱▱▱▱▱");
        assert_eq!(context_bar(100, Some(100), 8), "▰▰▰▰▰▰▰▰");
        assert_eq!(context_bar(50, Some(100), 4), "▰▰▱▱");
        assert_eq!(context_bar(50, None, 4), "▱▱▱▱");
        assert_eq!(context_bar(50, Some(0), 4), "▱▱▱▱");
    }

    /// 缓存命中率: 接口无字段 → 诚实 "—"。
    #[test]
    fn cache_hit_rate_is_honest_placeholder_without_interface() {
        let theme = CockpitTheme::new(crate::theme::MotionMode::Full);
        let app = App::new("http://127.0.0.1:8080", crate::theme::MotionMode::Full);
        let line = status_line(&app, &theme);
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(text.contains("CACHE —"));
        assert!(text.contains("TURN —"));
        assert_eq!(format_latency(342), "342ms");
        assert_eq!(format_latency(3_200), "3.2s");
    }
}
