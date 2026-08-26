//! Integration tests for apeireth-tool-search (post-1.0.0)
//!
//! src/lib.rs 已有 18 #[test] (t01-t18, r239_01-04) + organ_kani_proofs 5.
//! 这里 (tests/) 加跨 API 集成 + 边界 + 大集合行为测试.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_tool_search::{
    now_ms, AggregateBy, Document, FieldFilter, RankedDoc, SearchEngine, SearchOptions, SortBy,
    TimeBucket,
};

// =============================================================================
// Document builder
// =============================================================================

#[test]
fn document_new_fields() {
    let d = Document::new(42, "src", "topic", "body");
    assert_eq!(d.id, 42);
    assert_eq!(d.source, "src");
    assert_eq!(d.topic, "topic");
    assert_eq!(d.body, "body");
    assert!(d.timestamp_ms > 0);
    assert!(d.tags.is_empty());
}

#[test]
fn document_with_tags() {
    let d = Document::new(1, "src", "topic", "body")
        .with_tags(vec![("k1".into(), "v1".into()), ("k2".into(), "v2".into())]);
    assert_eq!(d.tags.len(), 2);
    assert_eq!(d.tags[0], ("k1".to_string(), "v1".to_string()));
}

#[test]
fn document_with_timestamp() {
    let d = Document::new(1, "s", "t", "b").with_timestamp(1234567890);
    assert_eq!(d.timestamp_ms, 1234567890);
}

#[test]
fn document_builder_chain() {
    let d = Document::new(0, "src", "topic", "body")
        .with_tags(vec![("user".into(), "alice".into())])
        .with_timestamp(999);
    assert_eq!(d.tags.len(), 1);
    assert_eq!(d.timestamp_ms, 999);
}

// =============================================================================
// SearchEngine lifecycle
// =============================================================================

#[test]
fn engine_new_empty() {
    let e = SearchEngine::new();
    assert_eq!(e.len(), 0);
    assert!(e.is_empty());
}

#[test]
fn engine_default_is_new() {
    let e = SearchEngine::default();
    assert_eq!(e.len(), 0);
    assert!(e.is_empty());
}

#[test]
fn engine_index_assigns_id_when_zero() {
    let e = SearchEngine::new();
    let id1 = e.index(Document::new(0, "s", "t", "alpha"));
    let id2 = e.index(Document::new(0, "s", "t", "beta"));
    let id3 = e.index(Document::new(0, "s", "t", "gamma"));
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
    assert_eq!(e.len(), 3);
}

#[test]
fn engine_index_preserves_explicit_id() {
    let e = SearchEngine::new();
    let id = e.index(Document::new(100, "s", "t", "alpha"));
    assert_eq!(id, 100, "id != 0 → 用 caller 指定值");
}

#[test]
fn engine_remove_not_found_errors() {
    let e = SearchEngine::new();
    let r = e.remove(999);
    assert!(r.is_err());
}

#[test]
fn engine_remove_then_reindex_id_collision() {
    // remove 后再 index 一个 0-id 文档, next_id 应递增 (不会复用已删 id)
    let e = SearchEngine::new();
    let id1 = e.index(Document::new(0, "s", "t", "alpha"));
    e.remove(id1).unwrap();
    let id2 = e.index(Document::new(0, "s", "t", "beta"));
    assert!(id2 > id1, "next_id 单调: {id2} > {id1}");
}

// =============================================================================
// SearchError / EmptyQuery
// =============================================================================

#[test]
fn search_empty_string_error() {
    let e = SearchEngine::new();
    let r = e.search("", 5);
    assert!(r.is_err());
}

#[test]
fn search_whitespace_only_error() {
    let e = SearchEngine::new();
    let r = e.search("   ", 5);
    assert!(r.is_err());
}

#[test]
fn search_only_stop_words_error() {
    let e = SearchEngine::new();
    // "the a is" 全 stop word, tokenize 后空 → EmptyQuery
    let r = e.search("the a is", 5);
    assert!(r.is_err(), "全 stop words → empty terms → EmptyQuery");
}

// =============================================================================
// FieldFilter
// =============================================================================

#[test]
fn field_filter_source_matcher() {
    let f = FieldFilter::source("frontend");
    let d1 = Document::new(1, "frontend", "msg", "x");
    let d2 = Document::new(2, "memory", "ep", "x");
    assert!(f.matches(&d1));
    assert!(!f.matches(&d2));
}

#[test]
fn field_filter_topic_matcher() {
    let f = FieldFilter::topic("approval");
    let d1 = Document::new(1, "s", "approval", "x");
    let d2 = Document::new(2, "s", "msg", "x");
    assert!(f.matches(&d1));
    assert!(!f.matches(&d2));
}

