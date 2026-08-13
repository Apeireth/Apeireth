# R239 -- tool-search SortBy + SearchOptions

## Problem
\SearchEngine::search()\ 是唯一的全文检索入口, 永远按 BM25-lite \score\ \u964d\u5e8f\u8fd4\u56de.
\u5b9e\u9645\u5e94\u7528\u91cc:
- **recency-sensitive**: \u67e5\u201c\u6700\u8fd1\u8bfb\u8fc7\u7684\u6587\u6863\u201d -- score \u4e0d\u91cd\u8981, \u65b0\u9c9c\u5ea6\u91cd\u8981.
- **hybrid**: score + freshness \u52a0\u6743.

\u6ca1\u6709\u7eaf score \u7684\u9879\u9700\u8981\u4e5f\u88ab\u201c\u540c\u4e00\u5206 = \u540c\u4e00\u6392\u5e8f\u4f4d\u201d -- \u8fd9\u662f\u4e00\u79cd\u4fe1\u606f\u4e27\u5931.

## Solution

### \SortBy\ enum (\u516c\u5f00, Serialize, PartialEq)

\\\ust
pub enum SortBy {
    Relevance,  // BM25-lite score desc (default)
    Recency,    // doc.timestamp_ms desc (newest first)
    Hybrid,     // score + alpha / (1 + age_secs / decay_secs)
}
\\\

### \SearchOptions\ struct (config)

\\\ust
pub struct SearchOptions {
    pub sort_by: SortBy,                // default Relevance
    pub recency_decay_secs: i64,        // default 3600 (1 hour)
    pub recency_alpha: f64,             // default 1.0
}
\\\

### \SearchEngine::search_with_options\ API

\u516c\u5f00 API \u4e0d\u7834\u5740\u73b0\u6709 \search\ / \search_batch\ / \search_with_filter\.
Sort \u5728\u5206\u6570\u8ba1\u7b97\u5b8c\u6210\u4e4b\u540e\u5e94\u7528, \u6700\u540e truncate limit.

## Tests (4 new tests pass)

- r239_01: default = Relevance, score desc
- r239_02: Recency \u6309 timestamp \u964d\u5e8f, newer first
- r239_03: Hybrid \u5728\u540c\u5206\u6587\u6863\u4e2d\u4f18\u5148\u8fd1\u671f\u9879
- r239_04: limit \u5728 sort \u4e4b\u540e\u622a\u53d6, \u9a8c\u8bc1 monotonic

## Design notes

- **\u4e0d\u7834\u5740\u73b0\u6709 API**: \u53e6\u5b9a\u4e49\u72ec\u7acb\u51fd\u6570, \u8001\u8def\u5f84\u4ecd\u53ef\u7528
- **\u539f\u5b50 score**: \u5206\u4e0d\u88ab\u91cd\u7f16\u8f91, \u53ea\u662f\u5728 sort \u9636\u6bb5\u53e0\u52a0\u52a0\u5206
- **\u7eaf\u51fd\u6570\u8fd0\u7b97**: Hybrid \u9879\u53ea\u9700 f64 \u9664\u6cd5, 0 \u5f15\u5165\u989d\u5916 dep

## Files

- \crates/apeireth-tool-search/src/lib.rs\ (+2 types, +1 search API, +4 tests)

cumulative: ~6318 tests pass.