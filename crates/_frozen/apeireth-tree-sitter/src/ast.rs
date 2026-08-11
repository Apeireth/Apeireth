//! §2 ast — AST 解析 (1:1 翻译 web-tree-sitter `parser.parse()` JS API).
//!
//! **核心 API**: [`parse`] (1:1 翻译 `parser.parse(input, options) -> ParseResult`).
//!
//! **状态**: ⚠️ skeleton — R20 阶段 5 估补, 真实 grammar 留 R20 阶段 4 续.
//! 当前所有 8 语言都返 `TreeSitterError::NotImplemented("parse")` (per O-5 不假装).
//!
//! **设计**:
//! - `NodeKind` 5 顶层类别 (function/class/variable/import/comment, per task spec §2)
//! - `AstNode` 字段对齐 tree-sitter `Node` (type/startIndex/endIndex/startPosition/endPosition)
//! - `ParseOptions` 公共载体, 给 highlight/fold/indent 共享
//! - `ParseResult` 公共载体, 5 核心 API 各自填充对应字段
//! - `AST_MAX_DEPTH = 64` (编译期 hardcode, 防栈溢出)

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{Language, TreeSitterError, TreeSitterResult, AST_MAX_DEPTH};

// ============================================================================
// NodeKind (5 顶层类别, per task spec §2: function/class/variable/import/comment)
// ============================================================================

/// AST 节点类型 (1:1 翻译 web-tree-sitter `node.type`, 5 顶层类别).
///
/// 暴露 function / class / variable / import / comment 5 节点类别 (per task spec §2).
/// 实际 tree-sitter grammar 可暴露 100+ 节点类型, skeleton 阶段只列 5 核心,
/// 其它节点类型用 `Other(String)` 兜底 (R20 阶段 4 续时扩 enum variant).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    /// 函数定义/声明 (`fn` / `def` / `function` 节点)
    Function,
    /// 类/结构体/接口定义 (`class` / `struct` / `interface` 节点)
    Class,
    /// 变量定义/声明 (`let` / `const` / `var` 节点)
    Variable,
    /// import / use 语句 (`use` / `import` / `require` 节点)
    Import,
    /// 注释 (`//` / `#` / `/* */` 节点)
    Comment,
    /// 其它节点 (兜底, R20 阶段 4 续时扩 enum variant)
    Other(String),
}

impl NodeKind {
    /// 从 tree-sitter type string 推断 (skeleton 阶段用简单字符串匹配).
    ///
    /// **匹配顺序** (优先级从高到低, 防 "use_declaration" 被 "declaration" 误判为 Variable):
    /// 1. Import (use_*/import_*/require_*/include_*) — 必须先于 Variable, 因为 `use_declaration` 也含 "declaration"
    /// 2. Class (class/struct/interface/impl) — 必须先于 Variable, 因为 `class_declaration` 也含 "declaration"
    /// 3. Function (function/method/fn)
    /// 4. Variable (variable/let_*/const_* / *_declaration 兜底)
    /// 5. Comment
    /// 6. Other
    #[must_use]
    pub fn from_type_str(type_str: &str) -> Self {
        let lower = type_str.to_ascii_lowercase();
        // 1) Import — 最先匹配 (use_*/import_*/require_*/include_*)
        if lower.starts_with("use_")
            || lower == "use"
            || lower.starts_with("import")
            || lower.starts_with("require")
            || lower.starts_with("include")
        {
            NodeKind::Import
        }
        // 2) Class — 先于 Variable (class/struct/interface/impl_*)
        else if lower.starts_with("class")
            || lower.contains("struct")
            || lower.starts_with("interface")
            || lower.starts_with("impl_")
            || lower == "impl"
        {
            NodeKind::Class
        }
        // 3) Function
        else if lower.starts_with("function")
            || lower.contains("method")
            || lower == "fn"
        {
            NodeKind::Function
        }
        // 4) Variable — 兜底 *_declaration / variable / let_* / const_*
        else if lower.starts_with("variable")
            || lower.starts_with("let_")
            || lower == "let"
            || lower.starts_with("const_")
            || lower == "const"
            || lower.ends_with("_declaration")
        {
            NodeKind::Variable
        }
        // 5) Comment
        else if lower.contains("comment") {
            NodeKind::Comment
        }
        // 6) 兜底
        else {
            NodeKind::Other(type_str.to_string())
        }
    }
}

