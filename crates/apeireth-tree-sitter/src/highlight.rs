//! §1 highlight — 8 语言语法高亮 (1:1 翻译 web-tree-sitter `highlighter.highlight()` JS API).
//!
//! **核心 API**: [`highlight`] (1:1 翻译 `highlighter.highlight(input, language) -> Vec<HighlightSpan>`).
//!
//! **状态**: ⚠️ skeleton — R20 阶段 5 估补, 真实 grammar 留 R20 阶段 4 续 (bash/typescript/python/rust 4 grammar).
//! 当前所有 8 语言都返 `TreeSitterError::NotImplemented("highlight")` (per O-5 不假装).
//!
//! **设计**:
//! - 8 Language 全部 hardcode 进 `SUPPORTED_LANGUAGES` (lib.rs)
//! - `HighlightKind` 12 类 (1:1 翻译 TextMate scope 名)
//! - `HighlightSpan { start, end, kind }` (1:1 翻译 web-tree-sitter `Highlight.startIndex/endIndex`)
//! - `HIGHLIGHT_MAX_TOKEN_LENGTH = 1024` (编译期 hardcode, 防止恶意超长 token)
//!
//! **R20 阶段 4 续**:
//! - bash: tree-sitter-bash = "0.23"
//! - typescript: tree-sitter-typescript = "0.23"
//! - python: tree-sitter-python = "0.23"
//! - rust: tree-sitter-rust = "0.23"
//! - 其它 4 (JavaScript/Go/Yaml/Json) 留 R21+

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{Language, TreeSitterError, TreeSitterResult, HIGHLIGHT_MAX_TOKEN_LENGTH};

// ============================================================================
// HighlightKind (12 类, 1:1 翻译 TextMate scope 名 + web-tree-sitter `Highlight` type)
// ============================================================================

/// 高亮 token 类别 (1:1 翻译 TextMate scope 名, 12 类, per Monaco Editor `Tokenization`).
///
/// 1:1 翻译 web-tree-sitter `highlight.query` 输出 + TextMate scope 约定.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HighlightKind {
    /// 关键字 (if/for/while/return 等)
    Keyword,
    /// 函数定义/调用
    Function,
    /// 类型名
    Type,
    /// 字符串字面量
    String,
    /// 数字字面量
    Number,
    /// 注释
    Comment,
    /// 操作符
    Operator,
    /// 变量名
    Variable,
    /// 常量
    Constant,
    /// import / use 路径
    Import,
    /// 标点
    Punctuation,
    /// 其它 (fallback)
    Other,
}

impl HighlightKind {
    /// TextMate scope 字符串 (1:1 翻译 vscode 高亮 scope).
    #[must_use]
    pub fn scope(&self) -> &'static str {
        match self {
            HighlightKind::Keyword => "keyword",
            HighlightKind::Function => "entity.name.function",
            HighlightKind::Type => "entity.name.type",
            HighlightKind::String => "string",
            HighlightKind::Number => "constant.numeric",
            HighlightKind::Comment => "comment",
            HighlightKind::Operator => "keyword.operator",
            HighlightKind::Variable => "variable",
            HighlightKind::Constant => "constant",
            HighlightKind::Import => "string.quoted.other",
            HighlightKind::Punctuation => "punctuation",
            HighlightKind::Other => "source",
        }
    }
}

/// 高亮 token 范围 (字节偏移 + 类别).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighlightSpan {
    /// 起始字节偏移 (UTF-8 byte offset, 1:1 翻译 web-tree-sitter `Highlight.startIndex/endIndex`).
    pub start: usize,
    /// 结束字节偏移 (exclusive).
    pub end: usize,
    /// 高亮类别.
    pub kind: HighlightKind,
}

impl HighlightSpan {
    /// 构造新高亮 token.
    #[must_use]
    pub fn new(start: usize, end: usize, kind: HighlightKind) -> Self {
        Self { start, end, kind }
    }

    /// token 长度 (bytes).
    #[must_use]
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// token 是否为空.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

// ============================================================================
// §1 核心 API: highlight (skeleton 阶段返 NotImplemented, R20 阶段 4 续真实 grammar)
// ============================================================================

/// 语法高亮 (1:1 翻译 web-tree-sitter `highlighter.highlight(input, language) -> Highlight[]`).
///
/// **skeleton 阶段**: 所有 8 语言都返 `TreeSitterError::NotImplemented("highlight")`.
/// R20 阶段 4 续: bash/typescript/python/rust 4 grammar 真实接入后, 此函数返回真实高亮 token 列表.
///
/// # Errors
///
/// - `LanguageNotSupported` — language 不在 SUPPORTED_LANGUAGES (理论上不会, 已编译期守门, 留给运行时校验)
/// - `NotImplemented` — skeleton 阶段未实现 (R20 阶段 4 续)
pub fn highlight(input: &str, language: Language) -> TreeSitterResult<Vec<HighlightSpan>> {
    // 1) 守门: 文件大小
    if input.len() as u64 > crate::MAX_FILE_SIZE_BYTES {
        return Err(TreeSitterError::FileSizeExceeded {
            size: input.len() as u64,
        });
    }
    // 2) 守门: 语言支持 (8 编译期 hardcode, 二次校验)
    if !crate::SUPPORTED_LANGUAGES.contains(&language) {
        return Err(TreeSitterError::LanguageNotSupported(language.as_str().to_string()));
    }
    // 3) skeleton 阶段返 NotImplemented
    warn!(
        language = language.as_str(),
        "highlight: skeleton 阶段未实现, R20 阶段 4 续真实 grammar 接入"
    );
    Err(TreeSitterError::NotImplemented("highlight"))
}

/// 高亮 token 长度守门 (per HIGHLIGHT_MAX_TOKEN_LENGTH 1024).
/// skeleton 阶段 helper, R20 阶段 4 续时用于过滤恶意超长 token.
#[must_use]
pub fn validate_span_length(span: &HighlightSpan) -> bool {
    span.len() <= HIGHLIGHT_MAX_TOKEN_LENGTH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_kind_scope_strings() {
        // 12 HighlightKind 都有非空 scope
        let kinds = [
            HighlightKind::Keyword,
            HighlightKind::Function,
            HighlightKind::Type,
            HighlightKind::String,
            HighlightKind::Number,
            HighlightKind::Comment,
            HighlightKind::Operator,
            HighlightKind::Variable,
            HighlightKind::Constant,
            HighlightKind::Import,
            HighlightKind::Punctuation,
            HighlightKind::Other,
        ];
        for k in kinds {
            assert!(!k.scope().is_empty(), "HighlightKind {:?} scope 不能为空", k);
        }
    }

    #[test]
    fn test_highlight_span_length() {
        let span = HighlightSpan::new(0, 100, HighlightKind::String);
        assert_eq!(span.len(), 100);
        assert!(!span.is_empty());

        let empty = HighlightSpan::new(5, 5, HighlightKind::Comment);
        assert!(empty.is_empty(), "start >= end 视为空");
    }

    #[test]
    fn test_highlight_skeleton_returns_not_implemented() {
        let result = highlight("fn main() {}", Language::Rust);
        assert!(matches!(result, Err(TreeSitterError::NotImplemented("highlight"))));
    }

    #[test]
    fn test_highlight_oversize_file_rejects() {
        // 模拟超 10MB 输入
        let huge = "a".repeat((crate::MAX_FILE_SIZE_BYTES + 1) as usize);
        let result = highlight(&huge, Language::Rust);
        assert!(matches!(result, Err(TreeSitterError::FileSizeExceeded { .. })));
    }
}
