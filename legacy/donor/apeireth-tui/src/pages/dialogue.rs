//! 1 对话 (Dialogue, ΔΙΑΛΟΓΟΣ)
//!
//! 设计参考 Claude Code 泄露源码 (src/components/messages/*.tsx):
//! - user 消息: 整块背景色 (userMessageBackground), 右 padding
//! - AI 消息: 左边框, 灰底, markdown 风格
//! - Thinking 链: 默认折叠 `∴ Thinking`, Ctrl+O 展开, 灰斜体
//! - 消息间隔 marginTop = 1
//!
//! W2.3 UX 借鉴 Claude Code:
//! - user 气泡用 theme.bg + dim fg 反色背景块
//! - Thinking 折叠, `Ctrl+O` 展开 (toggle thinking_expanded)
//! - Ctrl+O 状态存 App.thinking_expanded

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};
use ratatui::Frame;

use crate::app::App;
use crate::theme::ThemeStyle;
use unicode_width::UnicodeWidthStr;

/// R28 char-level 选区: 视觉行元信息 (render 时填到 chat_line_map, mouse handler + 复制用)
#[derive(Debug, Clone)]
pub struct LineInfo {
    /// 属于哪条 chat 消息 (选区高亮 + 字符提取定位 msg)
    pub msg_idx: usize,
    /// 该视觉行的纯文本内容 (不含 prefix / 图标, 复制时拼接用)
    pub text: String,
    /// prefix 部分 (如 " ❯ ", " ▌ ", " · ") 的显示宽度 cell cols. 鼠标 mx 减掉这个才是该行内 char_offset.
    pub prefix_cols: u16,
}

pub fn render(f: &mut Frame, area: Rect, app: &mut App, style: &ThemeStyle) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(area);

    render_history(f, chunks[0], app, style);
    render_status_bar(f, chunks[1], app, style);
    render_input(f, chunks[2], app, style);
}

/// R26-3-fixes: 反向计算 Paragraph::scroll.y
/// - scroll_to_bottom=true: 锁底, y = max_scroll (显示最新内容)
/// - scroll_to_bottom=false: 用户脱离锁底, scroll_offset = 向上滚了多少行
///   方向与 Paragraph::scroll.y (从顶部跳过多少行) 相反, 反向计算: y = max_scroll - scroll_offset
/// - PageUp → scroll_offset += 5 → y 减小 (看更早内容)
/// - PageDown → scroll_offset -= 5 → y 增大 (看更新内容), 到底重新锁底
/// - 滚轮同 PageUp/PageDown 方向 (在 main.rs)
pub(super) fn compute_scroll_y(scroll_to_bottom: bool, scroll_offset: u16, max_scroll: u16) -> u16 {
    if scroll_to_bottom {
        max_scroll
    } else {
        max_scroll.saturating_sub(scroll_offset)
    }
}

/// R26-3-fixes: Scrollbar position 重映射 (ratatui 公式期望 range [0, content-1])
/// - scroll_offset = 0 → position = 0 (thumb 顶)
/// - scroll_offset = max_scroll → position = content - 1 (thumb 底)
/// - max_scroll = 0 (content <= viewport) → position = 0
pub(super) fn compute_scrollbar_position(scroll: u16, max_scroll: u16, total: usize) -> usize {
    if max_scroll == 0 || total == 0 {
        return 0;
    }
    let s = u64::from(scroll);
    let m = u64::from(max_scroll);
    let n = (total as u64).saturating_sub(1);
    (s.saturating_mul(n) / m) as usize
}

/// R26-3-fixes: 判断某视觉行是否在选区内 (用于行级背景色反转)
/// - None: 无选区
/// - Some((line_a, line_b)): line_a/min..max/line_b 范围内的视觉行都算选中
pub(super) fn line_in_selection(app: &App, line_idx: usize) -> bool {
    match app.selection {
        None => false,
        // R28 char-level: 选区是 ((line,char),(line,char)) 元组, 范围按 line 维度判断
        Some(((line_a, _), (line_b, _))) => {
            let lo = line_a.min(line_b);
            let hi = line_a.max(line_b);
            line_idx >= lo && line_idx <= hi
        }
    }
}

/// R28 char-level: 算选区字符数 (跟 main::selection_text 同语义, 走归一化 + 切片)
/// 状态栏 "已选 N 字符" 用, 跟实际复制内容严格对齐.
pub fn compute_selection_char_count(app: &App) -> usize {
    let Some(((line_a, char_a), (line_b, char_b))) = app.selection else {
        return 0;
    };
    let (lo_line, lo_char, hi_line, hi_char) = if (line_a, char_a) <= (line_b, char_b) {
        (line_a, char_a, line_b, char_b)
    } else {
        (line_b, char_b, line_a, char_a)
    };
    if lo_line == hi_line && lo_char == hi_char {
        return 0;
    }
    let chars_in_line = |line_idx: usize| -> usize {
        app.chat_line_map
            .get(line_idx)
            .map(|info| info.text.chars().count())
            .unwrap_or(0)
    };
    let mut total = 0;
    for i in lo_line..=hi_line {
        let total_in_line = chars_in_line(i);
        let s = if i == lo_line {
            lo_char.min(total_in_line)
        } else {
            0
        };
        let e = if i == hi_line {
            hi_char.min(total_in_line)
        } else {
            total_in_line
        };
        if e > s {
            total += e - s;
        }
    }
    total
}

