//! 右栏遥测面板组 (P2): 记忆账本点阵 / 缓存命中率大数字 / 上下文用量波形 /
//! 治理事件灯阵 —— 伙伴的内在遥测, 不是系统资源监控。
//!
//! 每面板右上角小字副标签语法: `LEDGER · 3 库` / `CACHE` / `CTX · 128K` / `GOV`。
//! 动态效果全部渲染层逐帧循环: 波形走纸 / 描边呼吸辉光 / 大数字变化闪烁 /
//! 治理灯渐入; `--motion reduced` 档全部退化为静态。
//! 诚实纪律: 无数据显式 "—" / 暗格, 未接线数据源显式标注, 绝不画假曲线。

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::statusbar::format_tokens;
use crate::state::App;
use crate::telemetry::{self, GovVerdict, LampCell, LAMP_SLOTS, LEDGER_CELLS, WAVE_WIDTH};
use crate::theme::CockpitTheme;

/// 遥测面板组总高 (账本 7 + 大数字 7 + 波形 4 + 灯阵 5)。
pub const GROUP_HEIGHT: u16 = 23;

/// 合成遥测面板组 (纵向四面板; 底部剩余给 P1 频道桩)。
pub fn draw_group(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(7),
        Constraint::Length(7),
        Constraint::Length(4),
        Constraint::Length(5),
    ])
    .split(area);
    render_ledger(frame, app, theme, rows[0]);
    render_cache(frame, app, theme, rows[1]);
    render_wave(frame, app, theme, rows[2]);
    render_lamps(frame, app, theme, rows[3]);
}

/// 面板骨架: 左标题 (面板名) + 右上角小字副标签 + 呼吸辉光描边。
fn panel_block(
    theme: &CockpitTheme,
    frame_no: u64,
    accent: Color,
    title: &str,
    tag: String,
) -> Block<'static> {
    let border = theme
        .motion
        .breath_border(theme.palette.panel_edge, accent, frame_no);
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(theme.palette.hull))
        .title(
            Line::from(Span::styled(
                format!(" {title} "),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(Span::styled(
                format!(" {tag} "),
                Style::default().fg(theme.palette.text_dim),
            ))
            .alignment(Alignment::Right),
        )
}

/// 按前缀挑一条来源备注 (无备注 = 该节拉取全绿, 不硬造文案)。
fn note_for<'a>(notes: &'a [String], prefix: &str) -> Option<&'a str> {
    notes
        .iter()
        .find(|note| note.starts_with(prefix))
        .map(String::as_str)
}

