//! §4 fold — 代码折叠 (1:1 翻译 Monaco Editor `foldingRangeProvider` LSP 端).
//!
//! **核心 API**: [`fold`] (输入 source + Language + level, 返 FoldRange 列表).
//!
//! **状态**: ⚠️ skeleton — R20 阶段 5 估补, 真实 AST-driven folding 留 R20 阶段 4 续.
//! 当前返 `TreeSitterError::NotImplemented("fold")` (per O-5 不假装).
//!
//! **设计**:
//! - `FoldKind` 4 类 (1:1 翻译 LSP `FoldingRangeKind`: comment/imports/region/all)
//! - `FoldRange { start_line, end_line, kind }` (1:1 翻译 LSP `FoldingRange`)
//! - `FOLDING_DEFAULT_LEVEL = 2` (编译期 hardcode, 1:1 翻译 Monaco Editor default)
//! - level 范围 [1, 5] (per Monaco Editor)

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{Language, TreeSitterError, TreeSitterResult, FOLDING_DEFAULT_LEVEL};

// ============================================================================
// FoldKind (1:1 翻译 LSP `FoldingRangeKind`)
// ============================================================================

/// 折叠类别 (1:1 翻译 LSP `FoldingRangeKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FoldKind {
    /// 注释块折叠
    Comment,
    /// import / use 块折叠
    Imports,
    /// 区域折叠 (`#region` / `// region`)
    Region,
    /// 全部折叠 (兜底)
    All,
}

impl FoldKind {
    /// LSP `FoldingRangeKind` 字符串 (1:1 翻译 LSP 3.17 协议).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            FoldKind::Comment => "comment",
            FoldKind::Imports => "imports",
            FoldKind::Region => "region",
            FoldKind::All => "all",
        }
    }
}

// ============================================================================
// FoldRange (1:1 翻译 LSP `FoldingRange`)
// ============================================================================

/// 代码折叠范围 (per §4 fold, 1:1 翻译 LSP `FoldingRange`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoldRange {
    /// 起始行号 (0-based, 1:1 翻译 LSP `FoldingRange.startLine`).
    pub start_line: u32,
    /// 结束行号 (0-based, 1:1 翻译 LSP `FoldingRange.endLine`).
    pub end_line: u32,
    /// 折叠类别.
    pub kind: FoldKind,
}

impl FoldRange {
    /// 构造新折叠范围.
    #[must_use]
    pub fn new(start_line: u32, end_line: u32, kind: FoldKind) -> Self {
        Self {
            start_line,
            end_line,
            kind,
        }
    }

    /// 折叠行数.
    #[must_use]
    pub fn line_count(&self) -> u32 {
        self.end_line.saturating_sub(self.start_line) + 1
    }
}

// ============================================================================
// §4 核心 API: fold (skeleton 阶段返 NotImplemented, R20 阶段 4 续真实 folding)
// ============================================================================

/// 代码折叠 (1:1 翻译 Monaco Editor `foldingRangeProvider.provideFoldingRanges`).
///
/// **skeleton 阶段**: 返 `TreeSitterError::NotImplemented("fold")`.
/// R20 阶段 4 续: bash/typescript/python/rust 4 grammar 接入后, 此函数:
/// 1. parse(input, language) → AST
/// 2. 遍历 AST, 按 `level` 深度决定折叠范围 (1 = 顶层 fn/class, 2 = fn + 内部 impl, ...)
/// 3. 收集 FoldRange 列表
///
/// # Errors
///
/// - `FoldingLevelOutOfRange` — level 超出 [1, 5] 范围
/// - `NotImplemented` — skeleton 阶段未实现
pub fn fold(_source: &str, _language: Language, level: u32) -> TreeSitterResult<Vec<FoldRange>> {
    // 1) 守门: level 在 [1, 5] 范围 (per Monaco Editor FoldingLevel)
    if !(1..=5).contains(&level) {
        return Err(TreeSitterError::FoldingLevelOutOfRange(level));
    }
    // 2) skeleton 阶段返 NotImplemented
    warn!(
        level = level,
        default = FOLDING_DEFAULT_LEVEL,
        "fold: skeleton 阶段未实现, R20 阶段 4 续真实 AST 折叠"
    );
    Err(TreeSitterError::NotImplemented("fold"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_kind_lsp_strings() {
        assert_eq!(FoldKind::Comment.as_str(), "comment");
        assert_eq!(FoldKind::Imports.as_str(), "imports");
        assert_eq!(FoldKind::Region.as_str(), "region");
        assert_eq!(FoldKind::All.as_str(), "all");
    }

    #[test]
    fn test_fold_range_line_count() {
        let r = FoldRange::new(0, 9, FoldKind::Comment);
        assert_eq!(r.line_count(), 10);
        let r2 = FoldRange::new(5, 5, FoldKind::Imports);
        assert_eq!(r2.line_count(), 1);
    }

    #[test]
    fn test_fold_skeleton_returns_not_implemented() {
        let result = fold("fn main() {}", Language::Rust, FOLDING_DEFAULT_LEVEL);
        assert!(matches!(result, Err(TreeSitterError::NotImplemented("fold"))));
    }

    #[test]
    fn test_fold_level_zero_rejects() {
        let result = fold("fn main() {}", Language::Rust, 0);
        assert!(matches!(result, Err(TreeSitterError::FoldingLevelOutOfRange(0))));
    }

    #[test]
    fn test_fold_level_six_rejects() {
        let result = fold("fn main() {}", Language::Rust, 6);
        assert!(matches!(result, Err(TreeSitterError::FoldingLevelOutOfRange(6))));
    }
}