/// R28: 给定 chat_line_map 行索引, 返回该行文本内被选中的字符范围 (start, end).
/// - 返回 None: 该行不在选区内, 渲染时该行不加 REVERSED
/// - 返回 Some((start, end)): 选中 [start..end) 字符, 渲染时切 span 加 REVERSED
/// - 同行选区 (lo_line == hi_line): 返回 (lo_char, hi_char)
/// - 多行选区: 起始行返回 (lo_char, text.chars().count()), 中间行返回 (0, len), 结束行返回 (0, hi_char)
pub(super) fn selection_char_range_for_line(app: &App, line_idx: usize) -> Option<(usize, usize)> {
    let Some(((line_a, char_a), (line_b, char_b))) = app.selection else {
        return None;
    };
    let (lo_line, lo_char, hi_line, hi_char) = if (line_a, char_a) <= (line_b, char_b) {
        (line_a, char_a, line_b, char_b)
    } else {
        (line_b, char_b, line_a, char_a)
    };
    if line_idx < lo_line || line_idx > hi_line {
        return None;
    }
    let total_chars = app
        .chat_line_map
        .get(line_idx)
        .map(|i| i.text.chars().count())
        .unwrap_or(0);
    let start = if line_idx == lo_line {
        lo_char.min(total_chars)
    } else {
        0
    };
    let end = if line_idx == hi_line {
        hi_char.min(total_chars)
    } else {
        total_chars
    };
    if end <= start {
        return None;
    }
    Some((start, end))
}

/// R28 char-level highlight: 给一段文本 + 基础样式 + 选区, 返回多个 Span (0/1/2/3 段).
/// - sel_range = None: 返回 1 段 (整段不加 REVERSED)
/// - sel_range = Some((s, e)) 且 0<=s<e<=len: 返回 3 段 (before/REVERSED/after)
/// - 边界 (s==e 或越界): 当 None 处理 (返回 1 段)
fn styled_with_selection(
    text: &str,
    base_style: Style,
    sel_range: Option<(usize, usize)>,
) -> Vec<Span<'static>> {
    let Some((start, end)) = sel_range else {
        return vec![Span::styled(text.to_string(), base_style)];
    };
    let total = text.chars().count();
    let s = start.min(total);
    let e = end.min(total);
    if e <= s {
        return vec![Span::styled(text.to_string(), base_style)];
    }
    let chars: Vec<char> = text.chars().collect();
    let before: String = chars[..s].iter().collect();
    let middle: String = chars[s..e].iter().collect();
    let after: String = chars[e..].iter().collect();
    let mut spans = Vec::with_capacity(3);
    if !before.is_empty() {
        spans.push(Span::styled(before, base_style));
    }
    spans.push(Span::styled(
        middle,
        base_style.add_modifier(Modifier::REVERSED),
    ));
    if !after.is_empty() {
        spans.push(Span::styled(after, base_style));
    }
    spans
}

#[cfg(test)]
mod tests_r28_char_selection {
    use super::*;
    use crate::app::App;