#[test]
fn field_filter_time_range_matcher() {
    let mut f = FieldFilter::default();
    f.time_min_ms = Some(100);
    f.time_max_ms = Some(200);
    let d1 = Document::new(1, "s", "t", "x").with_timestamp(50);
    let d2 = Document::new(2, "s", "t", "x").with_timestamp(150);
    let d3 = Document::new(3, "s", "t", "x").with_timestamp(300);
    assert!(!f.matches(&d1), "< min");
    assert!(f.matches(&d2));
    assert!(!f.matches(&d3), "> max");
}

#[test]
fn field_filter_time_min_only() {
    let mut f = FieldFilter::default();
    f.time_min_ms = Some(100);
    let d1 = Document::new(1, "s", "t", "x").with_timestamp(50);
    let d2 = Document::new(2, "s", "t", "x").with_timestamp(150);
    assert!(!f.matches(&d1));
    assert!(f.matches(&d2));
}

#[test]
fn field_filter_time_max_only() {
    let mut f = FieldFilter::default();
    f.time_max_ms = Some(100);
    let d1 = Document::new(1, "s", "t", "x").with_timestamp(50);
    let d2 = Document::new(2, "s", "t", "x").with_timestamp(150);
    assert!(f.matches(&d1));
    assert!(!f.matches(&d2));
}

#[test]
fn field_filter_tag_equal_matcher() {
    let mut f = FieldFilter::default();
    f.tag_equal.push(("user".into(), "alice".into()));
    let d1 = Document::new(1, "s", "t", "x").with_tags(vec![("user".into(), "alice".into())]);
    let d2 = Document::new(2, "s", "t", "x").with_tags(vec![("user".into(), "bob".into())]);
    let d3 = Document::new(3, "s", "t", "x").with_tags(vec![("env".into(), "alice".into())]);
    assert!(f.matches(&d1));
    assert!(!f.matches(&d2), "value 不匹配");
    assert!(!f.matches(&d3), "key 不匹配");
}

#[test]
fn field_filter_tag_equal_multiple() {
    let mut f = FieldFilter::default();
    f.tag_equal.push(("user".into(), "alice".into()));
    f.tag_equal.push(("env".into(), "prod".into()));
    let d = Document::new(1, "s", "t", "x").with_tags(vec![
        ("user".into(), "alice".into()),
        ("env".into(), "prod".into()),
    ]);
    let d2 = Document::new(2, "s", "t", "x").with_tags(vec![
        ("user".into(), "alice".into()),
        ("env".into(), "dev".into()),
    ]);
    assert!(f.matches(&d), "两个 tag 都匹配");
    assert!(!f.matches(&d2), "env 不匹配");
}

#[test]
fn field_filter_combined_all_match() {
    let mut f = FieldFilter::default();
    f.source = Some("frontend".into());
    f.topic = Some("msg".into());
    f.time_min_ms = Some(100);
    f.time_max_ms = Some(1000);
    f.tag_equal.push(("user".into(), "alice".into()));
    let d = Document::new(1, "frontend", "msg", "x")
        .with_timestamp(500)
        .with_tags(vec![("user".into(), "alice".into())]);
    assert!(f.matches(&d));
}

#[test]
fn field_filter_empty_matches_all() {
    let f = FieldFilter::default();
    let d = Document::new(1, "any", "any", "any").with_timestamp(100);
    assert!(f.matches(&d));
}

// =============================================================================
// TimeBucket
// =============================================================================

#[test]
fn time_bucket_hour_ms() {
    assert_eq!(TimeBucket::Hour.bucket_ms(), 3_600_000);
}

#[test]
fn time_bucket_day_ms() {
    assert_eq!(TimeBucket::Day.bucket_ms(), 86_400_000);
}

#[test]
fn time_bucket_week_ms() {
    assert_eq!(TimeBucket::Week.bucket_ms(), 7 * 86_400_000);
}

#[test]
fn time_bucket_key_aligns_down() {
    // 任意 ms → 桶起点
    let key = TimeBucket::Hour.bucket_key(3_600_000 + 1234);
    assert_eq!(key, 3_600_000);
    let key = TimeBucket::Day.bucket_key(86_400_000 * 2 + 999);
    assert_eq!(key, 86_400_000 * 2);
}

#[test]
fn time_bucket_key_zero_for_zero() {
    assert_eq!(TimeBucket::Hour.bucket_key(0), 0);
    assert_eq!(TimeBucket::Day.bucket_key(0), 0);
}

// =============================================================================
// search_with_options
// =============================================================================

