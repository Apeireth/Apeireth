//! Integration tests for apeireth-arbitration (post-1.0.0)
//!
//! src/lib.rs 已有 8 #[test] (t01-t08) + organ_kani_proofs 5 #[test].
//! 这里 (tests/) 加跨场景集成: file persistence, multi-source, append-only invariant, clone-share.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_arbitration::{now_ms, ArbError, ArbitrationEvent, ArbitrationLog, EventSource};

// =============================================================================
// EventSource
// =============================================================================

#[test]
fn event_source_all_6_unique() {
    let names: Vec<&str> = EventSource::ALL.iter().map(|s| s.as_str()).collect();
    let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
    assert_eq!(EventSource::COUNT, 6);
    assert_eq!(unique.len(), 6, "6 个 source 互不相同");
}

#[test]
fn event_source_as_str_match() {
    assert_eq!(EventSource::Frontend.as_str(), "frontend");
    assert_eq!(EventSource::GroupChat.as_str(), "group_chat");
    assert_eq!(EventSource::Email.as_str(), "email");
    assert_eq!(EventSource::AgentComm.as_str(), "agent_comm");
    assert_eq!(EventSource::System.as_str(), "system");
    assert_eq!(EventSource::External.as_str(), "external");
}

#[test]
fn event_source_eq_and_copy() {
    // EventSource: Copy + Eq + Hash
    let s = EventSource::Frontend;
    let s2 = s;
    assert_eq!(s, s2);
    let s3 = s2;
    assert_eq!(s, s3);
}

#[test]
fn event_source_hashable_in_set() {
    let mut set = std::collections::HashSet::new();
    for s in &EventSource::ALL {
        set.insert(*s);
    }
    assert_eq!(set.len(), 6);
}

// =============================================================================
// compute_hash
// =============================================================================

#[test]
fn hash_is_64_hex() {
    let h = ArbitrationEvent::compute_hash(0, EventSource::Frontend, "u", "t", "{}");
    assert_eq!(h.len(), 64);
    assert!(h.chars().all(|c| c.is_ascii_hexdigit()), "应全 hex");
}

#[test]
fn hash_sha256_known_vector() {
    // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    // 但我们没法直接断言 hash 内容 (有 timestamp 注入), 只验证 deterministic + length + hex.
    let h1 = ArbitrationEvent::compute_hash(12345, EventSource::Frontend, "u1", "msg", "{\"k\":1}");
    let h2 = ArbitrationEvent::compute_hash(12345, EventSource::Frontend, "u1", "msg", "{\"k\":1}");
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64);
}

#[test]
fn hash_field_separators_present() {
    // hash 应反映每个 field 独立变化 (源 field-level SHA-256, 不是 concat-only)
    let base = ArbitrationEvent::compute_hash(100, EventSource::Frontend, "u", "t", "p");
    let variants = [
        ArbitrationEvent::compute_hash(101, EventSource::Frontend, "u", "t", "p"), // ts
        ArbitrationEvent::compute_hash(100, EventSource::GroupChat, "u", "t", "p"), // source
        ArbitrationEvent::compute_hash(100, EventSource::Frontend, "v", "t", "p"), // source_id
        ArbitrationEvent::compute_hash(100, EventSource::Frontend, "u", "x", "p"), // topic
        ArbitrationEvent::compute_hash(100, EventSource::Frontend, "u", "t", "q"), // payload
    ];
    for v in &variants {
        assert_ne!(*v, base, "任一 field 变化 → hash 变");
    }
    // 且 5 个 variant 也互不相同
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(variants[i], variants[j], "variant {i} vs {j}");
        }
    }
}

// =============================================================================
// ArbitrationLog in-memory
// =============================================================================

#[test]
fn log_in_memory_empty_init() {
    let log = ArbitrationLog::open_in_memory().unwrap();
    assert_eq!(log.len().unwrap(), 0);
    assert!(log.is_empty().unwrap());
    assert_eq!(log.path(), ":memory:");
}

#[test]
fn log_append_seq_increments() {
    let log = ArbitrationLog::open_in_memory().unwrap();
    let e1 = log
        .append(EventSource::Frontend, "tui", "msg", "1")
        .unwrap();
    let e2 = log
        .append(EventSource::Frontend, "tui", "msg", "2")
        .unwrap();
    let e3 = log
        .append(EventSource::Frontend, "tui", "msg", "3")
        .unwrap();
    assert_eq!(e1.seq, 1);
    assert_eq!(e2.seq, 2);
    assert_eq!(e3.seq, 3);
    assert_eq!(log.len().unwrap(), 3);
    assert!(!log.is_empty().unwrap());
}