    fn mk_app_with_lines(lines: Vec<(&'static str, u16)>) -> App {
        let mut app = App::default();
        app.chat_line_map = lines
            .into_iter()
            .enumerate()
            .map(|(i, (t, p))| LineInfo {
                msg_idx: i,
                text: t.to_string(),
                prefix_cols: p,
            })
            .collect();
        app
    }

    #[test]
    fn char_count_no_selection_returns_zero() {
        let app = mk_app_with_lines(vec![("hello", 3)]);
        assert_eq!(compute_selection_char_count(&app), 0);
    }

    #[test]
    fn char_count_same_line_same_offset_returns_zero() {
        let mut app = mk_app_with_lines(vec![("hello", 3)]);
        app.selection = Some(((0, 2), (0, 2)));
        assert_eq!(compute_selection_char_count(&app), 0);
    }

    #[test]
    fn char_count_single_line_partial() {
        let mut app = mk_app_with_lines(vec![("hello world", 3)]);
        app.selection = Some(((0, 0), (0, 5)));
        assert_eq!(compute_selection_char_count(&app), 5); // "hello"
    }

    #[test]
    fn char_count_multi_line_with_newlines() {
        let mut app = mk_app_with_lines(vec![("hello", 3), ("world", 3), ("!", 3)]);
        app.selection = Some(((0, 0), (2, 1)));
        // line 0: 5 chars [0..5)
        // line 1: 5 chars [0..5)
        // line 2: 1 char  [0..1)
        assert_eq!(compute_selection_char_count(&app), 11);
    }

    #[test]
    fn char_count_normalizes_reversed_endpoints() {
        let mut app = mk_app_with_lines(vec![("hello", 3), ("world", 3)]);
        app.selection = Some(((1, 2), (0, 1)));
        // Normalize to ((0,1),(1,2))
        // line 0: 4 chars [1..5)
        // line 1: 2 chars [0..2)
        assert_eq!(compute_selection_char_count(&app), 6);
    }

    #[test]
    fn char_count_clamps_out_of_bounds_offset() {
        let mut app = mk_app_with_lines(vec![("hi", 3)]);
        app.selection = Some(((0, 0), (0, 100)));
        assert_eq!(compute_selection_char_count(&app), 2); // "hi"
    }

    #[test]
    fn char_count_cjk_chars_counted_as_chars_not_bytes() {
        let mut app = mk_app_with_lines(vec![("晚上好", 3)]);
        app.selection = Some(((0, 0), (0, 2)));
        assert_eq!(compute_selection_char_count(&app), 2); // "晚上"
    }

    #[test]
    fn char_range_no_selection() {
        let app = mk_app_with_lines(vec![("hello", 3)]);
        assert_eq!(selection_char_range_for_line(&app, 0), None);
    }

    #[test]
    fn char_range_single_line_full() {
        let mut app = mk_app_with_lines(vec![("hello", 3)]);
        app.selection = Some(((0, 0), (0, 5)));
        assert_eq!(selection_char_range_for_line(&app, 0), Some((0, 5)));
    }

    #[test]
    fn char_range_single_line_partial() {
        let mut app = mk_app_with_lines(vec![("hello world", 3)]);
        app.selection = Some(((0, 2), (0, 7)));
        assert_eq!(selection_char_range_for_line(&app, 0), Some((2, 7)));
    }

    #[test]
    fn char_range_multi_line_first_line() {
        let mut app = mk_app_with_lines(vec![("aaa", 3), ("bbb", 3)]);
        app.selection = Some(((0, 1), (1, 1)));
        assert_eq!(selection_char_range_for_line(&app, 0), Some((1, 3))); // "aa" (chars 1..3)
    }

    #[test]
    fn char_range_multi_line_middle_line() {
        let mut app = mk_app_with_lines(vec![("aaa", 3), ("bbb", 3), ("ccc", 3)]);
        app.selection = Some(((0, 0), (2, 3)));
        assert_eq!(selection_char_range_for_line(&app, 1), Some((0, 3))); // entire "bbb"
    }

    #[test]
    fn char_range_multi_line_last_line() {
        let mut app = mk_app_with_lines(vec![("aaa", 3), ("bbb", 3)]);
        app.selection = Some(((0, 0), (1, 2)));
        assert_eq!(selection_char_range_for_line(&app, 1), Some((0, 2))); // "bb"
    }

    #[test]
    fn char_range_line_outside_returns_none() {
        let mut app = mk_app_with_lines(vec![("aaa", 3), ("bbb", 3), ("ccc", 3)]);
        app.selection = Some(((0, 0), (0, 3)));
        assert_eq!(selection_char_range_for_line(&app, 1), None);
        assert_eq!(selection_char_range_for_line(&app, 2), None);
    }

    #[test]
    fn char_range_normalizes_reversed_endpoints() {
        let mut app = mk_app_with_lines(vec![("aaa", 3), ("bbb", 3)]);
        app.selection = Some(((1, 2), (0, 1))); // reversed
        assert_eq!(selection_char_range_for_line(&app, 0), Some((1, 3)));
        assert_eq!(selection_char_range_for_line(&app, 1), Some((0, 2)));
    }

    #[test]
    fn char_range_zero_length_returns_none() {
        let mut app = mk_app_with_lines(vec![("hello", 3)]);
        app.selection = Some(((0, 2), (0, 2)));
        assert_eq!(selection_char_range_for_line(&app, 0), None);
    }

    #[test]
    fn char_range_clamps_overshoot() {
        let mut app = mk_app_with_lines(vec![("hi", 3)]);
        app.selection = Some(((0, 0), (0, 100)));
        assert_eq!(selection_char_range_for_line(&app, 0), Some((0, 2)));
    }
}

fn render_history(f: &mut Frame, area: Rect, app: &mut App, style: &ThemeStyle) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 对话 (CognitiveCycle · run_cycle) ",
            Style::default().fg(style.dim),
        ))
        .border_style(Style::default().fg(style.dim))
        .border_type(style.border_type);

    if app.chat_history.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  (空) 在下方输入文字, 按 Enter 提交",
                Style::default().fg(style.dim),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  · LLM 真接 minimaxi (OpenAI 协议)",
                Style::default().fg(style.dim),
            )),
            Line::from(Span::styled(
                "  · run_cycle 真跑 (R19 认知循环)",
                Style::default().fg(style.dim),
            )),
            Line::from(Span::styled(
                "  · 思考链 <think>...</think> 折叠 (Ctrl+O 展开)",
                Style::default().fg(style.dim),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  ↑ 调出上一条 user 输入, Tab 切 nav, q 退出",
                Style::default().fg(style.dim),
            )),
        ];
        f.render_widget(Paragraph::new(text).block(block), area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    // R26-3-fixes: 选区反馈 - 选中消息的所有 span 加 Modifier::REVERSED (反色显示)
    // 这是个简单粗暴但有效的方案 (per-char 高亮需 wrap-aware 映射, 后续 v2)
    // R26-3-fixes: chat_line_map - visual line index -> LineInfo { msg_idx, text }
    // 选区按视觉行粒度: 拖到哪一行就只高亮那一行, 复制按行拼 text
    let mut chat_line_map: Vec<LineInfo> = Vec::new();
    for (msg_idx, msg) in app.chat_history.iter().enumerate() {
        // 整块 push 之前把字符串 owned 化,避免后续循环借用问题
        let raw = msg.content.clone();
        // W2.6 修: 过滤 LLM 自己生成的 "—— R19 认知循环 ——" 段
        // (LLM 看到 system prompt 提到 R19 后,有时候会自己编这段)
        let raw = strip_r19_meta(&raw);
        // R26-3-fixes: 不要在外层 split_think — 助手分支要 thinking + rest 分离,
        //  若外层 split 后 content 已无 〰think〰 标签, 助手分支的 split_think(&content) 永远返空,
        //  thinking fold (Ctrl+O) 就会闪一下不见 (raw content 短暂可见, 然后变空).
        // 修复: 外层只 strip_r19_meta, 让各分支根据需要 split (assistant 用, user 不用).
        match msg.role.as_str() {
            "user" => {
                // user 气泡 (借鉴 Claude Code UserPromptMessage):
                // - 整块 primary bg + black fg 反色 (金卡)
                // - 右 padding 1
                // - 前缀 `❯ ` (Unicode U+276F = 重音号, Claude Code 风格)
                // user 不可能有 〰think〰 标签, 直接用 raw
                let line_idx = chat_line_map.len();
                chat_line_map.push(LineInfo {
                    msg_idx,
                    text: raw.clone(),
                    prefix_cols: unicode_width::UnicodeWidthStr::width(" ❯ ") as u16,
                });
                let sel_range = selection_char_range_for_line(app, line_idx);
                let mut spans = vec![Span::styled(
                    " ❯ ",
                    Style::default()
                        .fg(style.bg)
                        .bg(style.primary)
                        .add_modifier(Modifier::BOLD),
                )];
                let base_text_style = Style::default()
                    .fg(style.primary)
                    .bg(style.bg)
                    .add_modifier(Modifier::BOLD);
                spans.extend(styled_with_selection(&raw, base_text_style, sel_range));
                spans.push(Span::styled("  ", Style::default().bg(style.bg)));
                lines.push(Line::from(spans));
            }
            "assistant" => {
                // AI 气泡:
                // - 左边框 `▌` (accent 色)
                // - 内容用 accent 色
                // - thinking 链根据 thinking_expanded 决定
                let (think, rest) = split_think(&raw);
                if !think.is_empty() {
                    if app.thinking_expanded {
                        // 展开: 显示完整 thinking
                        let line_idx = chat_line_map.len();
                        chat_line_map.push(LineInfo {
                            msg_idx,
                            text: "∴ Thinking".to_string(),
                            prefix_cols: unicode_width::UnicodeWidthStr::width("   ∴ Thinking ")
                                as u16,
                        });
                        let sel_range = selection_char_range_for_line(app, line_idx);
                        let base_text_style = Style::default()
                            .fg(style.dim)
                            .add_modifier(Modifier::ITALIC);
                        // R28: displayed text = "   ∴ Thinking " (14 chars); chat_line_map.text = "∴ Thinking" (11 chars).
                        // prefix = "   " (3 cols, unselectable), middle = " ∴ Thinking " (the displayed text minus prefix).
                        // sel_range 在 chat_line_map.text 维度, 所以 prefix_cols = 3 chars 始终未选.
                        let mut spans = vec![Span::styled("   ", base_text_style)];
                        // 选中 chat_line_map.text 的 [s..e) 字符 — 跟 display chars [3+s..3+e] 对齐
                        // displayed text 的 11 个 text 字符 + trailing 1 space (不在 text 里, 永远不选)
                        // 简化: 把 trailing space 也视作 text 末 (s 自动 clamp 到 11, after = "" + space)
                        let display_text = "   ∴ Thinking ";
                        let display_text_chars: Vec<char> = display_text.chars().collect();
                        // sel_range 映射到 display: shift by +3 (prefix)
                        let shifted = sel_range.map(|(s, e)| (s + 3, e + 3));
                        let total_d = display_text_chars.len();
                        let adj = shifted
                            .map(|(s, e)| (s.min(total_d), e.min(total_d)))
                            .filter(|&(s, e)| e > s);
                        spans.extend(styled_with_selection(display_text, base_text_style, adj));
                        lines.push(Line::from(spans));
                        for tl in think.lines() {
                            let line_idx = chat_line_map.len();
                            chat_line_map.push(LineInfo {
                                msg_idx,
                                text: tl.to_string(),
                                prefix_cols: unicode_width::UnicodeWidthStr::width("     ") as u16,
                            });
                            let sel_range = selection_char_range_for_line(app, line_idx);
                            let base_text_style = Style::default()
                                .fg(style.dim)
                                .add_modifier(Modifier::ITALIC);
                            let mut spans = vec![Span::styled("     ", base_text_style)];
                            // sel_range 是 text 维度; display 加 5 chars prefix
                            let display = format!("     {}", tl);
                            let shifted = sel_range.map(|(s, e)| (s + 5, e + 5));
                            let total_d = display.chars().count();
                            let adj = shifted
                                .map(|(s, e)| (s.min(total_d), e.min(total_d)))
                                .filter(|&(s, e)| e > s);
                            spans.extend(styled_with_selection(&display, base_text_style, adj));
                            lines.push(Line::from(spans));
                        }
                    } else {
                        // 折叠: 只显示 `∴ Thinking <Ctrl-O>`
                        let line_idx = chat_line_map.len();
                        chat_line_map.push(LineInfo {
                            msg_idx,
                            text: "∴ Thinking (折叠)".to_string(),
                            prefix_cols: unicode_width::UnicodeWidthStr::width(
                                "   ∴ Thinking… <Ctrl+O 展开>",
                            ) as u16,
                        });
                        let sel_range = selection_char_range_for_line(app, line_idx);
                        let base_text_style = Style::default()
                            .fg(style.dim)
                            .add_modifier(Modifier::ITALIC);
                        // R28: displayed text = "   ∴ Thinking… <Ctrl+O 展开>" (26 chars);
                        // chat_line_map.text = "∴ Thinking (折叠)" (15 chars).
                        // 它们字符内容不同 (∴ Thinking… vs ∴ Thinking (折叠)), 所以这里整个 span 视为一体,
                        // sel_range 按"text 维度比例"映射回 display.
                        let display = "   ∴ Thinking… <Ctrl+O 展开>";
                        let text_len = "∴ Thinking (折叠)".chars().count();
                        let display_text_len = 13; // "∴ Thinking… " 部分
                        let total_d = display.chars().count();
                        let adj = sel_range
                            .map(|(s, e)| {
                                let sp = if text_len > 0 {
                                    (s * display_text_len / text_len) + 3
                                } else {
                                    3
                                };
                                let ep = if text_len > 0 {
                                    (e * display_text_len / text_len) + 3
                                } else {
                                    3
                                };
                                (sp.min(total_d), ep.min(total_d))
                            })
                            .filter(|&(s, e)| e > s);
                        let mut spans = Vec::new();
                        spans.extend(styled_with_selection(display, base_text_style, adj));
                        lines.push(Line::from(spans));
                    }
                    chat_line_map.push(LineInfo {
                        msg_idx,
                        text: String::new(),
                        prefix_cols: 0,
                    });
                    lines.push(Line::from(""));
                }
                // AI 回复主体 (to_owned 避免 borrow 跨块)
                for rl in rest.lines() {
                    let line_idx = chat_line_map.len();
                    chat_line_map.push(LineInfo {
                        msg_idx,
                        text: rl.to_string(),
                        prefix_cols: unicode_width::UnicodeWidthStr::width(" ▌ ") as u16,
                    });
                    let sel_range = selection_char_range_for_line(app, line_idx);
                    let mut spans = vec![Span::styled(
                        " ▌ ",
                        Style::default()
                            .fg(style.accent)
                            .add_modifier(Modifier::BOLD),
                    )];
                    let base_text_style = Style::default().fg(style.accent);
                    spans.extend(styled_with_selection(&rl, base_text_style, sel_range));
                    lines.push(Line::from(spans));
                }
            }
            "system" => {
                // 系统消息不包含思考链, 直接用 raw
                let line_idx = chat_line_map.len();
                chat_line_map.push(LineInfo {
                    msg_idx,
                    text: raw.clone(),
                    prefix_cols: unicode_width::UnicodeWidthStr::width(" · ") as u16,
                });
                let sel_range = selection_char_range_for_line(app, line_idx);
                let base_text_style = Style::default().fg(style.dim);
                let display = format!(" · {}", raw);
                // sel_range 是 raw 维度; display 偏移 +3 (prefix " · ")
                let shifted = sel_range.map(|(s, e)| (s + 3, e + 3));
                let total_d = display.chars().count();
                let adj = shifted
                    .map(|(s, e)| (s.min(total_d), e.min(total_d)))
                    .filter(|&(s, e)| e > s);
                lines.push(Line::from(styled_with_selection(
                    &display,
                    base_text_style,
                    adj,
                )));
            }
            _ => {
                // 未知 role, 也直接用 raw
                let line_idx = chat_line_map.len();
                chat_line_map.push(LineInfo {
                    msg_idx,
                    text: raw.clone(),
                    prefix_cols: unicode_width::UnicodeWidthStr::width(" ? ") as u16,
                });
                let sel_range = selection_char_range_for_line(app, line_idx);
                let base_text_style = Style::default().fg(style.dim);
                let display = format!(" ? {}", raw);
                let shifted = sel_range.map(|(s, e)| (s + 3, e + 3));
                let total_d = display.chars().count();
                let adj = shifted
                    .map(|(s, e)| (s.min(total_d), e.min(total_d)))
                    .filter(|&(s, e)| e > s);
                lines.push(Line::from(styled_with_selection(
                    &display,
                    base_text_style,
                    adj,
                )));
            }
        }
        // 消息间 1 行空 (借鉴 Claude Code marginTop=1)
        chat_line_map.push(LineInfo {
            msg_idx,
            text: String::new(),
            prefix_cols: 0,
        });
        lines.push(Line::from(""));
    }

    // 末尾附 R19 认知循环 meta (system, 单独行)
    // (W2.3 修复: 不再重复 — backend::chat 返回的 content 已经包含 R19 meta,
    //  这里只处理 thinking_expanded 状态下的额外信息,避免重复)
    if app.processing {
        // W3 #1 流式: 显示 partial streaming_message (处理期累积的 chunks)
        // 这是 W2.4 异步化的延伸 — 用户能边等边看到 LLM 边生成的内容
        if let Some(ref streamed) = app.streaming_message {
            if !streamed.is_empty() {
                // 跟 assistant 历史消息一样的 ▌ 左边框, 但加 ⏳ 表示还在生成
                for rl in streamed.lines() {
                    lines.push(Line::from(vec![
                        Span::styled(
                            " ▌ ",
                            Style::default()
                                .fg(style.accent)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(rl.to_string(), Style::default().fg(style.accent)),
                        Span::styled(
                            " ⏳",
                            Style::default()
                                .fg(style.accent)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
                lines.push(Line::from(""));
            }
        }
        // R30 P4: 工具事件灰色行 (Call/Result)
        // 每个事件 1 行, 灰色 (style.dim), 不带左边框 (跟 system 提示对齐)
        for evt in app.tool_events.iter() {
            let text = crate::backend::format_tool_event(evt);
            lines.push(Line::from(Span::styled(
                format!("    {text}"),
                Style::default().fg(style.dim),
            )));
        }
    }
    if app.processing && app.streaming_message.is_none() {
        // W2.4 旧 spinner 路径 (W3 #1 前向兼容, 如果 streaming_message 还是 None 用旧 spinner)
        // W2.3 spinner: 等 LLM 期间显示 ⟳ 思考中
        let spinner_chars = ['⟳', '◐', '◑', '◒'];
        let frame = app.spinner_frame as usize % spinner_chars.len();
        let spinner = spinner_chars[frame];
        let dot_count = (app.spinner_frame / 4) as usize % 4;
        let dots = match dot_count {
            0 => "·",
            1 => "··",
            2 => "···",
            _ => "····",
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", spinner),
                Style::default()
                    .fg(style.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" 思考中 {}", dots),
                Style::default().fg(style.accent),
            ),
        ]));
        lines.push(Line::from(""));
    }

    // R26-3-fixes: 实时锚到底 + 反向 scroll 计算 (scroll_offset = 用户向上滚了多少行, y = max_scroll - scroll_offset)
    // - scroll_to_bottom=true (default): 锚到底 (新消息自动升窗)
    // - scroll_to_bottom=false: 用户脱离锁底后: 越向上滚 scroll_offset 越大, y 越小, 显示偏顶部 (更早内容)
    // - 滚轮 + PageUp/PageDown 方向巹对齐: += 5 = 向上滚 (看更早), -= 5 = 向下滚 (看更新/锁底)
    let inner_height = area.height.saturating_sub(2) as usize;
    let total = lines.len();
    let max_scroll = total.saturating_sub(inner_height) as u16;
    let scroll = compute_scroll_y(app.scroll_to_bottom, app.scroll_offset, max_scroll);
    let inner = block.inner(area);
    // R26-3-fixes: 写 chat_line_map 给 main.rs 的 mouse drag handler 用
    app.chat_line_map = chat_line_map;
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        area,
    );
    // R26-3-fixes: Scrollbar position 重映射
    // ratatui ScrollbarState::position 范围 [0, content-1]
    // Paragraph::scroll.y 范围 [0, max_scroll]
    // 两坐标系不对齐, 直接传 scroll → 底部 thumb 中段 (用户反馈: 上半部分)
    // 重映射: position = scroll * (content - 1) / max_scroll (线性)
    let sb_position = compute_scrollbar_position(scroll, max_scroll, total);
    let mut sb_state = ScrollbarState::new(total)
        .position(sb_position)
        .viewport_content_length(inner_height);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(style.accent))
            .track_style(Style::default().fg(style.dim)),
        inner,
        &mut sb_state,
    );
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &App, style: &ThemeStyle) {
    let cycle = crate::backend::cycle_count_load();
    // W3.4: token 显示 LLM 报数 + R19 自研双字段, 跟 backend::r19_token_used_load 并行.
    // 跟 bridge 顶 7 数字保持一致, 让用户一眼看到两条数对比 (LLM 报数 vs R19 启发式估算).
    let token_llm = crate::backend::token_used_load();
    let token_r19 = crate::backend::r19_token_used_load();
    let five_self = crate::backend::five_self_armed_label();
    let line = Line::from(vec![
        Span::styled(" ", Style::default().fg(style.dim)),
        Span::styled("● ", Style::default().fg(style.accent)),
        Span::styled(
            format!("主题={}", app.theme.label()),
            Style::default()
                .fg(style.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  ", Style::default().fg(style.dim)),
        Span::styled(format!("循环 #{}", cycle), Style::default().fg(style.dim)),
        Span::styled("  ·  ", Style::default().fg(style.dim)),
        Span::styled(
            format!("LLM令牌 {} / R19令牌 {}", token_llm, token_r19),
            Style::default().fg(style.dim),
        ),
        Span::styled("  ·  ", Style::default().fg(style.dim)),
        Span::styled(
            format!("5-Self {}", five_self),
            Style::default().fg(style.dim),
        ),
        Span::styled("  ·  ", Style::default().fg(style.dim)),
        // R27 C 方案: API 连通状态 (● 在线 / ✗ 离线)
        // host:port 从 base_url 提取, 让用户一眼看到后端 daemon 在哪 + 通不通
        Span::styled("  ·  ", Style::default().fg(style.dim)),
        Span::styled("API ", Style::default().fg(style.dim)),
        Span::styled(
            crate::backend::parse_host_port(&app.api_url).unwrap_or_else(|| "-".to_string()),
            Style::default().fg(if app.api_online {
                style.accent
            } else {
                style.dim
            }),
        ),
        Span::styled(
            if app.api_online { " ●" } else { " ✗" },
            Style::default()
                .fg(if app.api_online {
                    style.accent
                } else {
                    style.dim
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if app.thinking_expanded {
                "思考:展开"
            } else {
                "思考:折叠"
            },
            Style::default().fg(if app.thinking_expanded {
                style.accent
            } else {
                style.dim
            }),
        ),
    ]);
    // R26-3-fixes: 选区 / 复制反馈状态 (跟状态栏同行, 右侧追加)
    // - 选区存在: "已选 — [Ctrl+C 复制 · Esc 取消]"
    // - 复制成功: copy_feedback 文本 (0.5s 后自动消失)
    let now = std::time::Instant::now();
    let copy_active = app
        .copy_feedback
        .as_ref()
        .map(|(_, t)| now.duration_since(*t) < std::time::Duration::from_millis(500))
        .unwrap_or(false);

    let mut line = line;
    if copy_active {
        let (msg, _) = app.copy_feedback.as_ref().unwrap();
        line.spans
            .push(Span::styled("  · ", Style::default().fg(style.dim)));
        line.spans.push(Span::styled(
            msg.clone(),
            Style::default()
                .fg(style.primary)
                .add_modifier(Modifier::BOLD),
        ));
    } else if app.selection.is_some() {
        line.spans
            .push(Span::styled("  · ", Style::default().fg(style.dim)));
        // R28 char-level: 显示实际字符数 (跟 Ctrl+C 复制的一致), 零长度选区也有反馈
        let sel_count = compute_selection_char_count(app);
        line.spans.push(Span::styled(
            format!("已选 {} 字符 — [Ctrl+C 复制 · Esc 取消]", sel_count),
            Style::default().fg(style.accent),
        ));
    }
    f.render_widget(Paragraph::new(line), area);
}

fn render_input(f: &mut Frame, area: Rect, app: &App, style: &ThemeStyle) {
    // R26-3-fixes: focus 状态视觉提示 (静态, 不闪烁)
    // - input_focused=true: border 亮金 (accent) — 明显活跃
    // - input_focused=false: border 暗灰 (dim) — 明显不活跃
    // 不闪烁 (主人 2026-08-08 反馈: 一直闪太晃眼, 改静态区分即可)
    let border_color = if app.input_focused {
        style.accent
    } else {
        style.dim
    };
    let title_color = border_color;
    // 历史导航中显示 [历史 N/M] 提示
    let title_text = if let Some(idx) = app.history_idx {
        let total = app.input_history.len();
        format!(" 输入 (历史 {}/{} · ↑↓ 导航) ", idx + 1, total)
    } else {
        " 输入 (Enter 提交) ".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title_text, Style::default().fg(title_color)))
        .border_style(Style::default().fg(border_color))
        .border_type(style.border_type);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // R26-3-fixes: 删掉画块光标 (与 OS 光标位置错位, 双光标冗余).
    // 现在只画输入文字 + OS 光标 (自带闪烁), `set_cursor()` 仍保留给 IME 定位.
    let s: String = app.input_buf.iter().collect();
    let byte_cursor: usize = s
        .char_indices()
        .nth(app.input_cursor)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let (before, after) = s.split_at(byte_cursor);
    // 输入文字颜色: focus 时 accent, 失焦时 dim (提示用户输入不活跃)
    let text_color = if app.input_focused {
        style.accent
    } else {
        style.dim
    };
    let text = vec![Line::from(vec![
        Span::styled(" ▏", Style::default().fg(text_color)),
        Span::styled(before.to_string(), Style::default().fg(text_color)),
        Span::styled(after.to_string(), Style::default().fg(text_color)),
    ])];
    f.render_widget(Paragraph::new(text), inner);

    // R26-3-fixes: explicitly set OS cursor to input box, so IME window follows input,
    // not the default top-of-screen position (which squeezes the nav bar).
    // focus 时 OS 光标可见 (默认 ratatui 设过), 失焦时仍 set 位置但前端不渲染
    let cursor_x = inner.x + 2 + unicode_width::UnicodeWidthStr::width(before) as u16;
    let cursor_y = inner.y;
    f.set_cursor_position((cursor_x, cursor_y));
}

/// 过滤掉 `` 标签 (return (think, rest_without_think))
fn split_think(s: &str) -> (String, String) {
    if let Some(start) = s.find("<think>") {
        let before = &s[..start];
        if let Some(end) = s[start + 7..].find("</think>") {
            let think = &s[start + 7..start + 7 + end];
            let after = &s[start + 7 + end + 8..];
            let rest = format!("{}{}", before, after);
            return (think.to_string(), rest.trim().to_string());
        }
    }
    (String::new(), s.to_string())
}

/// W2.6 修: 过滤 LLM 自己生成的 R19 meta 段
/// (LLM 看到 system prompt 提到 R19 后,偶尔会自己编 "—— R19 认知循环 —— / v0.5 transferability=..." 这段)
/// R19 内部数据(cycle / V0.5 / verdicts)统一在 status bar 显示,对话气泡里不要再出现
/// 兼容分隔符: —— (em dash U+2014) / ─── (box drawing U+2500) / 没分隔符
fn strip_r19_meta(s: &str) -> String {
    // LLM 实际生成的内容模式:
    //   你好,有什么可以帮你的?
    //   —— R19 认知循环 ——     ← 找 "R19 认知循环" 这个最稳的子串
    //   v0.5 transferability=1.000 · verdicts=1 · cycle#3
    //
    // 策略: 找 "R19 认知循环", 从它所在行 (含前导空行) 开始到末尾都砍掉
    if let Some(idx) = s.find("R19 认知循环") {
        // 找 idx 之前最近的 \n (即 marker 所在行的行首)
        let line_start = s[..idx].rfind('\n').map(|i| i + 1).unwrap_or(0);
        return s[..line_start].trim_end().to_string();
    }
    s.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        compute_scroll_y, compute_scrollbar_position, split_think, strip_r19_meta,
        strip_think_tags_simple,
    };
    #[test]
    fn compute_scrollbar_position_top_returns_zero() {
        // R26-3-fixes: scroll=0 -> position=0 (thumb 顶)
        assert_eq!(compute_scrollbar_position(0, 70, 100), 0);
        assert_eq!(compute_scrollbar_position(0, 0, 30), 0); // max_scroll=0 edge
        assert_eq!(compute_scrollbar_position(0, 70, 0), 0); // total=0 edge
    }

    #[test]
    fn compute_scrollbar_position_bottom_returns_max() {
        // R26-3-fixes: scroll=max_scroll -> position=content-1 (thumb 底)
        assert_eq!(compute_scrollbar_position(70, 70, 100), 99);
        assert_eq!(compute_scrollbar_position(5, 5, 10), 9);
    }

    #[test]
    fn compute_scrollbar_position_linear_mapping() {
        // R26-3-fixes: 线性映射 (中段 -> 中段 thumb)
        // scroll=35, max_scroll=70, total=100 -> position = 35 * 99 / 70 = 49
        assert_eq!(compute_scrollbar_position(35, 70, 100), 49);
        // scroll=17, max_scroll=70, total=100 -> position = 17 * 99 / 70 = 24
        assert_eq!(compute_scrollbar_position(17, 70, 100), 24);
    }

    #[test]
    fn compute_scroll_y_lock_bottom_returns_max() {
        // R26-3-fixes: 锁底返回 max_scroll
        assert_eq!(compute_scroll_y(true, 0, 100), 100);
        assert_eq!(compute_scroll_y(true, 50, 100), 100);
        assert_eq!(compute_scroll_y(true, 200, 100), 100);
    }

    #[test]
    fn compute_scroll_y_zero_offset_returns_max() {
        // R26-3-fixes: 脱离锁底, offset=0 返回 max_scroll (等同锁底效果)
        assert_eq!(compute_scroll_y(false, 0, 100), 100);
    }

    #[test]
    fn compute_scroll_y_offset_inverts_direction() {
        // R26-3-fixes: offset 越大 y 越小 (反向计算)
        // offset = 5 → 向上滚 5 行 → y 减小 5
        assert_eq!(compute_scroll_y(false, 5, 100), 95);
        // offset = max_scroll → 最顶
        assert_eq!(compute_scroll_y(false, 100, 100), 0);
    }

    #[test]
    fn compute_scroll_y_offset_saturates_to_zero() {
        // R26-3-fixes: offset saturate, 不会下溢到负
        assert_eq!(compute_scroll_y(false, 200, 100), 0);
        assert_eq!(compute_scroll_y(false, u16::MAX, 100), 0);
    }

    // 2026-08-04 回归测试 (chuling via mavis): char→byte cursor 转换
    //   防止 input_buf 是 Vec<char> 但 s.split_at(byte) 时再次踩 byte index panic.
    //   模拟主 render_input 的 byte_cursor 计算逻辑, 验证中文/Emoji 都不 panic.
    #[test]
    fn char_to_byte_cursor_ascii_safe() {
        // "hi" cursor=1 → byte 1 (在 'i' 前)
        let s = "hi".to_string();
        let byte: usize = s.char_indices().nth(1).map(|(i, _)| i).unwrap_or(s.len());
        assert_eq!(byte, 1);
        let (b, a) = s.split_at(byte);
        assert_eq!(b, "h");
        assert_eq!(a, "i");
    }

    #[test]
    fn char_to_byte_cursor_cjk_does_not_panic() {
        // "晚上好" cursor=1 → byte 3 (在 '上' 前, 不是 byte 1!)
        // 修复前: split_at(1) → panic
        // 修复后: split_at(3) → "晚" + "上好"
        let s = "晚上好".to_string();
        let byte: usize = s.char_indices().nth(1).map(|(i, _)| i).unwrap_or(s.len());
        assert_eq!(byte, 3, "中文 1 char = 3 bytes, byte cursor 必须在 3");
        let (b, a) = s.split_at(byte);
        assert_eq!(b, "晚");
        assert_eq!(a, "上好");
    }

    #[test]
    fn char_to_byte_cursor_emoji_does_not_panic() {
        // Emoji 是 4 bytes, 验证 char_indices 正确
        let s = "a😀b".to_string();
        // "a" (1 byte) + "😀" (4 bytes) + "b" (1 byte) = 6 bytes
        // cursor=1 (在 '😀' 前) → byte 1
        let byte1: usize = s.char_indices().nth(1).map(|(i, _)| i).unwrap_or(s.len());
        assert_eq!(byte1, 1);
        // cursor=2 (在 'b' 前) → byte 5
        let byte2: usize = s.char_indices().nth(2).map(|(i, _)| i).unwrap_or(s.len());
        assert_eq!(byte2, 5);
        let (b, a) = s.split_at(byte2);
        assert_eq!(b, "a😀");
        assert_eq!(a, "b");
    }

    #[test]
    fn char_to_byte_cursor_at_end_returns_byte_len() {
        // cursor 越界 → 回退到 s.len() (字符串末尾)
        let s = "晚上好".to_string();
        let byte: usize = s.char_indices().nth(99).map(|(i, _)| i).unwrap_or(s.len());
        assert_eq!(byte, s.len());
        assert_eq!(byte, 9); // 3 chars × 3 bytes
    }

    #[test]
    fn split_think_basic() {
        let (t, r) = split_think("<think>reasoning</think>嗨!");
        assert_eq!(t, "reasoning");
        assert_eq!(r, "嗨!");
    }

    #[test]
    fn split_think_no_think() {
        let (t, r) = split_think("hello");
        assert_eq!(t, "");
        assert_eq!(r, "hello");
    }

    #[test]
    fn strip_simple() {
        assert_eq!(strip_think_tags_simple("<think>x</think>ok"), "ok");
    }

    // W2.6: strip_r19_meta 必须能砍掉 em dash 变体 (LLM 实际生成的)
    #[test]
    fn strip_r19_em_dash() {
        let input = "你好,有什么可以帮你的?\n\n—— R19 认知循环 ——\nv0.5 transferability=1.000 · verdicts=1 · cycle#3";
        let out = strip_r19_meta(input);
        assert!(!out.contains("R19"));
        assert!(!out.contains("transferability"));
        assert!(out.contains("你好"));
    }

    #[test]
    fn strip_r19_no_meta() {
        let input = "普通回复, 没有 R19 段";
        let out = strip_r19_meta(input);
        assert_eq!(out, "普通回复, 没有 R19 段");
    }

    // R26-3-fixes 回归测试: 验证 split_think 在实际 LLM 响应格式下能正确分离 think + rest
    // (MiniMax-M3 真实返回: 思考内容 一般 < 1k 字符, 跟在 思考 内容后跟回复, 以 \n\n 分隔)
    #[test]
    fn split_think_minimax_real_format() {
        let real_response = "<think>The user said \"hi\" - a simple greeting. I should respond warmly and conversationally.</think>\n\nHi there! \u{1f44b} How can I help you today?";
        let (think, rest) = split_think(real_response);
        assert!(
            think.contains("simple greeting"),
            "think 应含 MiniMax 思考内容, 实际: {think}"
        );
        assert!(
            rest.contains("Hi there"),
            "rest 应含 Hi there, 实际: {rest}"
        );
        assert!(!rest.contains("思考"), "rest 不应含思考标签");
    }

    // R26-3-fixes 回归测试: 验证 chat_history 推送顺序不带双切
    // (双切 bug: 外层 split_think 后 content 已无 思考 标签, 助手分支 split_think(&content) 永远返空)
    // 这个测试验证 split_think 幂等性 — 再次调应该返同样结果 (空 think)
    #[test]
    fn split_think_idempotent_on_empty() {
        let (t1, r1) = split_think("a normal message");
        assert_eq!(t1, "");
        assert_eq!(r1, "a normal message");
        let (t2, r2) = split_think(&r1);
        assert_eq!(t2, "");
        assert_eq!(r2, "a normal message");
    }

    #[test]
    fn strip_r19_box_drawing() {
        // box drawing 变体 (理论上 LLM 不会用, 但兼容)
        let input = "前文\n─── R19 认知循环 ───\nv0.5=1.000";
        let out = strip_r19_meta(input);
        assert!(!out.contains("R19"));
        assert_eq!(out, "前文");
    }
}

/// 完整 strip 思考链 (UI 不用, 留作兼容)
#[allow(dead_code)]
pub fn strip_think_tags_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        if let Some(end) = rest[start + 7..].find("</think>") {
            rest = &rest[start + 7 + end + 8..];
        } else {
            return out.trim().to_string();
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}