// ============================================================================
// AstNode (1:1 翻译 web-tree-sitter `Node`)
// ============================================================================

/// AST 节点 (1:1 翻译 web-tree-sitter `Node`, 字段: type/startIndex/endIndex/startPosition/endPosition).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstNode {
    /// 节点类型 (1:1 翻译 tree-sitter `node.type`).
    pub kind: NodeKind,
    /// 节点原始 type string (兜底, 当 `kind` 是 `Other` 时存原值).
    pub type_str: String,
    /// 节点名 (e.g. function name "main" / class name "App").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 起始字节偏移.
    pub start_byte: usize,
    /// 结束字节偏移.
    pub end_byte: usize,
    /// 起始行号 (1-based, 1:1 翻译 tree-sitter `startPosition.row`).
    pub start_row: usize,
    /// 起始列号 (0-based).
    pub start_column: usize,
    /// 结束行号 (1-based).
    pub end_row: usize,
    /// 结束列号 (0-based).
    pub end_column: usize,
}

impl AstNode {
    /// 构造新 AST 节点.
    #[must_use]
    pub fn new(kind: NodeKind, type_str: impl Into<String>, start_byte: usize, end_byte: usize) -> Self {
        Self {
            kind,
            type_str: type_str.into(),
            name: None,
            start_byte,
            end_byte,
            start_row: 0,
            start_column: 0,
            end_row: 0,
            end_column: 0,
        }
    }

    /// 设置节点名 (链式).
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 节点字节大小.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.end_byte.saturating_sub(self.start_byte)
    }
}

// ============================================================================
// ParseOptions (公共, 5 API 共享)
// ============================================================================

/// 解析选项 (1:1 翻译 web-tree-sitter `parser.parse(input, options)` 第二参).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseOptions {
    /// 源语言 (必填, 8 Language 枚举之一).
    pub language: Language,
    /// 文件路径 (可选, 给错误消息用).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// 高亮查询 (默认 true).
    #[serde(default = "default_true")]
    pub with_highlight: bool,
    /// 折叠范围 (默认 true).
    #[serde(default = "default_true")]
    pub with_folding: bool,
    /// 缩进检测 (默认 true).
    #[serde(default = "default_true")]
    pub with_indent: bool,
}

fn default_true() -> bool {
    true
}

impl ParseOptions {
    /// 创建默认解析选项 (仅指定语言).
    #[must_use]
    pub fn new(language: Language) -> Self {
        Self {
            language,
            file_path: None,
            with_highlight: true,
            with_folding: true,
            with_indent: true,
        }
    }
}

// ============================================================================
// ParseResult (公共载体, 5 API 各自填充对应字段)
// ============================================================================

/// 解析结果 (公共载体, 5 核心 API 各自填充对应字段).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParseResult {
    /// 解析的源语言.
    pub language: Language,
    /// 高亮 token 列表 (§1 highlight 输出, 启用时填充).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<crate::HighlightSpan>,
    /// AST 顶层节点列表 (§2 ast 输出, 启用时填充, 仅 5 NodeKind 类别).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ast_nodes: Vec<AstNode>,
    /// 折叠范围列表 (§4 fold 输出, 启用时填充).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fold_ranges: Vec<crate::FoldRange>,
    /// 缩进规则 (§5 indent 输出, 启用时填充).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<crate::IndentStyle>,
    /// 文件字节大小 (供调用方按 MAX_FILE_SIZE_BYTES 守门).
    pub file_size: u64,
    /// 解析耗时 (毫秒, 1:1 翻译 web-tree-sitter `parse()` 计时, skeleton 阶段 = 0).
    pub elapsed_ms: u64,
}

impl ParseResult {
    /// 创建空结果 (skeleton 阶段 helper).
    #[must_use]
    pub fn empty(language: Language) -> Self {
        Self {
            language,
            highlights: Vec::new(),
            ast_nodes: Vec::new(),
            fold_ranges: Vec::new(),
            indent: None,
            file_size: 0,
            elapsed_ms: 0,
        }
    }
}

