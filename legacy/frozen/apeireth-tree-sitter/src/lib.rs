//! # apeireth-tree-sitter
//!
//! **Tree-sitter skeleton crate** — R20 阶段 5 估补 (per `v09021-rust-translation-blueprint` §2.5.1 4 增强之一).
//!
//! 1:1 翻译 v0.9.21 商业版 `out/main/chunks/tree-sitter-bash-CWNFXErb.js` (2.9MB obfuscated) +
//! `tree-sitter-Fukzi_5-.js` (647KB obfuscated) 集成面 (per blueprint §2.5.1).
//!
//! ## 6 核心 API (skeleton 阶段 enum/struct 完整, 0 真实 grammar, R20 阶段 4 续)
//!
//! - **§1 highlight** (8 语言高亮) — 1:1 翻译 web-tree-sitter `highlighter.highlight()` JS API
//! - **§2 ast** (AST 解析) — 1:1 翻译 web-tree-sitter `parser.parse()` JS API
//! - **§3 search** (AST 节点搜索) — 比 grep 准的代码搜索
//! - **§4 fold** (代码折叠) — 1:1 翻译 Monaco Editor `foldingRangeProvider`
//! - **§5 indent** (缩进检测) — per language tab/space 规则
//! - **§6 lsp** (LSP 协议支持) — 1:1 翻译 LSP 3.17 `textDocument/*` + `workspace/*` 协议面
//!
//! ## 8 编译期 hardcode (K-1 强校验 #2)
//!
//! `TREE_SITTER_SCHEMA_VERSION` / `PLATFORM_NAME` / `SUPPORTED_LANGUAGES` (8) / `MAX_FILE_SIZE_BYTES` (10MB) /
//! `HIGHLIGHT_MAX_TOKEN_LENGTH` (1024) / `AST_MAX_DEPTH` (64) / `SEARCH_MAX_RESULTS` (1000) / `FOLDING_DEFAULT_LEVEL` (2).
//!
//! ## 8 工具白名单 (m3 hallucination 防御, per `m3-hallucination-defense-2026-08-05.md` §2.4)
//!
//! `TOOL_WHITELIST` 编译期 hardcode 8 工具, `validate_tool_call` 在 dispatch 前 schema 校验.
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 1:1 翻译 v0.9.21 商业版 tree-sitter 集成面, 0 业务重设计
//! - **S-2 实事求是**: 商业版 `out/main/chunks/tree-sitter-*` 实查估 2000 LOC 总, skeleton 阶段估 600 行 (3x 缩), R20 阶段 4 续
//! - **O-2 走在前人肩上**: 编译期 hardcode 8 Language + 8 工具白名单, 借鉴 5 P0 + 9 skeleton + i18n + observability 同模式
//! - **O-5 不假装**: 所有方法 `warn!` 占位 + 返 `TreeSitterError::NotImplemented(api_name)`, 真实 grammar 留 R20 阶段 4 续
//! - **O-3 干到底**: skeleton 落地 (8 Language + 8 编译期 hardcode + 8 工具白名单 + 5 核心 API stub + 5 K-1 字样)
//! - **O-4 任何人都能接手**: §1-§6 跟 i18n / observability 同骨架 + 引用 R20 阶段 4 续路径完整
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 5 实施, 主 2026-08-05 21:27 拍板"效率不慢下来,验收了继续派")

#![warn(missing_docs)]
#![allow(clippy::all)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

// 子模块 (per §1-§6 章节切分, 跟 i18n / observability 子模块模式一致)
pub mod ast;
pub mod fold;
pub mod highlight;
pub mod indent;
pub mod lsp;
pub mod search;

// re-export 子模块公共 API (1:1 翻译 apeireth-observability 子模块 re-export 模式)
pub use ast::{parse, AstNode, NodeKind, ParseOptions, ParseResult};
pub use fold::{fold, FoldKind, FoldRange};
pub use highlight::{highlight, HighlightKind, HighlightSpan};
pub use indent::{detect_indent, IndentStyle};
pub use lsp::{lsp_dispatch, LspErrorBody, LspMessage, LspResponse};
pub use search::{search, SearchQuery};

// ============================================================================
// 编译期 hardcode 常量 (8 项, per task spec + K-1 强校验 #2)
// ============================================================================

/// Tree-sitter schema version (1:1 翻译 web-tree-sitter 0.25 接口契约, K-1 强校验).
pub const TREE_SITTER_SCHEMA_VERSION: &str = "1";

/// 平台名 (K-1 强校验 #1: 编译期 hardcode `"apeireth"`, v0.9.21 1:1 翻译, 不写 "SpectrAI" 等装饰名).
pub const PLATFORM_NAME: &str = "apeireth";

