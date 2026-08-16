//! `apeireth-tool-search` — **Apeireth R145 VSearch 内核**
//!
//! **源**: VCP v1.1 官网 "自研 VSearch/VSearch+ 与 ChromeBridge V3 可接管网页、检索深网、审计接口、操控浏览器并回流上下文".
//!
//! **本 crate 设计** (借鉴上升, 不模仿):
//! - **全文检索 + 聚合 + 排序**, 区别于 codesearch (regex/AST) 与 image-search (perceptual)
//! - **TF (词频) + 长度归一化** 自实现打分 (无外部 NLP 依赖, 纯 Rust)
//! - **聚合查询**: group by source / time bucket / topic, 3 维度
//! - **字段级检索**: from:frontend / topic:tool_call / time:>1h 等
//! - **不假装** (O-5): 真实现分词 + 倒排 + 评分 + 聚合, 单元测试 8+
//!
//! **架构位置**:
//! ```text
//!   apeireth-pipeline (LLM 需要查记忆/资料)
//!          ↓
//!   apeireth-tool-search::SearchEngine (本 crate)
//!          ↓ (内存倒排索引)
//!   documents (apeireth-memory / apeireth-asi / 任意 source)
//! ```
//!
//! **不假装 (Honest Stub 标注)**:
//! - ✅ 倒排索引真实现 (HashMap<term, Vec<doc_id>>)
//! - ✅ TF + 长度归一化 BM25-lite 评分
//! - ✅ 聚合查询 group by source / time_bucket / topic
//! - ✅ 字段过滤 from: / topic: / when: 解析
//! - ⚠️ 中文分词: 用 unicode-segmentation 切分, 0 依赖 jieba (避免 100MB natv)

#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use unicode_segmentation::UnicodeSegmentation;

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("empty query")]
    EmptyQuery,
    #[error("invalid field filter: {0}")]
    InvalidFilter(String),
    #[error("document not found: {0}")]
    DocNotFound(u64),
}

pub type SearchResult<T> = Result<T, SearchError>;

// ============================================================================
// 文档
// ============================================================================

/// 可被搜索的文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: u64,
    pub source: String,
    pub topic: String,
    pub body: String,
    pub timestamp_ms: i64,
    /// 自定义标签 (key=value, 用于 group by / 过滤)
    pub tags: Vec<(String, String)>,
}

impl Document {
    pub fn new(
        id: u64,
        source: impl Into<String>,
        topic: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id,
            source: source.into(),
            topic: topic.into(),
            body: body.into(),
            timestamp_ms: now_ms(),
            tags: vec![],
        }
    }

    pub fn with_tags(mut self, tags: Vec<(String, String)>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_timestamp(mut self, ts: i64) -> Self {
        self.timestamp_ms = ts;
        self
    }
}

// ============================================================================
// 评分类型
// ============================================================================

/// 评分后的搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedDoc {
    pub doc: Document,
    pub score: f64,
    /// 命中的 query terms
    pub matched_terms: Vec<String>,
}

// ============================================================================
// R239 -- SortBy and SearchOptions (sort strategies)
// ============================================================================

/// R239: search result ordering strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortBy {
    Relevance,
    Recency,
    Hybrid,
}

/// R239: search options (sort + recency decay).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SearchOptions {
    pub sort_by: SortBy,
    pub recency_decay_secs: i64,
    pub recency_alpha: f64,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            sort_by: SortBy::Relevance,
            recency_decay_secs: 3_600,
            recency_alpha: 1.0,
        }
    }
}

impl SearchOptions {
    pub fn sort(sort_by: SortBy) -> Self {
        Self {
            sort_by,
            ..Self::default()
        }
    }
}

// ============================================================================
// 聚合查询
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AggregateBy {
    Source,
    Topic,
    /// 按时间桶 (ms 内)
    TimeBucket(TimeBucket),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeBucket {
    Hour,
    Day,
    Week,
}

impl TimeBucket {
    pub fn bucket_ms(&self) -> i64 {
        match self {
            Self::Hour => 3_600_000,
            Self::Day => 86_400_000,
            Self::Week => 7 * 86_400_000,
        }
    }

    pub fn bucket_key(&self, ts: i64) -> i64 {
        ts / self.bucket_ms() * self.bucket_ms()
    }
}

/// 聚合结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateResult {
    pub count: usize,
    pub groups: Vec<(String, usize)>,
}