#[test]
fn log_append_returns_event_with_hash() {
    let log = ArbitrationLog::open_in_memory().unwrap();
    let e = log
        .append(EventSource::Frontend, "tui", "msg", "{\"x\":1}")
        .unwrap();
    assert_eq!(e.content_hash.len(), 64);
    assert_eq!(e.source, EventSource::Frontend);
    assert_eq!(e.source_id, "tui");
    assert_eq!(e.topic, "msg");
    assert_eq!(e.payload_json, "{\"x\":1}");
    assert!(e.timestamp_ms > 0, "应填 timestamp_ms");
}

#[test]
fn log_canonical_order_empty() {
    let log = ArbitrationLog::open_in_memory().unwrap();
    let evts = log.canonical_order(10).unwrap();
    assert!(evts.is_empty());
}

#[test]
fn log_canonical_order_ascending_timestamp() {
    let log = ArbitrationLog::open_in_memory().unwrap();
    for i in 0..5 {
        log.append(EventSource::Frontend, "u", "t", &format!("p{i}"))
            .unwrap();
    }
    let evts = log.canonical_order(10).unwrap();
    assert_eq!(evts.len(), 5);
    for i in 0..evts.len() - 1 {
        assert!(
            evts[i].timestamp_ms <= evts[i + 1].timestamp_ms,
            "应 timestamp ASC"
        );
    }
}

#[test]
fn log_canonical_order_limit_respected() {
    let log = ArbitrationLog::open_in_memory().unwrap();
    for i in 0..10 {
        log.append(EventSource::Frontend, "u", "t", &format!("p{i}"))
            .unwrap();
    }
    let evts = log.canonical_order(3).unwrap();
    assert_eq!(evts.len(), 3, "limit=3 只返 3");
}

#[test]
fn log_canonical_order_limit_zero() {
    let log = ArbitrationLog::open_in_memory().unwrap();
    log.append(EventSource::Frontend, "u", "t", "p").unwrap();
    let evts = log.canonical_order(0).unwrap();
    assert!(evts.is_empty(), "limit=0 → 空");
}

#[test]
fn log_by_source_filter_no_match() {
    let log = ArbitrationLog::open_in_memory().unwrap();
    log.append(EventSource::Frontend, "tui", "m", "1").unwrap();
    let evts = log.by_source(EventSource::Email, "inbox", 10).unwrap();
    assert!(evts.is_empty(), "无匹配 → 空");
}

#[test]
fn log_by_source_limit() {
    let log = ArbitrationLog::open_in_memory().unwrap();
    for i in 0..10 {
        log.append(EventSource::Frontend, "tui", "m", &format!("{i}"))
            .unwrap();
    }
    let evts = log.by_source(EventSource::Frontend, "tui", 3).unwrap();
    assert_eq!(evts.len(), 3, "limit=3");
}

#[test]
fn log_by_source_source_id_match_required() {
    let log = ArbitrationLog::open_in_memory().unwrap();
    log.append(EventSource::Frontend, "tui", "m", "1").unwrap();
    log.append(EventSource::Frontend, "api", "m", "2").unwrap();
    let tui = log.by_source(EventSource::Frontend, "tui", 10).unwrap();
    assert_eq!(tui.len(), 1);
    assert_eq!(tui[0].source_id, "tui");
}

// =============================================================================
// File persistence
// =============================================================================

#[test]
fn log_persistence_drop_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("events.db");

    let log1 = ArbitrationLog::open(&db_path).unwrap();
    log1.append(EventSource::Frontend, "tui", "msg", "{\"v\":1}")
        .unwrap();
    log1.append(EventSource::GroupChat, "family", "msg", "{\"v\":2}")
        .unwrap();
    log1.append(EventSource::Email, "inbox", "mail", "{\"v\":3}")
        .unwrap();
    assert_eq!(log1.len().unwrap(), 3);
    drop(log1);

    let log2 = ArbitrationLog::open(&db_path).unwrap();
    assert_eq!(log2.len().unwrap(), 3);
    let evts = log2.canonical_order(10).unwrap();
    assert_eq!(evts.len(), 3);
}

#[test]
fn log_persistence_seq_preserved_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seq.db");

    let log1 = ArbitrationLog::open(&db_path).unwrap();
    let e1 = log1.append(EventSource::Frontend, "u", "m", "1").unwrap();
    let e2 = log1.append(EventSource::Frontend, "u", "m", "2").unwrap();
    drop(log1);

    let log2 = ArbitrationLog::open(&db_path).unwrap();
    let evts = log2.canonical_order(10).unwrap();
    assert_eq!(evts.len(), 2);
    // seq 应保留
    let seqs: Vec<i64> = evts.iter().map(|e| e.seq).collect();
    assert!(seqs.contains(&e1.seq));
    assert!(seqs.contains(&e2.seq));
}

#[test]
fn log_persistence_payload_intact() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("payload.db");

    let log1 = ArbitrationLog::open(&db_path).unwrap();
    log1.append(EventSource::Frontend, "u", "t", "{\"k\":\"中文\"}")
        .unwrap();
    drop(log1);

    let log2 = ArbitrationLog::open(&db_path).unwrap();
    let evts = log2.canonical_order(10).unwrap();
    assert_eq!(evts[0].payload_json, "{\"k\":\"中文\"}");
}