/// 面板 1: 记忆账本点阵 (会话/记忆/保护/教训, 每格 = 单位计数)。
fn render_ledger(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let tag = format!("LEDGER · {} 库", app.telemetry.ledger.backed_rows());
    let block = panel_block(theme, app.frame, palette.glow_secondary, "记忆账本", tag);
    let mut lines: Vec<Line<'_>> = app
        .telemetry
        .ledger
        .rows()
        .iter()
        .map(|(label, value)| {
            Line::from(Span::styled(
                telemetry::ledger_line(label, *value, LEDGER_CELLS),
                Style::default()
                    .fg(if value.is_some() {
                        palette.digit
                    } else {
                        palette.text_dim
                    })
                    .bg(palette.hull),
            ))
        })
        .collect();
    lines.push(Line::from(Span::styled(
        note_for(&app.telemetry.notes, "教训")
            .unwrap_or("数据未回灌: — = 未探到 (不猜数)")
            .to_string(),
        Style::default().fg(palette.text_dim).bg(palette.hull),
    )));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// 面板 2: 缓存命中率大数字仪表 (变化闪烁一帧; 无数据显式 "—")。
fn render_cache(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let flash = theme.motion.digit_flash_enabled && app.telemetry.cache.flash_active(app.frame);
    let digit_style = if flash {
        Style::default()
            .fg(palette.glow_primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.digit)
    };
    let block = panel_block(
        theme,
        app.frame,
        palette.digit,
        "缓存命中",
        "CACHE".to_string(),
    );
    let inner_width = area.width.saturating_sub(2) as usize;
    let rows = telemetry::big_number_rows(&app.telemetry.cache.render());
    let lines: Vec<Line<'_>> = rows
        .into_iter()
        .map(|row| {
            let pad = inner_width.saturating_sub(row.chars().count()) / 2;
            Line::from(Span::styled(
                format!("{}{}", " ".repeat(pad), row),
                digit_style.bg(palette.hull),
            ))
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// 面板 3: 上下文用量波形 (滚动 sparkline, 每动画帧走纸一格)。
fn render_wave(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let window = app.status.context_window;
    let tag = format!("CTX · {}", telemetry::window_tag(window));
    let block = panel_block(theme, app.frame, palette.glow_primary, "上下文波形", tag);
    let used = app
        .status
        .usage
        .map(|usage| format_tokens(usage.total_tokens))
        .unwrap_or_else(|| telemetry::HONEST_PLACEHOLDER.to_string());
    let scale = telemetry::window_tag(window);
    let wave = app.telemetry.wave.render(WAVE_WIDTH, window);
    let lines = vec![
        Line::from(Span::styled(
            wave,
            Style::default().fg(palette.glow_primary).bg(palette.hull),
        )),
        Line::from(Span::styled(
            format!("用量 {used} / {scale}"),
            Style::default().fg(palette.text_dim).bg(palette.hull),
        )),
    ];
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// 面板 4: 治理事件灯阵 (绿=放行 / 琥珀=审批 / 红=拒绝, 时间序点亮, 新灯渐入)。
fn render_lamps(frame: &mut Frame<'_>, app: &App, theme: &CockpitTheme, area: Rect) {
    let palette = theme.palette;
    let block = panel_block(
        theme,
        app.frame,
        palette.glow_ok,
        "治理灯阵",
        "GOV".to_string(),
    );
    let cells = app.telemetry.lamps.cells(app.frame, &theme.motion);
    let lamp_line = Line::from(
        cells
            .iter()
            .map(|cell| lamp_span(cell, theme))
            .collect::<Vec<Span<'_>>>(),
    );
    let legend = Line::from(vec![
        Span::styled("●放行 ", Style::default().fg(palette.glow_ok)),
        Span::styled("●审批 ", Style::default().fg(palette.glow_warn)),
        Span::styled("●拒绝", Style::default().fg(palette.glow_danger)),
    ]);
    let lit = app.telemetry.lamps.len();
    let tail = if lit == 0 {
        "无治理事件: 暗格待点亮 (不猜色)".to_string()
    } else {
        format!("最近 {lit} 判定 / {LAMP_SLOTS} 格 (时间序)")
    };
    let tail = match note_for(&app.telemetry.notes, "治理灯阵") {
        Some(note) => format!("{tail} · {note}"),
        None => tail,
    };
    let tail = Line::from(Span::styled(
        tail,
        Style::default().fg(palette.text_dim).bg(palette.hull),
    ));
    frame.render_widget(
        Paragraph::new(vec![lamp_line, legend, tail]).block(block),
        area,
    );
}

/// 灯格 span: 按判定三色 + 渐入亮度着色; 暗格不着色 (无数据 = 暗格)。
fn lamp_span(cell: &LampCell, theme: &CockpitTheme) -> Span<'static> {
    let palette = theme.palette;
    let (lit, accent) = match cell.verdict {
        Some(GovVerdict::Allow) => ("●", palette.glow_ok),
        Some(GovVerdict::Approve) => ("●", palette.glow_warn),
        Some(GovVerdict::Reject) => ("●", palette.glow_danger),
        None => return Span::styled("▫ ", Style::default().fg(palette.panel_edge)),
    };
    let color = crate::theme::blend(palette.panel_edge, accent, cell.level);
    Span::styled(
        format!("{lit} "),
        Style::default().fg(color).bg(palette.hull),
    )
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;
    use crate::state::Input;
    use crate::telemetry::{GovernanceEvent, LedgerCounts, TelemetrySnapshot};
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

    fn draw_frame(app: &App) -> String {
        let backend = TestBackend::new(100, 42);
        let mut terminal = Terminal::new(backend).expect("终端");
        terminal
            .draw(|frame| crate::render::draw(frame, app))
            .expect("渲染");
        buffer_text(&terminal)
    }

    fn live_app() -> App {
        let mut app = App::new("http://127.0.0.1:8080", MotionMode::Full);
        app.feed(Input::ConnectionOk);
        app.status.context_window = Some(128_000);
        app.telemetry.absorb(
            TelemetrySnapshot {
                ledger: LedgerCounts {
                    sessions: Some(3),
                    memories: Some(5),
                    protected: Some(1),
                    lessons: None,
                },
                cache_hit_rate: None,
                governance: vec![
                    GovernanceEvent {
                        at_ms: 100,
                        capability: "tool.a".into(),
                        verdict: GovVerdict::Allow,
                    },
                    GovernanceEvent {
                        at_ms: 200,
                        capability: "tool.b".into(),
                        verdict: GovVerdict::Approve,
                    },
                    GovernanceEvent {
                        at_ms: 300,
                        capability: "tool.c".into(),
                        verdict: GovVerdict::Reject,
                    },
                ],
                notes: vec!["教训计数: 未接线 (自述探测口未上 HTTP 契约)".to_string()],
            },
            0,
        );
        app.telemetry.observe_usage(None, 64_000, 0);
        app
    }

    /// 角标签渲染: `LEDGER · 3 库` / `CACHE` / `CTX · 128K` / `GOV` 齐上屏。
    #[test]
    fn telemetry_panels_render_corner_tags() {
        let app = live_app();
        let text = draw_frame(&app);
        assert_shown(&text, "LEDGER · 3 库");
        assert_shown(&text, "CACHE");
        assert_shown(&text, "CTX · 128K");
        assert_shown(&text, "GOV");
    }

    /// 无数据诚实占位: 显式 "—" / 暗格, 绝不画假曲线; 未接线来源显式标注。
    #[test]
    fn telemetry_panels_show_honest_placeholders_without_data() {
        let mut app = App::new("http://127.0.0.1:8080", MotionMode::Full);
        app.feed(Input::ConnectionOk);
        let text = draw_frame(&app);

        // 账本四行全暗 + "—"; 大数字 "—"; 灯阵暗格。
        assert_shown(&text, "会话 ▫▫▫▫▫▫▫▫▫▫ —");
        assert_shown(&text, "教训 ▫▫▫▫▫▫▫▫▫▫ —");
        assert!(text.contains('▫'), "无数据须给暗格\n{text}");
        assert!(!text.contains('▪'), "无数据不得点亮点阵格\n{text}");
        // 绝不画假曲线: 无采样不得出现波形帧循环字形。
        for glyph in telemetry::WAVE_GLYPHS {
            assert!(!text.contains(glyph), "无数据不得画波形 ({glyph})\n{text}");
        }

        // 数据源未接线 → 显式「未接线」标注 (沿用 P1 桩的诚实语法)。
        app.telemetry.absorb(
            TelemetrySnapshot {
                notes: vec!["教训计数: 未接线 (自述探测口未上 HTTP 契约)".to_string()],
                ..TelemetrySnapshot::default()
            },
            0,
        );
        let text = draw_frame(&app);
        assert_shown(&text, "教训计数: 未接线");
    }

    /// 有数据: 点阵按单位计数点亮, 波形按用量比例出帧, 灯阵时间序上灯。
    #[test]
    fn telemetry_panels_render_live_counts_wave_and_lamps() {
        let app = live_app();
        let text = draw_frame(&app);
        // 点阵计数: 3 亮 7 暗; 无教训数据行保持 "—"。
        assert_shown(&text, "会话 ▪▪▪▫▫▫▫▫▫▫ 3");
        assert_shown(&text, "记忆 ▪▪▪▪▪▫▫▫▫▫ 5");
        assert_shown(&text, "保护 ▪▫▫▫▫▫▫▫▫▫ 1");
        assert_shown(&text, "教训 ▫▫▫▫▫▫▫▫▫▫ —");
        // 波形: 64k/128K → 中档帧循环字形。
        assert!(text.contains('▃'), "波形帧循环字形未上屏\n{text}");
        // 大数字仪表: 无缓存数据 = 破折号字形 (不编数)。
        assert!(text.contains('█'), "大数字字形未上屏\n{text}");
        // 治理灯阵: 三盏判定灯已点亮。
        let lit_lamps = text.matches('●').count();
        assert!(lit_lamps >= 3, "治理灯阵未点亮\n{text}");
    }

    /// 大数字变化闪烁: 新值帧亮度脉冲一帧, 之后回到常亮样式。
    #[test]
    fn big_digit_flashes_for_one_frame_then_settles() {
        let mut app = live_app();
        app.frame = 5;
        app.telemetry.observe_usage(Some(87), 64_000, 5);
        let flash_styles = digit_styles(&draw_frame_with_styles(&app));

        app.frame = 6;
        let steady_styles = digit_styles(&draw_frame_with_styles(&app));
        app.frame = 7;
        let later_styles = digit_styles(&draw_frame_with_styles(&app));

        assert!(!flash_styles.is_empty(), "大数字像素块未上屏");
        assert_ne!(flash_styles, steady_styles, "新值帧应有亮度脉冲 (样式不同)");
        assert_eq!(steady_styles, later_styles, "脉冲只有一帧, 之后常亮");
    }

    fn draw_frame_with_styles(app: &App) -> Vec<(char, String)> {
        let backend = TestBackend::new(100, 42);
        let mut terminal = Terminal::new(backend).expect("终端");
        terminal
            .draw(|frame| crate::render::draw(frame, app))
            .expect("渲染");
        let buffer = terminal.backend().buffer();
        let area = buffer.area();
        let mut cells = Vec::new();
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buffer.cell((x, y)) {
                    cells.push((
                        cell.symbol().chars().next().unwrap_or(' '),
                        format!("{:?}", cell.fg),
                    ));
                }
            }
        }
        cells
    }

    fn digit_styles(cells: &[(char, String)]) -> Vec<String> {
        cells
            .iter()
            .filter(|(symbol, _)| *symbol == '█')
            .map(|(_, fg)| fg.clone())
            .collect()
    }
}
