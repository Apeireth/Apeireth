//! 流式 Markdown 渲染: 标题 / 列表 / 引用 / 行内样式 / 围栏代码高亮 /
//! 工具卡片 (工具名 + 状态徽章 + 耗时)。
//!
//! 输出为带样式的行, 由面板层排版; 流式累积文本每帧重渲, 因此解析保持
//! 纯函数、O(行) 开销。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::state::{ToolCard, ToolStatus};
use crate::theme::PaletteTokens;

/// 关键字集 (多语言常见词, 供代码高亮着色)。
const KEYWORDS: &[&str] = &[
    "async", "await", "break", "case", "catch", "class", "const", "continue", "def", "do", "else",
    "enum", "export", "extends", "false", "fn", "for", "func", "if", "impl", "import", "in", "let",
    "match", "mod", "move", "mut", "None", "Ok", "Err", "package", "pub", "ref", "return", "self",
    "Self", "Some", "static", "struct", "super", "trait", "true", "try", "type", "use", "var",
    "where", "while", "with", "yield",
];

/// 把 Markdown 文本渲染为带样式行 (行内不折行, 由面板层 wrap)。
pub fn render_markdown(text: &str, palette: &PaletteTokens) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    for raw in text.split('\n') {
        let trimmed = raw.trim_start();
        if trimmed.starts_with("```") {
            // 围栏开合 + 语言标签行。
            if in_code_block {
                lines.push(Line::from(Span::styled(
                    "  └── code ──",
                    Style::default().fg(palette.panel_edge),
                )));
            } else {
                let lang = trimmed.trim_start_matches('`').trim();
                let label = if lang.is_empty() {
                    "code".to_string()
                } else {
                    format!("code · {lang}")
                };
                lines.push(Line::from(vec![
                    Span::styled("  ┌── ", Style::default().fg(palette.panel_edge)),
                    Span::styled(
                        label,
                        Style::default()
                            .fg(palette.glow_secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            lines.push(Line::from(highlight_code(raw, palette)));
            continue;
        }
        lines.push(render_prose_line(raw, palette));
    }
    lines
}

/// 围栏代码行 → 语法着色 spans (注释 / 字符串 / 数字 / 关键字)。
pub fn highlight_code(line: &str, palette: &PaletteTokens) -> Vec<Span<'static>> {
    if let Some(comment_at) = line.find("//") {
        let (code, comment) = line.split_at(comment_at);
        let mut spans = tokenize_code(code, palette);
        spans.push(Span::styled(
            comment.to_string(),
            Style::default().fg(palette.code_comment),
        ));
        return spans;
    }
    tokenize_code(line, palette)
}

/// 极简词法着色: 字符串 / 数字 / 关键字 / 其余合并成朴素 span。
fn tokenize_code(code: &str, palette: &PaletteTokens) -> Vec<Span<'static>> {
    let plain_style = Style::default().fg(palette.code_plain);
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut plain = String::new();
    let chars: Vec<char> = code.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch == '"' || ch == '\'' {
            // 字符串字面量 (到配对引号)。
            if !plain.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut plain), plain_style));
            }
            let mut literal = String::new();
            literal.push(ch);
            index += 1;
            let mut escaped = false;
            while index < chars.len() {
                let inner = chars[index];
                literal.push(inner);
                index += 1;
                if inner == ch && !escaped {
                    break;
                }
                escaped = inner == '\\' && !escaped;
            }
            spans.push(Span::styled(
                literal,
                Style::default().fg(palette.code_string),
            ));
        } else if ch.is_ascii_digit() {
            // 数字字面量。
            if !plain.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut plain), plain_style));
            }
            let mut literal = String::new();
            while index < chars.len()
                && (chars[index].is_ascii_digit() || chars[index] == '.' || chars[index] == '_')
            {
                literal.push(chars[index]);
                index += 1;
            }
            spans.push(Span::styled(
                literal,
                Style::default().fg(palette.code_number),
            ));
        } else if ch.is_ascii_alphabetic() || ch == '_' {
            // 标识符: 命中关键字单独着色, 否则并入朴素段。
            let mut word = String::new();
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                word.push(chars[index]);
                index += 1;
            }
            if KEYWORDS.contains(&word.as_str()) {
                if !plain.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut plain), plain_style));
                }
                spans.push(Span::styled(
                    word,
                    Style::default().fg(palette.code_keyword),
                ));
            } else {
                plain.push_str(&word);
            }
        } else {
            plain.push(ch);
            index += 1;
        }
    }
    if !plain.is_empty() {
        spans.push(Span::styled(plain, plain_style));
    }
    spans
}

