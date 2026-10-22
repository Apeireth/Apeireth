//! `re.js`：JavaScript / Electron 源码静态观察（零新依赖）。
//!
//! 这是**词法级**分析：提取字符串字面量、模块引用、sourcemap 标记与动态执行
//! 迹象，不做完整 AST/作用域分析。能力边界写进 limitations，不冒充编译器。

use serde::{Deserialize, Serialize};

/// 单次 JS 静态观察结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsReport {
    /// 输入字节数（UTF-8）。
    pub bytes: usize,
    /// 提取到的字符串字面量（去重、保序、上限保护）。
    pub strings: Vec<String>,
    /// 模块引用（`require("x")` / `import ... from "x"` / `import("x")`）。
    pub imports: Vec<String>,
    /// `sourceMappingURL` 值。
    pub source_maps: Vec<String>,
    /// `eval(` / `new Function(` 出现次数。
    pub dynamic_eval: usize,
    /// 已知框架/运行时标记。
    pub markers: Vec<String>,
    /// 显式局限。
    pub limitations: Vec<String>,
}

/// 分析参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsOptions {
    /// 最大处理字符数（超出截断并记录局限）。
    pub max_chars: usize,
    /// 字符串上限。
    pub max_strings: usize,
}

impl Default for JsOptions {
    fn default() -> Self {
        Self {
            max_chars: 8 * 1024 * 1024,
            max_strings: 4096,
        }
    }
}

/// 分析一段 JavaScript 源码。
pub fn analyze_js(source: &str, options: &JsOptions) -> JsReport {
    let chars: Vec<char> = source.chars().take(options.max_chars).collect();
    let truncated = source.chars().count() > options.max_chars;

    let mut strings = Vec::new();
    let mut imports = Vec::new();
    let mut source_maps = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    let mut index = 0usize;
    while index < chars.len() {
        let current = chars[index];
        if current == '\'' || current == '"' || current == '`' {
            let quote = current;
            let start = index;
            index += 1;
            let mut literal = String::new();
            let mut terminated = false;
            while index < chars.len() {
                let ch = chars[index];
                if ch == '\\' {
                    if let Some(next) = chars.get(index + 1) {
                        literal.push(*next);
                        index += 2;
                        continue;
                    }
                    break;
                }
                if ch == quote {
                    terminated = true;
                    index += 1;
                    break;
                }
                if ch == '\n' && quote != '`' {
                    break;
                }
                literal.push(ch);
                index += 1;
            }
            if terminated && !literal.is_empty() && seen.insert(literal.clone()) {
                if strings.len() < options.max_strings {
                    strings.push(literal.clone());
                }
                if let Some(context) = context_before(&chars, start) {
                    if (context.contains("require(")
                        || context.contains("from ")
                        || context.contains("import("))
                        && looks_like_module(&literal)
                    {
                        imports.push(literal.clone());
                    }
                }
            }
            continue;
        }
        index += 1;
    }

    let text: String = chars.iter().collect();
    for (position, _) in text.match_indices("sourceMappingURL=") {
        let rest = &text[position + "sourceMappingURL=".len()..];
        let end = rest
            .find(|ch: char| ch.is_whitespace() || ch == '*' || ch == '"' || ch == '\'')
            .unwrap_or(rest.len());
        let value = rest[..end].trim();
        if !value.is_empty() {
            source_maps.push(value.to_string());
        }
    }
    source_maps.sort();
    source_maps.dedup();
    let mut markers = Vec::new();
    for marker in [
        "__webpack_require__",
        "webpackChunk",
        "Electron",
        "process.env.NODE_ENV",
        "module.exports",
        "exports.default",
    ] {
        if text.contains(marker) {
            markers.push(marker.to_string());
        }
    }
    let dynamic_eval =
        count_occurrences(&text, "eval(") + count_occurrences(&text, "new Function(");

    let mut limitations = vec![
        "lexical string scan only; no AST/scope analysis".to_string(),
        "minified identifiers are not resolved".to_string(),
    ];
    if truncated {
        limitations.push(format!("input truncated to {} chars", options.max_chars));
    }

    JsReport {
        bytes: source.len(),
        strings,
        imports,
        source_maps,
        dynamic_eval,
        markers,
        limitations,
    }
}

/// 取字符串字面量前 32 个字符的上下文。
fn context_before(chars: &[char], start: usize) -> Option<String> {
    let from = start.saturating_sub(32);
    Some(chars[from..start].iter().collect())
}

/// 模块说明符的粗略过滤：非空、无空白、长度合理。
fn looks_like_module(literal: &str) -> bool {
    !literal.is_empty()
        && literal.len() <= 512
        && !literal.chars().any(char::is_whitespace)
        && (literal.contains('/')
            || literal.contains('.')
            || literal.contains(':')
            || literal.starts_with('@'))
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_imports_strings_and_markers() {
        let source = r#"
            const fs = require("node:fs");
            import path from "node:path";
            const lazy = import("./lazy-module.js");
            //# sourceMappingURL=app.js.map
            const label = "hello world";
            if (process.env.NODE_ENV !== "production") { eval("1+1"); }
            // __webpack_require__ marker
        "#;
        let report = analyze_js(source, &JsOptions::default());
        assert!(report.imports.contains(&"node:fs".to_string()));
        assert!(report.imports.contains(&"node:path".to_string()));
        assert!(report.imports.contains(&"./lazy-module.js".to_string()));
        assert!(report.strings.contains(&"hello world".to_string()));
        assert_eq!(report.source_maps, vec!["app.js.map".to_string()]);
        assert_eq!(report.dynamic_eval, 1);
        assert!(report.markers.contains(&"__webpack_require__".to_string()));
        assert!(report.markers.contains(&"process.env.NODE_ENV".to_string()));
    }

    #[test]
    fn plain_strings_are_not_modules() {
        let report = analyze_js("const a = 'plain text';", &JsOptions::default());
        assert!(report.strings.contains(&"plain text".to_string()));
        assert!(report.imports.is_empty());
    }

    #[test]
    fn truncation_is_reported() {
        let source = "x".repeat(100);
        let options = JsOptions {
            max_chars: 10,
            ..JsOptions::default()
        };
        let report = analyze_js(&source, &options);
        assert!(report
            .limitations
            .iter()
            .any(|line| line.contains("truncated")));
    }
}
