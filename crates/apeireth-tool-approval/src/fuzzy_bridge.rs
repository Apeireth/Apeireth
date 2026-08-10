//! **战役 2-3 / VCP §6.2.2 #18 — Fuzzy matching 集成**
//!
//! **目标**: 在审批前, 用战役 2-2 的 `FuzzyToolMatcher` 把 LLM 拼写错的工具名纠正为注册表里的真名.
//!
//! **VCP 借鉴**: `toolApprovalManager.js:55-64 applyRuntimeConfig` `fuzzyToolMatching: true` 字段
//! + `toolMarkerFuzzyMatcher.js` (Levenshtein ≤ 2)
//!
//! **Apeireth 实现**: 直接 import 战役 2-2 的 `FuzzyToolMatcher` (已经在 workspace),
//! 提供 1 个简单 wrapper `match_tool_name(marker, registry) → Option<String>`.

use apeireth_tool_registry::ToolRegistry;
use apeireth_tool_runtime::FuzzyToolMatcher;

/// **战役 2-3 — 工具名 fuzzy matching 集成 (VCP §6.2.2 #18)**
///
/// **VCP 借鉴**: `toolMarkerFuzzyMatcher.js` (Levenshtein ≤ 2 视为同工具)
///
/// **Apeireth 实现**: 直接 wrap 战役 2-2 的 `FuzzyToolMatcher::match_tool`
///
/// **行为**:
/// - `match_tool_name("FileOperater", &registry)` → `Some("FileOperator")` (Levenshtein = 1)
/// - `match_tool_name("FileOperator", &registry)` → `Some("FileOperator")` (完全匹配)
/// - `match_tool_name("XyzNotInRegistry", &registry)` → `None`
/// - `match_tool_name("", &registry)` → `None` (VCP 空 marker 行为)
pub fn match_tool_name(marker: &str, registry: &ToolRegistry) -> Option<String> {
    FuzzyToolMatcher::match_tool(marker, registry)
}

/// **带阈值的 fuzzy matching (供高级用户 / 测试用)**
///
/// **VCP 借鉴**: VCP `toolMarkerFuzzyMatcher` 内部也用阈值, 默认 ≤ 2
pub fn match_tool_name_threshold(
    marker: &str,
    registry: &ToolRegistry,
    max_distance: usize,
) -> Option<String> {
    FuzzyToolMatcher::match_tool_threshold(marker, registry, max_distance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::MockSyncTool;
    use std::sync::Arc;

    fn registry_with_tools() -> ToolRegistry {
        let r = ToolRegistry::new();
        r.register(
            "FileOperator".to_string(),
            Arc::new(MockSyncTool {
                name: "FileOperator".to_string(),
            }),
        );
        r.register(
            "WeatherQuery".to_string(),
            Arc::new(MockSyncTool {
                name: "WeatherQuery".to_string(),
            }),
        );
        r.register(
            "Greeting".to_string(),
            Arc::new(MockSyncTool {
                name: "Greeting".to_string(),
            }),
        );
        r
    }

    #[test]
    fn fuzzy_exact_match() {
        let r = registry_with_tools();
        assert_eq!(
            match_tool_name("FileOperator", &r),
            Some("FileOperator".to_string())
        );
    }

    #[test]
    fn fuzzy_typo_tolerance() {
        let r = registry_with_tools();
        // FileOperater 拼写错误 (少 1 个 o), Levenshtein = 1
        assert_eq!(
            match_tool_name("FileOperater", &r),
            Some("FileOperator".to_string())
        );
        // WeatherQuary 拼写错误 (少 1 个 e), Levenshtein = 1
        assert_eq!(
            match_tool_name("WeatherQuary", &r),
            Some("WeatherQuery".to_string())
        );
    }

    #[test]
    fn fuzzy_no_match_too_far() {
        let r = registry_with_tools();
        // Levenshtein > 2 → None
        assert_eq!(match_tool_name("CompletelyDifferent", &r), None);
    }

    #[test]
    fn fuzzy_empty_marker() {
        let r = registry_with_tools();
        assert_eq!(match_tool_name("", &r), None);
    }

    #[test]
    fn fuzzy_empty_registry() {
        let r = ToolRegistry::new();
        assert_eq!(match_tool_name("FileOperator", &r), None);
    }

    #[test]
    fn fuzzy_threshold_strict() {
        let r = registry_with_tools();
        // 阈值 0 = 只完全匹配
        assert_eq!(
            match_tool_name_threshold("FileOperator", &r, 0),
            Some("FileOperator".to_string())
        );
        assert_eq!(match_tool_name_threshold("FileOperater", &r, 0), None);
        // 阈值 1 = Levenshtein ≤ 1
        assert_eq!(
            match_tool_name_threshold("FileOperater", &r, 1),
            Some("FileOperator".to_string())
        );
    }

    #[test]
    fn fuzzy_case_insensitive() {
        // VCP 模糊匹配默认大小写不敏感
        let r = registry_with_tools();
        assert_eq!(
            match_tool_name("fileoperator", &r),
            Some("FileOperator".to_string()),
            "lowercase marker 应能匹配 camelCase registry 名"
        );
    }
}