/// 正文行 (标题 / 列表 / 引用 / 段落) → 带样式行。
fn render_prose_line(raw: &str, palette: &PaletteTokens) -> Line<'static> {
    let trimmed = raw.trim_start();
    if trimmed.starts_with('#') {
        let level = trimmed.chars().take_while(|ch| *ch == '#').count();
        let title = trimmed.trim_start_matches('#').trim_start();
        let style = Style::default()
            .fg(palette.glow_secondary)
            .add_modifier(Modifier::BOLD);
        let mut spans = vec![Span::styled(
            format!("{} ", "▰".repeat(level.min(3))),
            style,
        )];
        spans.extend(spans_for_inline(title, palette, style));
        return Line::from(spans);
    }
    if let Some(item) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
    {
        let mut spans = vec![Span::styled(
            "▸ ",
            Style::default().fg(palette.glow_primary),
        )];
        spans.extend(spans_for_inline(
            item,
            palette,
            Style::default().fg(palette.text_primary),
        ));
        return Line::from(spans);
    }
    if let Some(quoted) = trimmed.strip_prefix("> ") {
        let mut spans = vec![Span::styled(
            "▍ ",
            Style::default().fg(palette.glow_secondary),
        )];
        spans.extend(spans_for_inline(
            quoted,
            palette,
            Style::default().fg(palette.text_dim),
        ));
        return Line::from(spans);
    }
    Line::from(spans_for_inline(
        raw,
        palette,
        Style::default().fg(palette.text_primary),
    ))
}

/// 行内样式: `**粗体**` 与 `` `行内代码` ``。
fn spans_for_inline(text: &str, palette: &PaletteTokens, base: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let bold_open = rest.find("**");
        let code_open = rest.find('`');
        let next = match (bold_open, code_open) {
            (Some(b), Some(c)) => Some(if b < c {
                (b, Inline::Bold)
            } else {
                (c, Inline::Code)
            }),
            (Some(b), None) => Some((b, Inline::Bold)),
            (None, Some(c)) => Some((c, Inline::Code)),
            (None, None) => None,
        };
        let Some((at, inline)) = next else {
            spans.push(Span::styled(rest.to_string(), base));
            break;
        };
        if at > 0 {
            spans.push(Span::styled(rest[..at].to_string(), base));
        }
        let marker = if inline == Inline::Bold { "**" } else { "`" };
        let after = &rest[at + marker.len()..];
        if let Some(close) = after.find(marker) {
            let inner = &after[..close];
            let style = match inline {
                Inline::Bold => base.add_modifier(Modifier::BOLD),
                Inline::Code => Style::default().fg(palette.code_keyword),
            };
            spans.push(Span::styled(inner.to_string(), style));
            rest = &after[close + marker.len()..];
        } else {
            spans.push(Span::styled(rest[at..].to_string(), base));
            break;
        }
    }
    spans
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Inline {
    Bold,
    Code,
}

