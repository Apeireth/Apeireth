//! R125-4: MCP tools protocol — naming validation
//!
//! **拆分自原 `tools/mod.rs` (R65)**: `is_valid_tool_name` kebab-case 校验
//! 单独成 module, 借鉴 VCP `toolCallParser.js` 实现 (0 改语义).
//!
//! **不漂移 (主哲学锚 #1, 8 硬墙 #3)**:
//! - 0 改 `is_valid_tool_name` 签名 / 实现 (入口签名 0 改)
//! - 0 改 kebab-case 校验规则

/// **校验 tool name 为 kebab-case** (per VCP `toolCallParser.js` 借鉴)
///
/// **规则**:
/// - 非空, 不以 `-` 开头/结尾
/// - 字符仅 ASCII 小写字母 + 数字 + `-`
/// - 不允许连续 `--` (double-dash)
pub fn is_valid_tool_name(name: &str) -> bool {
    if name.is_empty() || name.starts_with('-') || name.ends_with('-') {
        return false;
    }
    let mut prev_dash = false;
    for c in name.chars() {
        if c == '-' {
            if prev_dash {
                return false;
            }
            prev_dash = true;
        } else {
            prev_dash = false;
            if !(c.is_ascii_lowercase() || c.is_ascii_digit()) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_tool_name_kebab() {
        assert!(is_valid_tool_name("echo"));
        assert!(is_valid_tool_name("summarize-text"));
        assert!(is_valid_tool_name("a-1-b-2"));
        assert!(!is_valid_tool_name(""));
        assert!(!is_valid_tool_name("-start"));
        assert!(!is_valid_tool_name("end-"));
        assert!(!is_valid_tool_name("CamelCase"));
        assert!(!is_valid_tool_name("under_score"));
        assert!(!is_valid_tool_name("double--dash"));
    }
}