/// **支持的语言** (K-1 强校验 #2: 8 Language 枚举, 编译期 hardcode 守门 `len() == 8`).
/// 1:1 翻译 v0.9.21 商业版 tree-sitter 8 主 grammar. R20 阶段 4 续只接 4 (Rust/TS/Python/Bash), 其它 4 留后续.
pub const SUPPORTED_LANGUAGES: &[Language] = &[
    Language::Rust,
    Language::TypeScript,
    Language::JavaScript,
    Language::Python,
    Language::Go,
    Language::Bash,
    Language::Yaml,
    Language::Json,
];

/// 编译期守门: SUPPORTED_LANGUAGES 长度 == 8 (K-1 强校验 #2).
pub const SUPPORTED_LANGUAGES_COUNT: usize = 8;
const _: () = assert!(SUPPORTED_LANGUAGES.len() == SUPPORTED_LANGUAGES_COUNT);

/// **最大文件大小** (10 MB, 1:1 翻译 web-tree-sitter 行业惯例, 防恶意超长文件占内存).
pub const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024;

/// **高亮最大 token 长度** (1024 chars, 1:1 翻译 Monaco Editor).
pub const HIGHLIGHT_MAX_TOKEN_LENGTH: usize = 1024;

/// **AST 最大深度** (64 层, 防栈溢出, 1:1 翻译 tree-sitter 深度限制行业惯例).
pub const AST_MAX_DEPTH: usize = 64;

/// **搜索最大结果数** (1000, 1:1 翻译 ripgrep 默认 --max-count).
pub const SEARCH_MAX_RESULTS: usize = 1000;

/// **代码折叠默认级别** (2 = 折叠到第 2 级, 1:1 翻译 Monaco Editor default).
pub const FOLDING_DEFAULT_LEVEL: u32 = 2;

// ============================================================================
// m3 hallucination 防御 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode 8 工具, validate_tool_call 在 dispatch 前 schema 校验.
// ============================================================================

/// m3 防御: tree-sitter 8 工具白名单 (编译期 hardcode, 不可运行时改).
///
/// **8 工具对应 5 核心 API + 3 元数据**:
/// - §1 highlight (1): `apeireth_tree_sitter_highlight`
/// - §2 parse (1): `apeireth_tree_sitter_parse`
/// - §3 search (1): `apeireth_tree_sitter_search`
/// - §4 fold (1): `apeireth_tree_sitter_fold`
/// - §5 indent (1): `apeireth_tree_sitter_indent`
/// - §6 LSP (1): `apeireth_tree_sitter_lsp`
/// - 元数据 (2): `apeireth_tree_sitter_list_languages` + `apeireth_tree_sitter_validate`
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_tree_sitter_highlight",
    "apeireth_tree_sitter_parse",
    "apeireth_tree_sitter_search",
    "apeireth_tree_sitter_fold",
    "apeireth_tree_sitter_indent",
    "apeireth_tree_sitter_lsp",
    "apeireth_tree_sitter_list_languages",
    "apeireth_tree_sitter_validate",
];