/// 工具卡片: 工具名 + 状态徽章 + 耗时 (两行卡片, 描边取舱体风)。
pub fn tool_card_lines(card: &ToolCard, palette: &PaletteTokens) -> Vec<Line<'static>> {
    let badge_color = match card.status {
        ToolStatus::Running => palette.glow_warn,
        ToolStatus::Ok => palette.glow_ok,
        ToolStatus::Failed => palette.glow_danger,
    };
    let duration = match card.duration_ms {
        Some(ms) if ms >= 1_000 => format!("{:.1}s", ms as f64 / 1_000.0),
        Some(ms) => format!("{ms}ms"),
        None => "…".to_string(),
    };
    vec![
        Line::from(vec![
            Span::styled("┌─ ", Style::default().fg(palette.tool_card)),
            Span::styled("⚙ ", Style::default().fg(palette.tool_card)),
            Span::styled(
                card.name.clone(),
                Style::default()
                    .fg(palette.tool_card)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default()),
            Span::styled(
                card.status.label(),
                Style::default()
                    .fg(badge_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {duration}"), Style::default().fg(palette.digit)),
        ]),
        Line::from(Span::styled(
            "└──────────────",
            Style::default().fg(palette.tool_card),
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 围栏代码块: 语言标签 + 关键字/字符串/注释着色齐备。
    #[test]
    fn code_blocks_get_highlighted() {
        let palette = PaletteTokens::deep_space_hud();
        let rendered = render_markdown(
            "前置段落\n```rust\nfn main() { let s = \"hi\"; } // 注释\n```\n后置段落",
            &palette,
        );
        let text = plain(&rendered);
        assert!(text.contains("code · rust"));
        assert!(text.contains("// 注释"));
        assert!(text.contains("后置段落"));

        // 关键字 / 字符串 / 注释拿到各自颜色。
        let keyword_span = rendered
            .iter()
            .flat_map(|line| line.spans.iter())
            .find(|span| span.content.as_ref() == "fn")
            .expect("关键字 span");
        assert_eq!(keyword_span.style.fg, Some(palette.code_keyword));
        let string_span = rendered
            .iter()
            .flat_map(|line| line.spans.iter())
            .find(|span| span.content.as_ref().contains("\"hi\""))
            .expect("字符串 span");
        assert_eq!(string_span.style.fg, Some(palette.code_string));
        let comment_span = rendered
            .iter()
            .flat_map(|line| line.spans.iter())
            .find(|span| span.content.as_ref().contains("//"))
            .expect("注释 span");
        assert_eq!(comment_span.style.fg, Some(palette.code_comment));
    }

    /// 工具卡片: 工具名 + 状态徽章 + 耗时三要素齐上, 配色随状态。
    #[test]
    fn tool_cards_carry_name_badge_and_duration() {
        let palette = PaletteTokens::deep_space_hud();
        let card = ToolCard {
            name: "tool.demo".to_string(),
            status: ToolStatus::Ok,
            started_ms: 0,
            duration_ms: Some(342),
        };
        let text = plain(&tool_card_lines(&card, &palette));
        assert!(text.contains("tool.demo"));
        assert!(text.contains("OK"));
        assert!(text.contains("342ms"));

        let failed = ToolCard {
            status: ToolStatus::Failed,
            duration_ms: Some(1_500),
            ..card
        };
        let text = plain(&tool_card_lines(&failed, &palette));
        assert!(text.contains("FAILED"));
        assert!(text.contains("1.5s"));

        let running = ToolCard {
            status: ToolStatus::Running,
            duration_ms: None,
            ..failed
        };
        let text = plain(&tool_card_lines(&running, &palette));
        assert!(text.contains("RUNNING"));
        assert!(text.contains("…"));
    }

    /// 正文结构: 标题 / 列表 / 引用 / 粗体 / 行内代码。
    #[test]
    fn prose_structure_renders_with_inline_styles() {
        let palette = PaletteTokens::deep_space_hud();
        let rendered = render_markdown(
            "# 标题\n- 条目 **重点** 尾巴\n> 引用一句\n段落里有 `行内代码`。",
            &palette,
        );
        let text = plain(&rendered);
        assert!(text.contains("▰ 标题"));
        assert!(text.contains("▸ 条目"));
        assert!(text.contains("▍ 引用一句"));
        assert!(text.contains("行内代码"));
        let bold = rendered
            .iter()
            .flat_map(|line| line.spans.iter())
            .find(|span| span.content.as_ref() == "重点")
            .expect("粗体 span");
        assert!(bold.style.add_modifier.contains(Modifier::BOLD));
    }
}