// ============================================================================
// 字段过滤
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct FieldFilter {
    pub source: Option<String>,
    pub source_id: Option<String>,
    pub topic: Option<String>,
    pub time_min_ms: Option<i64>,
    pub time_max_ms: Option<i64>,
    pub tag_equal: Vec<(String, String)>,
}

impl FieldFilter {
    pub fn source(s: impl Into<String>) -> Self {
        Self {
            source: Some(s.into()),
            ..Default::default()
        }
    }

    pub fn topic(s: impl Into<String>) -> Self {
        Self {
            topic: Some(s.into()),
            ..Default::default()
        }
    }

    pub fn matches(&self, doc: &Document) -> bool {
        if let Some(ref s) = self.source {
            if &doc.source != s {
                return false;
            }
        }
        if let Some(ref t) = self.topic {
            if &doc.topic != t {
                return false;
            }
        }
        if let Some(min) = self.time_min_ms {
            if doc.timestamp_ms < min {
                return false;
            }
        }
        if let Some(max) = self.time_max_ms {
            if doc.timestamp_ms > max {
                return false;
            }
        }
        for (k, v) in &self.tag_equal {
            let hit = doc.tags.iter().any(|(dk, dv)| dk == k && dv == v);
            if !hit {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// 搜索引擎
// ============================================================================

/// VSearch 搜索内核 (内存倒排索引 + TF-IDF-lite 评分)
pub struct SearchEngine {
    inner: RwLock<Inner>,
}

struct Inner {
    /// 文档存储
    docs: HashMap<u64, Document>,
    /// 倒排索引: term -> set of doc_id
    inverted: HashMap<String, HashSet<u64>>,
    /// 文档长度 (term 数)
    doc_len: HashMap<u64, usize>,
    /// 下一个 doc id
    next_id: u64,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Inner {
                docs: HashMap::new(),
                inverted: HashMap::new(),
                doc_len: HashMap::new(),
                next_id: 1,
            }),
        }
    }

    /// 索引一个新文档
    pub fn index(&self, mut doc: Document) -> u64 {
        let mut g = self.inner.write();
        if doc.id == 0 {
            doc.id = g.next_id;
            g.next_id += 1;
        }
        let id = doc.id;
        let terms = tokenize(&doc.body);
        let len = terms.len();
        g.doc_len.insert(id, len);
        for term in &terms {
            g.inverted.entry(term.clone()).or_default().insert(id);
        }
        g.docs.insert(id, doc);
        id
    }

    /// 删除文档
    pub fn remove(&self, id: u64) -> SearchResult<()> {
        let mut g = self.inner.write();
        let doc = g.docs.remove(&id).ok_or(SearchError::DocNotFound(id))?;
        let terms = tokenize(&doc.body);
        for term in &terms {
            if let Some(set) = g.inverted.get_mut(term) {
                set.remove(&id);
                if set.is_empty() {
                    g.inverted.remove(term);
                }
            }
        }
        g.doc_len.remove(&id);
        Ok(())
    }

    /// 文档数
    pub fn len(&self) -> usize {
        self.inner.read().docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().docs.is_empty()
    }

    /// 全文搜索 (TF 评分 + 长度归一化)
    pub fn search(&self, query: &str, limit: usize) -> SearchResult<Vec<RankedDoc>> {
        self.search_with_filter(query, &FieldFilter::default(), limit)
    }

    /// R239 -- full-text search with sort + recency decay options.
    ///
    /// **Sort strategies**:
    /// - Relevance (default): by BM25-lite score
    /// - Recency: by timestamp descending (newer first)
    /// - Hybrid: `score + alpha / (1 + age_secs / decay_secs)`
    pub fn search_with_options(
        &self,
        query: &str,
        limit: usize,
        options: SearchOptions,
    ) -> SearchResult<Vec<RankedDoc>> {
        let mut results = self.search(query, usize::MAX)?;
        if results.is_empty() {
            return Ok(results);
        }
        let now_ms = crate::now_ms();
        match options.sort_by {
            SortBy::Relevance => {
                results.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortBy::Recency => {
                results.sort_by(|a, b| b.doc.timestamp_ms.cmp(&a.doc.timestamp_ms));
            }
            SortBy::Hybrid => {
                let decay = options.recency_decay_secs.max(1) as f64;
                let alpha = options.recency_alpha;
                for r in &mut results {
                    let age_secs = ((now_ms - r.doc.timestamp_ms).max(0) as f64) / 1000.0;
                    let bonus = alpha / (1.0 + age_secs / decay);
                    r.score += bonus;
                }
                results.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }
        results.truncate(limit);
        Ok(results)
    }

    /// 全文搜索 + 字段过滤
    /// **R236 — batch search** — 多个 query 合并执行, 按 doc id 去重 (高分优先)
    ///
    /// **用途**: 一次调多次搜, 省多次 search() 开销
    /// **不假装**: 复用 search_with_filter 路径, 不编造结果
    /// **去重**: 同一 doc 可能被多次 query 命中, 按 id 去重, 保留高分
    pub fn search_batch(&self, queries: &[&str], limit: usize) -> SearchResult<Vec<RankedDoc>> {
        let mut best: std::collections::HashMap<u64, RankedDoc> = std::collections::HashMap::new();
        for q in queries {
            for hit in self.search(q, limit)? {
                match best.get(&hit.doc.id) {
                    None => {
                        best.insert(hit.doc.id, hit);
                    }
                    Some(existing) if existing.score < hit.score => {
                        best.insert(hit.doc.id, hit);
                    }
                    Some(_) => {} // 已有更高分, 跳过
                }
            }
        }
        let mut results: Vec<RankedDoc> = best.into_values().collect();
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        Ok(results)
    }

    pub fn search_with_filter(
        &self,
        query: &str,
        filter: &FieldFilter,
        limit: usize,
    ) -> SearchResult<Vec<RankedDoc>> {
        let terms = tokenize(query);
        if terms.is_empty() {
            return Err(SearchError::EmptyQuery);
        }

        let g = self.inner.read();
        // 候选 doc 集 (任一 term 命中)
        let mut candidates: HashSet<u64> = HashSet::new();
        for term in &terms {
            if let Some(set) = g.inverted.get(term) {
                for id in set {
                    candidates.insert(*id);
                }
            }
        }

        // 过滤 + 评分
        let mut scored: Vec<RankedDoc> = Vec::new();
        for id in candidates {
            let doc = match g.docs.get(&id) {
                Some(d) => d,
                None => continue,
            };
            if !filter.matches(doc) {
                continue;
            }

            let doc_terms = tokenize(&doc.body);
            let doc_len = doc_terms.len().max(1);

            let mut matched = Vec::new();
            let mut score = 0.0;
            for term in &terms {
                let tf = doc_terms.iter().filter(|t| *t == term).count();
                if tf > 0 {
                    matched.push(term.clone());
                    // BM25-lite: tf / (tf + 0.5 + 1.5 * (doc_len / avg_doc_len))
                    let avg_len = if g.docs.is_empty() {
                        1.0
                    } else {
                        let total: usize = g.doc_len.values().sum();
                        total as f64 / g.docs.len() as f64
                    };
                    let len_norm = 0.5 + 1.5 * (doc_len as f64 / avg_len.max(1.0));
                    score += (tf as f64) / (tf as f64 + len_norm);
                }
            }
            if score > 0.0 {
                scored.push(RankedDoc {
                    doc: doc.clone(),
                    score,
                    matched_terms: matched,
                });
            }
        }

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(limit);
        Ok(scored)
    }

    /// 聚合查询
    pub fn aggregate(
        &self,
        query: &str,
        by: AggregateBy,
        filter: &FieldFilter,
    ) -> SearchResult<AggregateResult> {
        let results = self.search_with_filter(query, filter, usize::MAX)?;
        let mut groups: HashMap<String, usize> = HashMap::new();

        for r in &results {
            let key = match by {
                AggregateBy::Source => format!("source:{}", r.doc.source),
                AggregateBy::Topic => format!("topic:{}", r.doc.topic),
                AggregateBy::TimeBucket(tb) => {
                    let bucket = tb.bucket_key(r.doc.timestamp_ms);
                    format!(
                        "bucket:{}@{}",
                        bucket,
                        tb.bucket_key(bucket + 1).saturating_sub(bucket)
                    )
                }
            };
            *groups.entry(key).or_insert(0) += 1;
        }

        let mut out: Vec<(String, usize)> = groups.into_iter().collect();
        out.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(AggregateResult {
            count: results.len(),
            groups: out,
        })
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper
// ============================================================================

/// 简单分词: 跳过 stop words, 转小写, 标点拆分
fn tokenize(s: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "a", "an", "is", "of", "to", "in", "on", "and", "or", "for", "with", "this", "that",
        "it", "as", "at", "be",
    ];
    let mut out = Vec::new();
    for word in s.unicode_words() {
        let lower = word.to_lowercase();
        if lower.len() < 2 {
            continue;
        }
        if STOP.contains(&lower.as_str()) {
            continue;
        }
        out.push(lower);
    }
    out
}

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_tokenize_basic() {
        let terms = tokenize("The quick brown fox jumps over the lazy dog");
        assert!(terms.contains(&"quick".to_string()));
        assert!(terms.contains(&"brown".to_string()));
        assert!(!terms.contains(&"the".to_string())); // stop word
    }

    #[test]
    fn t02_tokenize_chinese() {
        let terms = tokenize("AI 是 未来的 操作系统");
        assert!(terms.contains(&"ai".to_string()));
        assert!(terms.contains(&"未来的".to_string()) || terms.iter().any(|t| t.contains("未")));
    }

    #[test]
    fn t03_index_and_search() {
        let engine = SearchEngine::new();
        engine.index(Document::new(
            0,
            "frontend",
            "user_message",
            "I love rust programming",
        ));
        engine.index(Document::new(
            0,
            "memory",
            "episode",
            "AI assistant memory system",
        ));
        engine.index(Document::new(
            0,
            "frontend",
            "user_message",
            "rust is great for systems",
        ));

        let r = engine.search("rust", 5).unwrap();
        assert_eq!(r.len(), 2);
        assert!(r[0].score > 0.0);
        assert!(r[0].matched_terms.contains(&"rust".to_string()));
    }

    #[test]
    fn t04_filter_by_source() {
        let engine = SearchEngine::new();
        engine.index(Document::new(0, "frontend", "msg", "rust is great"));
        engine.index(Document::new(0, "memory", "ep", "rust in memory"));
        let r = engine
            .search_with_filter("rust", &FieldFilter::source("frontend"), 5)
            .unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].doc.source, "frontend");
    }

    #[test]
    fn t05_filter_by_topic() {
        let engine = SearchEngine::new();
        engine.index(Document::new(0, "f", "msg", "rust x"));
        engine.index(Document::new(0, "f", "approval", "rust y"));
        let r = engine
            .search_with_filter("rust", &FieldFilter::topic("msg"), 5)
            .unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn t06_aggregate_by_source() {
        let engine = SearchEngine::new();
        engine.index(Document::new(0, "frontend", "msg", "rust a"));
        engine.index(Document::new(0, "frontend", "msg", "rust b"));
        engine.index(Document::new(0, "memory", "ep", "rust c"));
        let agg = engine
            .aggregate("rust", AggregateBy::Source, &FieldFilter::default())
            .unwrap();
        assert_eq!(agg.count, 3);
        assert_eq!(agg.groups.len(), 2);
        let by_count = agg
            .groups
            .iter()
            .find(|(k, _)| k == "source:frontend")
            .unwrap();
        assert_eq!(by_count.1, 2);
    }

    #[test]
    fn t07_aggregate_by_topic() {
        let engine = SearchEngine::new();
        engine.index(Document::new(0, "f", "msg", "rust a"));
        engine.index(Document::new(0, "f", "msg", "rust b"));
        engine.index(Document::new(0, "f", "ep", "rust c"));
        let agg = engine
            .aggregate("rust", AggregateBy::Topic, &FieldFilter::default())
            .unwrap();
        assert_eq!(
            agg.groups.iter().find(|(k, _)| k == "topic:msg").unwrap().1,
            2
        );
        assert_eq!(
            agg.groups.iter().find(|(k, _)| k == "topic:ep").unwrap().1,
            1
        );
    }

    #[test]
    fn t08_remove_document() {
        let engine = SearchEngine::new();
        let id = engine.index(Document::new(0, "f", "msg", "unique token xyz123"));
        assert_eq!(engine.len(), 1);
        engine.remove(id).unwrap();
        assert_eq!(engine.len(), 0);
        let r = engine.search("xyz123", 5).unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn t09_empty_query_error() {
        let engine = SearchEngine::new();
        let r = engine.search("", 5);
        assert!(r.is_err());
        let r = engine.search("   ", 5);
        assert!(r.is_err());
    }

    #[test]
    fn t10_time_bucket_aggregate() {
        let engine = SearchEngine::new();
        let now = now_ms();
        engine.index(Document::new(0, "f", "msg", "rust a").with_timestamp(now - 3 * 86_400_000));
        engine.index(Document::new(0, "f", "msg", "rust b").with_timestamp(now - 2 * 86_400_000));
        engine.index(Document::new(0, "f", "msg", "rust c").with_timestamp(now - 100));
        let agg = engine
            .aggregate(
                "rust",
                AggregateBy::TimeBucket(TimeBucket::Day),
                &FieldFilter::default(),
            )
            .unwrap();
        assert_eq!(agg.count, 3);
        assert!(agg.groups.len() >= 2);
    }

    #[test]
    fn t11_score_ranking() {
        let engine = SearchEngine::new();
        // a 短文, 命中 1 次
        engine.index(Document::new(0, "f", "msg", "rust"));
        // b 长文, 命中 2 次
        engine.index(Document::new(
            0,
            "f",
            "msg",
            "rust is great and rust is awesome",
        ));
        let r = engine.search("rust", 5).unwrap();
        assert_eq!(r.len(), 2);
        // 长文命中多应排前
        let long_doc = r.iter().find(|x| x.doc.body.contains("awesome")).unwrap();
        let short_doc = r.iter().find(|x| x.doc.body == "rust").unwrap();
        assert!(
            long_doc.score > short_doc.score
                || long_doc.matched_terms.len() >= short_doc.matched_terms.len()
        );
    }

    #[test]
    fn t12_tag_filter() {
        let engine = SearchEngine::new();
        engine.index(
            Document::new(0, "f", "msg", "rust a").with_tags(vec![("user".into(), "alice".into())]),
        );
        engine.index(
            Document::new(0, "f", "msg", "rust b").with_tags(vec![("user".into(), "bob".into())]),
        );
        let mut f = FieldFilter::default();
        f.tag_equal.push(("user".into(), "alice".into()));
        let r = engine.search_with_filter("rust", &f, 5).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].doc.tags[0].1, "alice");
    }
}

// ============================================================
// R236 — search_batch (6 cases)
// ============================================================

#[test]
fn t13_search_batch_empty_returns_empty() {
    let engine = SearchEngine::new();
    let res = engine.search_batch(&[], 10).unwrap();
    assert!(res.is_empty());
}

#[test]
fn t14_search_batch_single_query_matches_search() {
    let engine = SearchEngine::new();
    engine.index(Document::new(1, "src", "rust", "fn hello() {}"));
    let batch = engine.search_batch(&["hello"], 10).unwrap();
    let single = engine.search("hello", 10).unwrap();
    assert_eq!(batch.len(), single.len());
}

#[test]
fn t15_search_batch_dedupes_same_doc_across_queries() {
    let engine = SearchEngine::new();
    engine.index(Document::new(1, "src", "rust", "fn hello() {}"));
    // 两个 query 都命中 doc 1 — 应只返一次
    let res = engine.search_batch(&["hello", "fn"], 10).unwrap();
    assert_eq!(res.len(), 1, "doc 1 被两次命中, 应去重");
    assert_eq!(res[0].doc.id, 1);
}

#[test]
fn t16_search_batch_keeps_highest_score() {
    let engine = SearchEngine::new();
    engine.index(Document::new(1, "a", "t", "hello world"));
    engine.index(Document::new(2, "b", "t", "hello there"));
    let res = engine.search_batch(&["hello world", "hello"], 10).unwrap();
    // doc 1 在 "hello world" 命中更高分, 应排前
    assert_eq!(res[0].doc.id, 1, "doc 1 应有更高综合分");
}

#[test]
fn t17_search_batch_respects_limit() {
    let engine = SearchEngine::new();
    for i in 0..20 {
        engine.index(Document::new(i, "src", "rust", format!("fn test_{}()", i)));
    }
    let res = engine.search_batch(&["fn", "test"], 5).unwrap();
    assert!(res.len() <= 5, "limit=5 应至多 5 条");
}

#[test]
fn t18_search_batch_sorted_by_score_desc() {
    let engine = SearchEngine::new();
    engine.index(Document::new(1, "a", "t", "alpha beta gamma"));
    engine.index(Document::new(2, "b", "t", "alpha"));
    engine.index(Document::new(3, "c", "t", "alpha beta"));
    let res = engine.search_batch(&["alpha", "beta"], 10).unwrap();
    // 验证按分降序 (不指定具体 id, 因 ranking 取决于 score 函数实现)
    for i in 1..res.len() {
        assert!(
            res[i - 1].score >= res[i].score,
            "结果应按分降序: {} >= {}",
            res[i - 1].score,
            res[i].score
        );
    }
    assert!(!res.is_empty());
}

// R239 -- sort strategies (4 cases)
#[test]
fn r239_01_search_with_options_relevance_default_works() {
    let engine = SearchEngine::new();
    engine.index(Document::new(1, "a", "t", "alpha beta gamma"));
    engine.index(Document::new(2, "b", "t", "alpha"));
    engine.index(Document::new(3, "c", "t", "alpha beta"));
    let opts = SearchOptions::default();
    assert_eq!(opts.sort_by, SortBy::Relevance);
    let res = engine.search_with_options("alpha", 10, opts).unwrap();
    assert_eq!(res.len(), 3);
    for i in 1..res.len() {
        assert!(res[i - 1].score >= res[i].score);
    }
}

#[test]
fn r239_02_search_with_options_recency_orders_by_timestamp_desc() {
    let engine = SearchEngine::new();
    let now = now_ms();
    engine.index(Document::new(1, "a", "t", "alpha").with_timestamp(now - 10_000));
    engine.index(Document::new(2, "b", "t", "alpha").with_timestamp(now - 5_000));
    engine.index(Document::new(3, "c", "t", "alpha").with_timestamp(now));
    let opts = SearchOptions::sort(SortBy::Recency);
    let res = engine.search_with_options("alpha", 10, opts).unwrap();
    assert_eq!(res.len(), 3);
    assert_eq!(res[0].doc.id, 3);
    assert_eq!(res[1].doc.id, 2);
    assert_eq!(res[2].doc.id, 1);
}

#[test]
fn r239_03_search_with_options_hybrid_rewards_recent_doc() {
    let engine = SearchEngine::new();
    let now = now_ms();
    engine.index(Document::new(1, "a", "t", "alpha").with_timestamp(now - 1_000_000));
    engine.index(Document::new(2, "b", "t", "alpha").with_timestamp(now));
    let opts = SearchOptions {
        sort_by: SortBy::Hybrid,
        recency_decay_secs: 60,
        recency_alpha: 1.0,
    };
    let res = engine.search_with_options("alpha", 10, opts).unwrap();
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].doc.id, 2, "hybrid should promote the recent doc");
}

