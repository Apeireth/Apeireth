//! Deterministic query expansion for hybrid retrieval.
//!
//! Assigned donors (`apeireth-graph`, `apeireth-graph-primitive`,
//! `apeireth-vector`, `apeireth-repo-tools`) have **no** dedicated query
//! rewriter. The closest v2 owner is [`crate::topic_predictor`], whose keyword
//! clusters already map surface forms onto topics.
//!
//! This module inverts those clusters into a retrieval-facing expander:
//! if the query contains any member of a group, the other members are appended.
//! Callers pass [`ExpandedQuery::rewritten`] into
//! [`crate::hybrid_search::HybridSearchEngine`] — this is not a second search
//! owner.

use std::collections::BTreeSet;

use crate::hybrid_search::tokenize;

/// Keyword clusters adapted from [`crate::topic_predictor`] `TOPIC_KEYWORDS`.
/// Groups are the retrieval-facing synonym sets; topic keys themselves are
/// not injected into the rewritten query.
const EXPANSION_GROUPS: &[&[&str]] = &[
    &["考试", "备考", "复习", "线代", "高数"],
    &["作业", "课题", "论文"],
    &[
        "项目",
        "部署",
        "bug",
        "代码",
        "commit",
        "重构",
        "rust",
        "architecture",
    ],
    &["累", "烦", "难过", "孤独", "陪我", "抱抱", "开心"],
    &["游戏", "番剧", "电影", "音乐", "旅行"],
    &["早安", "晚安", "吃饭", "睡觉", "失眠", "健身"],
];

/// Result of expanding a query string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedQuery {
    /// Original query, unchanged.
    pub original: String,
    /// Extra terms that were not already present as substrings, sorted.
    pub extra_terms: Vec<String>,
    /// `original` plus extra terms, joined by spaces. Equals `original` when
    /// nothing was added (including after trim of a blank original).
    pub rewritten: String,
}

/// Expand `query` by appending sibling keywords from matching clusters.
///
/// Matching is case-insensitive substring (same as the topic predictor).
/// Original tokens from [`tokenize`] are never dropped. Extra terms are
/// unique, sorted, and omitted when they already occur in the query.
pub fn expand_query(query: &str) -> ExpandedQuery {
    let original = query.to_string();
    if query.trim().is_empty() {
        return ExpandedQuery {
            original,
            extra_terms: Vec::new(),
            rewritten: query.to_string(),
        };
    }

    let lower = query.to_lowercase();
    let original_tokens: BTreeSet<String> = tokenize(query).into_iter().collect();

    let mut extra: BTreeSet<String> = BTreeSet::new();
    for group in EXPANSION_GROUPS {
        let hit = group.iter().any(|member| {
            let m = member.to_ascii_lowercase();
            lower.contains(&m)
        });
        if !hit {
            continue;
        }
        for member in *group {
            let m = (*member).to_string();
            let already_substring = lower.contains(&m.to_ascii_lowercase());
            let already_token =
                original_tokens.contains(&m.to_ascii_lowercase()) || original_tokens.contains(&m);
            if !already_substring && !already_token {
                extra.insert(m);
            }
        }
    }

    let extra_terms: Vec<String> = extra.into_iter().collect();
    let rewritten = if extra_terms.is_empty() {
        original.clone()
    } else {
        let mut out = original.trim().to_string();
        for term in &extra_terms {
            out.push(' ');
            out.push_str(term);
        }
        out
    };

    ExpandedQuery {
        original,
        extra_terms,
        rewritten,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_is_unchanged() {
        let expanded = expand_query("");
        assert!(expanded.extra_terms.is_empty());
        assert_eq!(expanded.rewritten, "");
        assert!(expand_query("   ").extra_terms.is_empty());
    }

    #[test]
    fn exam_cluster_appends_siblings() {
        let expanded = expand_query("今天复习高数");
        assert!(expanded.extra_terms.contains(&"考试".to_string()));
        assert!(expanded.extra_terms.contains(&"备考".to_string()));
        assert!(!expanded
            .extra_terms
            .iter()
            .any(|t| t == "复习" || t == "高数"));
        assert!(expanded.rewritten.starts_with("今天复习高数"));
        assert!(expanded.rewritten.contains("考试"));
        // Extra terms are sorted, so replay is exact.
        let again = expand_query("今天复习高数");
        assert_eq!(expanded, again);
    }

    #[test]
    fn unmatched_query_does_not_invent_terms() {
        let expanded = expand_query("正交残差金字塔");
        assert!(expanded.extra_terms.is_empty());
        assert_eq!(expanded.rewritten, "正交残差金字塔");
    }

    #[test]
    fn ascii_cluster_is_case_insensitive() {
        let expanded = expand_query("Fix the RUST bug");
        assert!(expanded
            .extra_terms
            .iter()
            .any(|t| t == "项目" || t == "代码"));
        assert_eq!(
            expand_query("Fix the RUST bug").extra_terms,
            expanded.extra_terms
        );
    }

    #[test]
    fn original_tokens_are_never_dropped() {
        let expanded = expand_query("项目部署");
        assert!(expanded.rewritten.contains("项目"));
        assert!(expanded.rewritten.contains("部署"));
    }
}
