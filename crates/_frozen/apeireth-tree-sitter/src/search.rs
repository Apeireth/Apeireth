//! §3 search — AST 节点搜索 (1:1 翻译 `ripgrep --type rust` AST 节点过滤).
//!
//! **核心 API**: [`search`] (输入 source + SearchQuery, 返 AstNode 列表).
//!
//! **状态**: ⚠️ skeleton — R20 阶段 5 估补, 真实 AST 搜索留 R20 阶段 4 续.
//! 当前返 `TreeSitterError::NotImplemented("search")` (per O-5 不假装).
//!
//! **设计**:
//! - `SearchQuery { kind, name_pattern, max_results }` 3 字段
//! - `SEARCH_MAX_RESULTS = 1000` (编译期 hardcode, 1:1 翻译 ripgrep --max-count)
//! - 比 grep 准: 按 AST 节点类型过滤, 不按文本 (e.g. 搜 "function" 不匹配字符串 "function" 在注释中)
//!
//! **R20 阶段 4 续**: 真实 grammar 接入后, search 走 AST 遍历而非文本匹配.

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{ast::AstNode, ast::NodeKind, Language, TreeSitterError, TreeSitterResult, SEARCH_MAX_RESULTS};

// ============================================================================
// SearchQuery (3 字段: kind + name_pattern + max_results)
// ============================================================================

/// 代码搜索查询 (per §3 search, 1:1 翻译 `ripgrep --type rust` AST 节点过滤).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    /// 节点类型过滤 (e.g. `Some(NodeKind::Function)` 只搜函数).
    /// `None` = 搜所有类型.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<NodeKind>,
    /// 节点名 regex pattern (e.g. `"^test_"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_pattern: Option<String>,
    /// 最大结果数 (默认 SEARCH_MAX_RESULTS 1000, 编译期 hardcode).
    #[serde(default = "default_search_limit")]
    pub max_results: usize,
}

fn default_search_limit() -> usize {
    SEARCH_MAX_RESULTS
}

impl SearchQuery {
    /// 创建新搜索查询 (默认 `max_results = SEARCH_MAX_RESULTS`).
    #[must_use]
    pub fn new() -> Self {
        Self {
            kind: None,
            name_pattern: None,
            max_results: SEARCH_MAX_RESULTS,
        }
    }

    /// 按节点类型过滤 (链式).
    #[must_use]
    pub fn with_kind(mut self, kind: NodeKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// 按节点名 regex 过滤 (链式).
    #[must_use]
    pub fn with_name_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.name_pattern = Some(pattern.into());
        self
    }

    /// 设置最大结果数 (链式).
    #[must_use]
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// §3 核心 API: search (skeleton 阶段返 NotImplemented, R20 阶段 4 续真实 AST 搜索)
// ============================================================================

/// 代码搜索 (按 AST 节点类型 + 名称 regex, 比 grep 准).
///
/// **skeleton 阶段**: 返 `TreeSitterError::NotImplemented("search")`.
/// R20 阶段 4 续: bash/typescript/python/rust 4 grammar 接入后, 此函数:
/// 1. parse(input, language) → AST
/// 2. 遍历 AST, 按 `query.kind` 过滤
/// 3. 按 `query.name_pattern` regex 匹配节点名
/// 4. 截取到 `query.max_results` (默认 SEARCH_MAX_RESULTS 1000)
///
/// # Errors
///
/// - `SearchResultLimitExceeded` — `max_results > SEARCH_MAX_RESULTS`
/// - `NotImplemented` — skeleton 阶段未实现
pub fn search(_source: &str, _language: Language, query: &SearchQuery) -> TreeSitterResult<Vec<AstNode>> {
    // 1) 守门: max_results 不超 SEARCH_MAX_RESULTS
    if query.max_results > SEARCH_MAX_RESULTS {
        return Err(TreeSitterError::SearchResultLimitExceeded {
            max: query.max_results,
        });
    }
    // 2) skeleton 阶段返 NotImplemented
    warn!(
        max_results = query.max_results,
        kind = ?query.kind,
        name_pattern = query.name_pattern.as_deref().unwrap_or("<any>"),
        "search: skeleton 阶段未实现, R20 阶段 4 续真实 AST 遍历"
    );
    Err(TreeSitterError::NotImplemented("search"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_default() {
        let q = SearchQuery::new();
        assert_eq!(q.max_results, SEARCH_MAX_RESULTS);
        assert!(q.kind.is_none());
        assert!(q.name_pattern.is_none());
    }

    #[test]
    fn test_search_query_with_kind_and_pattern() {
        let q = SearchQuery::new()
            .with_kind(NodeKind::Function)
            .with_name_pattern("^test_")
            .with_max_results(50);
        assert_eq!(q.kind, Some(NodeKind::Function));
        assert_eq!(q.name_pattern.as_deref(), Some("^test_"));
        assert_eq!(q.max_results, 50);
    }

    #[test]
    fn test_search_skeleton_returns_not_implemented() {
        let q = SearchQuery::new().with_kind(NodeKind::Function);
        let result = search("fn main() {}", Language::Rust, &q);
        assert!(matches!(result, Err(TreeSitterError::NotImplemented("search"))));
    }

    #[test]
    fn test_search_oversize_max_results_rejects() {
        let q = SearchQuery::new().with_max_results(SEARCH_MAX_RESULTS + 1);
        let result = search("fn main() {}", Language::Rust, &q);
        assert!(matches!(
            result,
            Err(TreeSitterError::SearchResultLimitExceeded { .. })
        ));
    }
}