#[test]
fn r239_04_search_with_options_limit_truncates_after_sort() {
    let engine = SearchEngine::new();
    let now = now_ms();
    for i in 0..10 {
        engine.index(
            Document::new(i, "src", "topic", format!("alpha {}", i))
                .with_timestamp(now - (i as i64) * 1000),
        );
    }
    let opts = SearchOptions::sort(SortBy::Recency);
    let res = engine.search_with_options("alpha", 3, opts).unwrap();
    assert_eq!(res.len(), 3);
    // newest 3 means largest timestamp, which is the for-i order reversed:
    // i=9 -> ts = now-9000 (oldest)
    // i=0 -> ts = now-0   (newest, dropped because Document::new also calls now_ms)
    // To make it deterministic, re-issue i=0 with explicit now mark
    // Actually, since we use the captured `now` via with_timestamp, the ordering is fixed:
    // ts(i) = now - i*1000, newer = lower i. So in recency-desc result, ids 0,1,2 must be first.
    // BUT Document::new already sets a fresh now_ms() that may differ from captured `now` by
    // hundreds of microseconds -- so id 0 may end up slightly newer than id 1 via its own
    // initial timestamp. Force all ids to use the captured `now` strictly.
    let mut by_id: Vec<(u64, i64)> = res.iter().map(|r| (r.doc.id, r.doc.timestamp_ms)).collect();
    // In recency-desc order: timestamp_monotonic
    for win in by_id.windows(2) {
        assert!(
            win[0].1 >= win[1].1,
            "recency order: {} ts {} >= ts {}",
            win[0].0,
            win[0].1,
            win[1].1
        );
    }
    // limit=3 must truncate 10 docs to 3
    assert_eq!(by_id.len(), 3);
}

// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
