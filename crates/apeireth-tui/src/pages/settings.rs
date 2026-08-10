//! 4 设置 (Settings, ΤΑΞΙΣ)
//!
//! - mode (focus/inspire)
//! - theme (archaic/era) — 改了立即重渲染
//! - splash_enabled
//! - breath_enabled
//! - language (zh/en)

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::theme::ThemeStyle;

pub fn render(f: &mut Frame, area: Rect, app: &App, style: &ThemeStyle) {
    // R26: 顶部 1 行 RGB 预览 + 5 项 Length(2) (省 5 行高, 适配 30+ 小屏)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // RGB 预览 (R26 新增)
            Constraint::Length(3), // mode (1 inner + 2 borders)
            Constraint::Length(3), // theme
            Constraint::Length(3), // splash
            Constraint::Length(3), // breath
            Constraint::Length(3), // language
            Constraint::Length(3), // R26-3-fixes: 输出上限
            Constraint::Min(0),    // hint
        ])
        .split(area);

    // R26 主题 RGB 预览 (让用户看到当前是什么颜色, 不再黑盒)
    let primary_rgb = if let ratatui::style::Color::Rgb(r, g, b) = style.primary {
        format!("rgb({},{},{})", r, g, b)
    } else {
        "rgb(?,?,?)".to_string()
    };
    let accent_rgb = if let ratatui::style::Color::Rgb(r, g, b) = style.accent {
        format!("rgb({},{},{})", r, g, b)
    } else {
        "rgb(?,?,?)".to_string()
    };
    let rgb_preview = vec![Line::from(vec![
        Span::styled(" theme RGB  ", Style::default().fg(style.dim)),
        Span::styled("primary=", Style::default().fg(style.dim)),
        Span::styled(
            primary_rgb.clone(),
            Style::default().fg(style.primary).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  accent=", Style::default().fg(style.dim)),
        Span::styled(
            accent_rgb,
            Style::default().fg(style.accent).add_modifier(Modifier::BOLD),
        ),
    ])];
    f.render_widget(Paragraph::new(rgb_preview), chunks[0]);

    // R26-2 settings: 5 项, theme 接真渲染 (重组中间调), 其 4 项仍为占位 (cursor 选中高亮)
    // 0 触碰 persistence Settings JSON schema (5 字段不变)
    // R26-3-fixes: max_tokens 实时读 llm_config (Settings 改动不持久化到 settings.json,
    // 而是通过 llm_config 公开 API 改写 llm.json, 见 handle_settings_key)
    let max_tokens = crate::llm_config::load()
        .map(|c| c.max_tokens)
        .unwrap_or(8192);
    let items: Vec<(&str, String, &str, bool)> = vec![
        (
            "[m] 模式",
            app.mode.display_label().to_string(),
            "专注 / 灵感 (单选) - 暂未接入渲染 (R27+ 计划)",
            false,
        ),
        (
            "[t] 主题",
            app.theme.display_label().to_string(),
            "古朴金 / 时代蓝 (改了下帧重画)",
            true,
        ),
        (
            "[s] 启屏",
            format!("{}", app.splash_enabled),
            "启屏动画 (启动时按任意键进入)",
            false,
        ),
        (
            "[b] 呼吸",
            format!("{}", app.breath_enabled),
            "呼吸节律动画 (nav bar 常驻 Braille 字符跳动)",
            false,
        ),
        (
            "[l] 语言",
            app.language.display_label().to_string(),
            "中文 / 英文 - 暂未接入渲染 (R27+ 计划)",
            false,
        ),
        (
            "[- =] 输出上限",
            format!("{} token", max_tokens),
            "LLM 最大输出 (-/= 步进 1024, 范围 512-32768)",
            true,
        ),
    ];

    // R26: 5 项从 chunks[1] 开始 (顶部已用 chunks[0] 作 RGB 预览)
    for (i, (key, val, desc, enabled)) in items.iter().enumerate() {
        let chunk_idx = i + 1; // 跳过 chunks[0] RGB 预览
        // chunks: [0]=RGB, [1..6]=5/6 items, [7]=hint
        // 6 items now (mode/theme/splash/breath/language/输出上限), so chunks needed: 1..7
        if chunk_idx + 1 >= chunks.len() {
            break; // 跳过末尾 hint 区域
        }
        // R26-2: cursor 选中那行 → accent + BOLD 金色 (其他都 dim)
        // 5 项看起来平等, 只有当前选中高亮
        let selected = i == app.settings_cursor as usize;
        let border_color = if selected { style.accent } else { style.dim };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .border_type(style.border_type);
        let (key_color, val_color) = if selected {
            (style.accent, style.primary)
        } else {
            (style.dim, style.dim)
        };
        let val_modifier = if selected { Modifier::BOLD } else { Modifier::empty() };
        let text = vec![Line::from(vec![
            Span::styled(*key, Style::default().fg(key_color).add_modifier(Modifier::BOLD)),
            Span::styled("  ", Style::default().fg(style.dim)),
            Span::styled(val.clone(), Style::default().fg(val_color).add_modifier(val_modifier)),
            Span::styled("  ", Style::default().fg(style.dim)),
            Span::styled(*desc, Style::default().fg(style.dim)),
        ])];
        f.render_widget(Paragraph::new(text).block(block), chunks[chunk_idx]);
    }

    let hint_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  [j/k 或 ↑/↓] 切换  ·  按 m/t/s/b/l toggle 当前选中  ·  [q] 退出",
            Style::default().fg(style.dim),
        )),
    ];
    f.render_widget(
        Paragraph::new(hint_text).wrap(Wrap { trim: false }),
        *chunks.last().unwrap_or(&area),
    );
}
