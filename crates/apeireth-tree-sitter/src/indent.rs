//! §5 indent — 缩进检测 (1:1 翻译 EditorConfig `.editorconfig` 规则).
//!
//! **核心 API**: [`detect_indent`] (输入 source + Language, 返 IndentStyle).
//!
//! **状态**: ⚠️ skeleton — R20 阶段 5 估补, 真实缩进统计留 R20 阶段 4 续.
//! 当前返 `TreeSitterError::NotImplemented("indent")` (per O-5 不假装).
//!
//! **设计**:
//! - `IndentStyle::Tab` (Go/Rust 历史上 Go 用 tab, Rust 用 4 空格, skeleton 阶段 Go 默认 Tab)
//! - `IndentStyle::Space { size: u8 }` (per language 默认: Rust/TS=2/4, Python=4, Bash/Yaml/JSON=2)
//! - 1:1 翻译 EditorConfig `indent_style` + `indent_size` 字段
//!
//! **R20 阶段 4 续**: 真实 grammar 接入后, detect_indent 走 AST 缩进统计 (per NodeKind::Class/Function 行首缩进).

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{Language, TreeSitterError, TreeSitterResult};

// ============================================================================
// IndentStyle (1:1 翻译 EditorConfig `.editorconfig` indent_style + indent_size)
// ============================================================================

/// 缩进规则 (per §5 indent, 1:1 翻译 EditorConfig `.editorconfig` `indent_style` + `indent_size`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndentStyle {
    /// Tab 缩进
    Tab,
    /// 空格缩进 (size 由 `indent_size` 决定, 默认 4)
    Space {
        /// 缩进空格数 (per language 默认: Rust/Go=4, Python=4, Bash=2, Yaml=2, JSON=2)
        size: u8,
    },
}

impl IndentStyle {
    /// per language 默认缩进 (1:1 翻译 PEP 8 / gofmt / rustfmt 行业惯例).
    ///
    /// - Rust: 4 空格 (rustfmt default)
    /// - TypeScript / JavaScript: 2 空格 (prettier default)
    /// - Python: 4 空格 (PEP 8)
    /// - Go: Tab (gofmt 强制)
    /// - Bash: 2 空格 (shellcheck 建议)
    /// - Yaml: 2 空格 (YAML 行业惯例)
    /// - Json: 2 空格 (prettier default)
    #[must_use]
    pub fn default_for(language: Language) -> Self {
        match language {
            Language::Rust => IndentStyle::Space { size: 4 },
            Language::TypeScript | Language::JavaScript => IndentStyle::Space { size: 2 },
            Language::Python => IndentStyle::Space { size: 4 },
            Language::Go => IndentStyle::Tab,
            Language::Bash => IndentStyle::Space { size: 2 },
            Language::Yaml => IndentStyle::Space { size: 2 },
            Language::Json => IndentStyle::Space { size: 2 },
        }
    }

    /// EditorConfig 字符串 (1:1 翻译 `.editorconfig` `indent_style` + `indent_size`).
    #[must_use]
    pub fn as_editorconfig(&self) -> String {
        match self {
            IndentStyle::Tab => "indent_style = tab".to_string(),
            IndentStyle::Space { size } => format!("indent_style = space\nindent_size = {size}"),
        }
    }
}

impl std::fmt::Display for IndentStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndentStyle::Tab => f.write_str("tab"),
            IndentStyle::Space { size } => write!(f, "space:{size}"),
        }
    }
}

// ============================================================================
// §5 核心 API: detect_indent (skeleton 阶段返 NotImplemented, R20 阶段 4 续真实统计)
// ============================================================================

/// 缩进检测 (1:1 翻译 EditorConfig `.editorconfig` 推断 + tree-sitter AST 缩进统计).
///
/// **skeleton 阶段**: 返 `TreeSitterError::NotImplemented("indent")`.
/// R20 阶段 4 续: bash/typescript/python/rust 4 grammar 接入后, 此函数:
/// 1. 解析源文件第 1 个非空非注释行的行首字符
/// 2. 若以 `\t` 开头 → Tab
/// 3. 若以 N 个空格开头 → Space { size: N }
/// 4. 兜底用 `IndentStyle::default_for(language)`
///
/// # Errors
///
/// - `IndentDetectionFailed` — 文件非 UTF-8 (实际 rust 字符串都是 UTF-8, 留给 R20 阶段 4 续真实 IO 时校验)
/// - `NotImplemented` — skeleton 阶段未实现
pub fn detect_indent(_source: &str, language: Language) -> TreeSitterResult<IndentStyle> {
    // skeleton 阶段返 NotImplemented
    warn!(
        language = language.as_str(),
        default = %IndentStyle::default_for(language),
        "detect_indent: skeleton 阶段未实现, R20 阶段 4 续真实 AST 缩进统计"
    );
    Err(TreeSitterError::NotImplemented("indent"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indent_default_for_languages() {
        // 8 语言全部有 default, 不返 panic
        for &lang in crate::SUPPORTED_LANGUAGES {
            let style = IndentStyle::default_for(lang);
            assert!(!format!("{style}").is_empty(), "{lang:?} default indent 不能为空");
        }
    }

    #[test]
    fn test_indent_rust_default_is_4_space() {
        assert_eq!(IndentStyle::default_for(Language::Rust), IndentStyle::Space { size: 4 });
    }

    #[test]
    fn test_indent_go_default_is_tab() {
        assert_eq!(IndentStyle::default_for(Language::Go), IndentStyle::Tab);
    }

    #[test]
    fn test_indent_python_default_is_4_space() {
        assert_eq!(IndentStyle::default_for(Language::Python), IndentStyle::Space { size: 4 });
    }

    #[test]
    fn test_indent_editorconfig_string() {
        assert_eq!(IndentStyle::Tab.as_editorconfig(), "indent_style = tab");
        assert_eq!(
            IndentStyle::Space { size: 4 }.as_editorconfig(),
            "indent_style = space\nindent_size = 4"
        );
    }

    #[test]
    fn test_detect_indent_skeleton_returns_not_implemented() {
        let result = detect_indent("fn main() {}", Language::Rust);
        assert!(matches!(result, Err(TreeSitterError::NotImplemented("indent"))));
    }
}