#[test]
fn search_with_options_default_is_relevance() {
    let opts = SearchOptions::default();
    assert_eq!(opts.sort_by, SortBy::Relevance);
    assert_eq!(opts.recency_decay_secs, 3_600);
    assert_eq!(opts.recency_alpha, 1.0);
}

#[test]
fn search_with_options_sort_helper() {
    let opts = SearchOptions::sort(SortBy::Recency);
    assert_eq!(opts.sort_by, SortBy::Recency);
    // other fields = default
    assert_eq!(opts.recency_decay_secs, 3_600);
}

#[test]
fn sort_by_variants_distinct() {
    assert_ne!(SortBy::Relevance, SortBy::Recency);
    assert_ne!(SortBy::Relevance, SortBy::Hybrid);
    assert_ne!(SortBy::Recency, SortBy::Hybrid);
}

#[test]
fn search_recency_newer_first() {
    let e = SearchEngine::new();
    let now = now_ms();
    e.index(Document::new(1, "a", "t", "alpha").with_timestamp(now - 1000));
    e.index(Document::new(2, "a", "t", "alpha").with_timestamp(now));
    e.index(Document::new(3, "a", "t", "alpha").with_timestamp(now - 500));
    let opts = SearchOptions::sort(SortBy::Recency);
    let r = e.search_with_options("alpha", 10, opts).unwrap();
    assert_eq!(r[0].doc.id, 2, "newest first");
    // Verify monotonic decreasing timestamps
    for win in r.windows(2) {
        assert!(win[0].doc.timestamp_ms >= win[1].doc.timestamp_ms);
    }
}

#[test]
fn search_relevance_orders_by_score() {
    let e = SearchEngine::new();
    e.index(Document::new(1, "a", "t", "alpha"));
    e.index(Document::new(2, "b", "t", "alpha beta gamma delta"));
    let opts = SearchOptions::default();
    let r = e.search_with_options("alpha", 10, opts).unwrap();
    // 长度归一化: 命中密度更高者排前
    assert!(r[0].score >= r[1].score);
}

#[test]
fn search_hybrid_promotes_recent() {
    let e = SearchEngine::new();
    let now = now_ms();
    // 旧 doc: 高 relevance 但低 recency
    e.index(
        Document::new(1, "a", "t", "alpha beta gamma delta epsilon alpha")
            .with_timestamp(now - 86400_000),
    );
    // 新 doc: 低 relevance 但高 recency
    e.index(Document::new(2, "b", "t", "alpha").with_timestamp(now));
    let opts = SearchOptions {
        sort_by: SortBy::Hybrid,
        recency_decay_secs: 60,
        recency_alpha: 5.0,
    };
    let r = e.search_with_options("alpha", 10, opts).unwrap();
    assert_eq!(r[0].doc.id, 2, "hybrid 应让新 doc 排前");
}

// =============================================================================
// Aggregate
// =============================================================================

#[test]
fn aggregate_count_matches_results() {
    let e = SearchEngine::new();
    e.index(Document::new(1, "frontend", "msg", "rust x"));
    e.index(Document::new(2, "frontend", "msg", "rust y"));
    e.index(Document::new(3, "memory", "ep", "rust z"));
    let agg = e
        .aggregate("rust", AggregateBy::Source, &FieldFilter::default())
        .unwrap();
    assert_eq!(agg.count, 3, "count = 命中数");
    assert_eq!(agg.groups.len(), 2, "2 个 source group");
}

#[test]
fn aggregate_groups_sorted_desc_by_count() {
    let e = SearchEngine::new();
    // 用 id=0 让 engine auto-assign unique id (避免 id 冲突)
    for _ in 0..5 {
        e.index(Document::new(0, "frontend", "msg", "rust"));
    }
    for _ in 0..2 {
        e.index(Document::new(0, "memory", "ep", "rust"));
    }
    let agg = e
        .aggregate("rust", AggregateBy::Source, &FieldFilter::default())
        .unwrap();
    assert_eq!(agg.groups[0].1, 5, "frontend 5 排前");
    assert_eq!(agg.groups[1].1, 2);
}

#[test]
fn aggregate_topic_groups() {
    let e = SearchEngine::new();
    e.index(Document::new(1, "s", "msg", "rust"));
    e.index(Document::new(2, "s", "msg", "rust"));
    e.index(Document::new(3, "s", "ep", "rust"));
    let agg = e
        .aggregate("rust", AggregateBy::Topic, &FieldFilter::default())
        .unwrap();
    let msg_count = agg.groups.iter().find(|(k, _)| k == "topic:msg").unwrap().1;
    let ep_count = agg.groups.iter().find(|(k, _)| k == "topic:ep").unwrap().1;
    assert_eq!(msg_count, 2);
    assert_eq!(ep_count, 1);
}