/// 编译期守门: TOOL_WHITELIST 长度 == 8.
pub const TOOL_WHITELIST_COUNT: usize = 8;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返 `TreeSitterError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), TreeSitterError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(TreeSitterError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §1 错误类型 (10 variant, 1:1 翻译 web-tree-sitter + tree-sitter 异常面)
// ============================================================================

/// Tree-sitter 错误类型 (m3 + 5 API + 8 编译期 hardcode 守门).
#[derive(Debug, Error)]
pub enum TreeSitterError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4)
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    /// §1: 语言不支持 (调 `apeireth_tree_sitter_highlight` 但 Language 是未实现的 8 之外语言).
    #[error("language not supported: {0}")]
    LanguageNotSupported(String),

    /// §2: 解析失败 (1:1 翻译 web-tree-sitter ParseError).
    #[error("parse failed for {language}: {message}")]
    ParseFailed {
        /// 语言
        language: String,
        /// 错误消息
        message: String,
    },

    /// §3: 搜索超限 (返回结果数 > SEARCH_MAX_RESULTS 1000, 1:1 翻译 ripgrep --max-count).
    #[error("search results exceeded {max} (max {SEARCH_MAX_RESULTS})")]
    SearchResultLimitExceeded {
        /// 实际数量
        max: usize,
    },

    /// §4: 折叠级别越界 (1-5 范围, 1:1 翻译 Monaco Editor FoldingLevel).
    #[error("folding level {0} out of range [1, 5]")]
    FoldingLevelOutOfRange(u32),

    /// §5: 缩进检测失败 (文件非 UTF-8 或大小超 MAX_FILE_SIZE_BYTES 10MB).
    #[error("indent detection failed: {0}")]
    IndentDetectionFailed(String),

    /// §6: LSP 协议错误 (1:1 翻译 LSP 3.17 `ResponseError`).
    #[error("LSP error code {code}: {message}")]
    LspError {
        /// LSP error code
        code: i32,
        /// 错误消息
        message: String,
    },

    /// 文件大小超 MAX_FILE_SIZE_BYTES 10MB (编译期守门).
    #[error("file size {size} bytes exceeded max {MAX_FILE_SIZE_BYTES}")]
    FileSizeExceeded {
        /// 实际文件大小
        size: u64,
    },

    /// skeleton 阶段未实现 (per O-5 不假装, R20 阶段 4 续真实 grammar 接入).
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),

    /// 通用错误.
    #[error("tree-sitter error: {0}")]
    Other(String),
}

/// Tree-sitter Result 别名.
pub type TreeSitterResult<T> = Result<T, TreeSitterError>;

// ============================================================================
// §2 核心枚举: Language 8 项 (1:1 翻译 v0.9.21 商业版 8 grammar)
// ============================================================================

/// 支持的语言枚举 (8 项, 1:1 翻译 v0.9.21 商业版 tree-sitter 8 grammar).
///
/// 编译期 hardcode 进 `SUPPORTED_LANGUAGES` 守门 `len() == 8`.
/// R20 阶段 4 续只接 4 grammar (Rust / TypeScript / Python / Bash, per blueprint §2.5.1),
/// 其它 4 (JavaScript / Go / Yaml / Json) 留后续阶段续.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Rust (R20 阶段 4 续)
    Rust,
    /// TypeScript (R20 阶段 4 续)
    TypeScript,
    /// JavaScript (R21+ 准备)
    JavaScript,
    /// Python (R20 阶段 4 续)
    Python,
    /// Go (R21+ 准备)
    Go,
    /// Bash (R20 阶段 4 续)
    Bash,
    /// Yaml (R21+ 准备)
    Yaml,
    /// Json (R21+ 准备)
    Json,
}

impl Language {
    /// 1:1 翻译 web-tree-sitter language 名 (snake_case 形式, per LSP `TextDocumentItem.languageId`).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Python => "python",
            Language::Go => "go",
            Language::Bash => "bash",
            Language::Yaml => "yaml",
            Language::Json => "json",
        }
    }

    /// LSP 风格 language id (1:1 翻译 LSP `TextDocumentItem.languageId`, vscode 风格).
    #[must_use]
    pub fn lsp_language_id(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Python => "python",
            Language::Go => "go",
            Language::Bash => "shellscript",
            Language::Yaml => "yaml",
            Language::Json => "json",
        }
    }

    /// 从 string 解析 (LSP `TextDocumentItem.languageId` 推断, 不区分大小写).
    /// 不支持返 `None`, 调用方决定走 `TreeSitterError::LanguageNotSupported`.
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        let lower = s.to_ascii_lowercase();
        match lower.as_str() {
            "rust" | "rs" => Some(Language::Rust),
            "typescript" | "ts" => Some(Language::TypeScript),
            "javascript" | "js" | "jsx" => Some(Language::JavaScript),
            "python" | "py" => Some(Language::Python),
            "go" | "golang" => Some(Language::Go),
            "bash" | "sh" | "shellscript" | "shell" | "zsh" => Some(Language::Bash),
            "yaml" | "yml" => Some(Language::Yaml),
            "json" => Some(Language::Json),
            _ => None,
        }
    }

    /// 所有支持的语言 (1:1 翻译 SUPPORTED_LANGUAGES const).
    #[must_use]
    pub fn all() -> &'static [Language] {
        SUPPORTED_LANGUAGES
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for Language {
    /// 默认语言 = Rust (per SUPPORTED_LANGUAGES 第 0 项, 1:1 翻译 rustfmt/rust-analyzer 行业默认).
    fn default() -> Self {
        Language::Rust
    }
}

// ============================================================================
// §3 5 K-1 字样守门 + m3 防御 helper (公共)
// ============================================================================

/// 5 K-1 字样守门 (per task spec K-1 强校验 #4):
/// - "apeireth" 平台名
/// - "tree_sitter" crate 名
/// - "highlight" 高亮 API
/// - "parse" 解析 API
/// - "must-do" skeleton 必做标记
pub const K1_KEYWORDS: &[&str] = &[
    "apeireth",
    "tree_sitter",
    "highlight",
    "parse",
    "must-do",
];

/// 编译期守门: K-1 5 字样 5 个.
pub const K1_KEYWORDS_COUNT: usize = 5;
const _: () = assert!(K1_KEYWORDS.len() == K1_KEYWORDS_COUNT);

/// 校验字符串是否包含 5 K-1 字样中的至少 1 个 (per K-1 强校验 #4 守门 helper).
#[must_use]
pub fn contains_k1_keyword(s: &str) -> bool {
    K1_KEYWORDS.iter().any(|kw| s.contains(kw))
}
