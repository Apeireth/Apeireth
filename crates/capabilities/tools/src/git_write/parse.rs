//! `git_write` 的**词法面**（段切分 / 提词）—— 从 `git_write.rs` **纯搬移**过来。
//!
//! 存在理由：`git_write.rs` 在 size 棘轮基线上（`arch-baseline.txt`，0 容差）
//! ⇒ 任何净增都会让 `arch_guard` 变红；棘轮文件**不归我改** ⇒ 正路是**让本文件净减**。
//!
//! `src/git_write.rs` + `src/git_write/parse.rs` 是 Rust 2018 的标准模块布局
//! ⇒ `mod parse;` 自动解析 ⇒ **不需要**改 `lib.rs`（别人的文件）。

use super::refuse_morph;

/// 段切分: 只认 `&&` / `;` / 换行为段分隔; shell 面 (管道/重定向/插值/分组)
/// 拒绝即帧 (本工具不经 shell, 不给"看起来像 shell"的幻觉空间)。
pub(super) fn split_command_segments(command: &str) -> Result<Vec<String>, String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(ch) = chars.next() {
        match quote {
            Some(open) => {
                current.push(ch);
                if ch == open {
                    quote = None;
                }
            }
            None => match ch {
                '\'' | '"' => {
                    quote = Some(ch);
                    current.push(ch);
                }
                ';' | '\n' | '\r' => segments.push(std::mem::take(&mut current)),
                '&' => {
                    if chars.peek() == Some(&'&') {
                        chars.next();
                        segments.push(std::mem::take(&mut current));
                    } else {
                        return Err(refuse_morph(
                            "单个 & (后台/句柄面) 不入本工具: 段分隔只认 && 与 ;",
                        ));
                    }
                }
                '|' | '<' | '>' => {
                    return Err(refuse_morph(
                        "管道/重定向面不入本工具: 命令按结构化参数执行 git, 不经 shell",
                    ))
                }
                '`' | '$' | '(' | ')' | '{' | '}' => {
                    return Err(refuse_morph(format!(
                        "shell 元字符 {ch:?} 不入本工具: 不做插值/分组/展开 (引号内按字面参数)"
                    )))
                }
                _ => current.push(ch),
            },
        }
    }
    if quote.is_some() {
        return Err(refuse_morph("引号未闭合: 命令文本无法按参数面解析"));
    }
    segments.push(current);
    Ok(segments)
}

/// 段内提词: 引号成组去壳; 引号内字面保留 (无转义/无插值); 空引号仍是词。
pub(super) fn tokenize_segment(segment: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    for ch in segment.chars() {
        match quote {
            Some(open) => {
                if ch == open {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            None => match ch {
                '\'' | '"' => {
                    quote = Some(ch);
                    started = true;
                }
                whitespace if whitespace.is_whitespace() => {
                    if started || !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                        started = false;
                    }
                }
                _ => {
                    current.push(ch);
                    started = true;
                }
            },
        }
    }
    if quote.is_some() {
        return Err(refuse_morph("引号未闭合: 命令文本无法按参数面解析"));
    }
    if started || !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}