// ============================================================================
// §2 核心 API: parse (skeleton 阶段返 NotImplemented, R20 阶段 4 续真实 grammar)
// ============================================================================

/// AST 解析 (1:1 翻译 web-tree-sitter `parser.parse(input, options) -> Tree`).
///
/// **skeleton 阶段**: 所有 8 语言都返 `TreeSitterError::NotImplemented("parse")`.
/// R20 阶段 4 续: bash/typescript/python/rust 4 grammar 真实接入后, 此函数返回 `ParseResult` 完整 (highlights + ast_nodes + fold_ranges + indent).
///
/// # Errors
///
/// - `FileSizeExceeded` — input.len() > MAX_FILE_SIZE_BYTES 10MB
/// - `LanguageNotSupported` — language 不在 SUPPORTED_LANGUAGES
/// - `NotImplemented` — skeleton 阶段未实现
pub fn parse(input: &str, options: &ParseOptions) -> TreeSitterResult<ParseResult> {
    // 1) 守门: 文件大小
    let size = input.len() as u64;
    if size > crate::MAX_FILE_SIZE_BYTES {
        return Err(TreeSitterError::FileSizeExceeded { size });
    }
    // 2) 守门: 语言支持
    if !crate::SUPPORTED_LANGUAGES.contains(&options.language) {
        return Err(TreeSitterError::LanguageNotSupported(
            options.language.as_str().to_string(),
        ));
    }
    // 3) AST 深度守门 (skeleton 阶段: 永远 0 深度, R20 阶段 4 续时校验实际 AST)
    let _depth: usize = 0;
    if _depth > AST_MAX_DEPTH {
        return Err(TreeSitterError::ParseFailed {
            language: options.language.as_str().to_string(),
            message: format!("AST depth {AST_MAX_DEPTH} exceeded"),
        });
    }
    // 4) skeleton 阶段返 NotImplemented
    warn!(
        language = options.language.as_str(),
        file_path = options.file_path.as_deref().unwrap_or("<inline>"),
        "parse: skeleton 阶段未实现, R20 阶段 4 续真实 grammar 接入"
    );
    Err(TreeSitterError::NotImplemented("parse"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_kind_from_type_str() {
        assert_eq!(NodeKind::from_type_str("function_item"), NodeKind::Function);
        assert_eq!(NodeKind::from_type_str("function_declaration"), NodeKind::Function);
        assert_eq!(NodeKind::from_type_str("fn"), NodeKind::Function);
        assert_eq!(NodeKind::from_type_str("class_definition"), NodeKind::Class);
        assert_eq!(NodeKind::from_type_str("struct_item"), NodeKind::Class);
        assert_eq!(NodeKind::from_type_str("let_declaration"), NodeKind::Variable);
        assert_eq!(NodeKind::from_type_str("use_declaration"), NodeKind::Import);
        assert_eq!(NodeKind::from_type_str("line_comment"), NodeKind::Comment);
        assert_eq!(
            NodeKind::from_type_str("expression_statement"),
            NodeKind::Other("expression_statement".to_string())
        );
    }

    #[test]
    fn test_ast_node_with_name() {
        let node = AstNode::new(NodeKind::Function, "function_item", 0, 100)
            .with_name("main");
        assert_eq!(node.name.as_deref(), Some("main"));
        assert_eq!(node.byte_len(), 100);
    }

    #[test]
    fn test_parse_options_default() {
        let opts = ParseOptions::new(Language::Rust);
        assert_eq!(opts.language, Language::Rust);
        assert!(opts.with_highlight);
        assert!(opts.with_folding);
        assert!(opts.with_indent);
    }

    #[test]
    fn test_parse_skeleton_returns_not_implemented() {
        let result = parse("fn main() {}", &ParseOptions::new(Language::Rust));
        assert!(matches!(result, Err(TreeSitterError::NotImplemented("parse"))));
    }

    #[test]
    fn test_parse_oversize_file_rejects() {
        let huge = "a".repeat((crate::MAX_FILE_SIZE_BYTES + 1) as usize);
        let result = parse(&huge, &ParseOptions::new(Language::Rust));
        assert!(matches!(result, Err(TreeSitterError::FileSizeExceeded { .. })));
    }
}