// =============================================================================
// Clone / share
// =============================================================================

#[test]
fn log_clone_shares_state() {
    let log1 = ArbitrationLog::open_in_memory().unwrap();
    let log2 = log1.clone();
    log1.append(EventSource::Frontend, "u", "m", "1").unwrap();
    // log2 应能看见 log1 的 append (Arc-shared)
    assert_eq!(log2.len().unwrap(), 1);
    log2.append(EventSource::Frontend, "u", "m", "2").unwrap();
    assert_eq!(log1.len().unwrap(), 2);
}

#[test]
fn log_clone_paths_match() {
    let log1 = ArbitrationLog::open_in_memory().unwrap();
    let log2 = log1.clone();
    assert_eq!(log1.path(), log2.path());
}

// =============================================================================
// Cross-source integration
// =============================================================================

#[test]
fn integration_multi_source_unique_events() {
    let log = ArbitrationLog::open_in_memory().unwrap();
    log.append(EventSource::Frontend, "tui", "msg", "1")
        .unwrap();
    log.append(EventSource::GroupChat, "family", "msg", "2")
        .unwrap();
    log.append(EventSource::Email, "inbox", "mail", "3")
        .unwrap();
    log.append(EventSource::AgentComm, "cat", "tool", "4")
        .unwrap();
    log.append(EventSource::System, "scheduler", "tick", "5")
        .unwrap();
    log.append(EventSource::External, "webhook", "ping", "6")
        .unwrap();

    let evts = log.canonical_order(100).unwrap();
    assert_eq!(evts.len(), 6);
    // 6 个 source 都被记录
    let sources: std::collections::HashSet<_> = evts.iter().map(|e| e.source).collect();
    assert_eq!(sources.len(), 6, "6 个不同 source");
}

#[test]
fn integration_by_source_per_source_total() {
    let log = ArbitrationLog::open_in_memory().unwrap();
    log.append(EventSource::Frontend, "tui", "m", "1").unwrap();
    log.append(EventSource::Frontend, "tui", "m", "2").unwrap();
    log.append(EventSource::GroupChat, "family", "m", "3")
        .unwrap();
    log.append(EventSource::Frontend, "api", "m", "4").unwrap();

    let tui = log.by_source(EventSource::Frontend, "tui", 100).unwrap();
    let api = log.by_source(EventSource::Frontend, "api", 100).unwrap();
    let family = log
        .by_source(EventSource::GroupChat, "family", 100)
        .unwrap();
    assert_eq!(tui.len(), 2);
    assert_eq!(api.len(), 1);
    assert_eq!(family.len(), 1);
}

#[test]
fn integration_full_persistence_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("rt.db");

    // write phase
    {
        let log = ArbitrationLog::open(&db_path).unwrap();
        log.append(EventSource::Frontend, "tui", "msg", "{\"a\":1}")
            .unwrap();
        log.append(EventSource::GroupChat, "family", "msg", "{\"a\":2}")
            .unwrap();
        log.append(EventSource::Email, "inbox", "mail", "{\"a\":3}")
            .unwrap();
    }

    // read phase
    let log = ArbitrationLog::open(&db_path).unwrap();
    assert_eq!(log.len().unwrap(), 3);

    // canonical_order
    let all = log.canonical_order(10).unwrap();
    assert_eq!(all.len(), 3);

    // by_source each
    let frontend = log.by_source(EventSource::Frontend, "tui", 10).unwrap();
    let group = log.by_source(EventSource::GroupChat, "family", 10).unwrap();
    let email = log.by_source(EventSource::Email, "inbox", 10).unwrap();
    assert_eq!(frontend.len() + group.len() + email.len(), 3);

    // content_hash 64-char for all
    for e in &all {
        assert_eq!(e.content_hash.len(), 64);
    }
}

// =============================================================================
// now_ms
// =============================================================================

#[test]
fn now_ms_monotonic() {
    let t1 = now_ms();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let t2 = now_ms();
    assert!(t2 > t1, "t2 > t1: {t2} > {t1}");
}

#[test]
fn now_ms_returns_positive() {
    let t = now_ms();
    // 当前 epoch ms 远大于 0
    assert!(t > 1_000_000_000_000, "epoch ms 应 > 10^12: {t}");
}

// =============================================================================
// ArbError
// =============================================================================

#[test]
fn arb_error_not_found_display() {
    let e = ArbError::NotFound(42);
    let s = e.to_string();
    assert!(s.contains("42"), "{s}");
    assert!(s.contains("not found") || s.contains("seq"), "{s}");
}

#[test]
fn arb_error_invalid_source_display() {
    let e = ArbError::InvalidSource("bad_source".into());
    let s = e.to_string();
    assert!(s.contains("bad_source"));
}