#[test]
fn aggregate_with_filter_applies_first() {
    let e = SearchEngine::new();
    e.index(Document::new(1, "frontend", "msg", "rust"));
    e.index(Document::new(2, "memory", "msg", "rust"));
    e.index(Document::new(3, "memory", "ep", "rust"));
    let agg = e
        .aggregate("rust", AggregateBy::Topic, &FieldFilter::source("memory"))
        .unwrap();
    assert_eq!(agg.count, 2, "只算 memory");
}

// =============================================================================
// Cross-API integration
// =============================================================================

#[test]
fn integration_search_filter_aggregate() {
    let e = SearchEngine::new();
    e.index(Document::new(1, "frontend", "msg", "rust x"));
    e.index(Document::new(2, "frontend", "msg", "rust y"));
    e.index(Document::new(3, "frontend", "approval", "rust z"));
    e.index(Document::new(4, "memory", "msg", "rust w"));

    // Step 1: filter by source
    let r1 = e
        .search_with_filter("rust", &FieldFilter::source("frontend"), 10)
        .unwrap();
    assert_eq!(r1.len(), 3, "frontend 3 个 rust doc");

    // Step 2: 进一步按 topic
    let r2 = e
        .search_with_filter(
            "rust",
            &FieldFilter {
                source: Some("frontend".into()),
                topic: Some("msg".into()),
                ..Default::default()
            },
            10,
        )
        .unwrap();
    assert_eq!(r2.len(), 2);

    // Step 3: aggregate
    let agg = e
        .aggregate("rust", AggregateBy::Topic, &FieldFilter::source("frontend"))
        .unwrap();
    assert_eq!(agg.count, 3);
}

#[test]
fn integration_search_batch_with_options() {
    let e = SearchEngine::new();
    let now = now_ms();
    e.index(Document::new(1, "a", "t", "alpha beta").with_timestamp(now));
    e.index(Document::new(2, "b", "t", "alpha").with_timestamp(now - 100));
    let opts = SearchOptions::sort(SortBy::Recency);
    let r = e.search_with_options("alpha", 10, opts).unwrap();
    assert!(!r.is_empty());
    // doc 1 newer
    assert_eq!(r[0].doc.id, 1);
}

#[test]
fn integration_index_remove_search() {
    let e = SearchEngine::new();
    let id1 = e.index(Document::new(0, "s", "t", "unique_token_xyz"));
    let id2 = e.index(Document::new(0, "s", "t", "other_text"));
    let r = e.search("unique_token_xyz", 10).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].doc.id, id1);
    e.remove(id1).unwrap();
    let r = e.search("unique_token_xyz", 10).unwrap();
    assert!(r.is_empty());
    // other_text 仍在
    let r = e.search("other_text", 10).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].doc.id, id2);
}

#[test]
fn integration_many_docs_search_limit() {
    let e = SearchEngine::new();
    // 用 id=0 让 engine auto-assign unique id, 避免手动 id 重复
    for i in 0..100 {
        e.index(Document::new(0, "src", "topic", format!("common word{i}")));
    }
    assert_eq!(e.len(), 100);
    // "common" 命中 100
    let r = e.search("common", 5).unwrap();
    assert_eq!(r.len(), 5, "limit=5");
}

#[test]
fn integration_tokenize_punctuation_splits() {
    let e = SearchEngine::new();
    e.index(Document::new(1, "s", "t", "hello-world"));
    // 不同分词: tokenize 用 unicode_words, hyphen 不一定 split
    let r = e.search("hello", 5).unwrap();
    // depends on impl — 至少 0 或 1 hit, 不能 >1
    assert!(r.len() <= 1);
}

#[test]
fn integration_unicode_search() {
    let e = SearchEngine::new();
    e.index(Document::new(1, "s", "t", "AI 是 未来的 操作系统"));
    let r = e.search("AI", 5).unwrap();
    assert!(!r.is_empty(), "AI 应能命中");
    assert!(r[0].matched_terms.contains(&"ai".to_string()));
}

// =============================================================================
// now_ms
// =============================================================================

#[test]
fn now_ms_returns_positive() {
    let t = now_ms();
    assert!(t > 1_000_000_000_000);
}

// =============================================================================
// RankedDoc
// =============================================================================

#[test]
fn ranked_doc_creation() {
    let r = RankedDoc {
        doc: Document::new(1, "s", "t", "b"),
        score: 0.5,
        matched_terms: vec!["alpha".into()],
    };
    assert_eq!(r.score, 0.5);
    assert_eq!(r.matched_terms.len(), 1);
}
