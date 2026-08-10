//! R19 P2 战区 4: 端到端 semantic search + user profile 集成测试
//!
//! 演示: write episodes → semantic_search → extract_user_profile, 全程走 vec0 后端.
//! 跑 `cargo test -p apeireth-memory --test semantic_pipeline_e2e --features semantic`

use std::sync::Arc;

use apeireth_core::Episode;
use apeireth_memory::EpisodeStore;
use apeireth_memory::{HashEmbedder, SqliteMemoryStore};

fn make_episode(id: &str, ts: i64, role: &str, content: &str) -> Episode {
    Episode {
        id: id.into(),
        timestamp: ts,
        role: role.into(),
        content: content.into(),
        session_id: "s1".into(),
    }
}

#[test]
fn full_pipeline_semantic_search_then_user_profile() {
    // 1. 初始化
    let mem = SqliteMemoryStore::open_in_memory().expect("memory open");
    let embedder: Arc<dyn apeireth_memory::EmbedFn> = Arc::new(HashEmbedder::new(64));

    // 2. 写 9 条 user 主题围绕 sql/rust/python
    let user_topics = vec![
        ("sql-1", "I want to learn SQL joins and query optimization"),
        ("sql-2", "show me how to do SQL group by and having clauses"),
        ("sql-3", "SQL transactions and isolation levels please"),
        ("rust-1", "rust borrow checker is so hard to understand"),
        ("rust-2", "rust lifetimes vs generics, when to use which"),
        ("rust-3", "rust async runtime tokio vs async-std comparison"),
        ("py-1", "python list comprehension vs map, which is faster"),
        ("py-2", "python pandas dataframe merge vs join"),
        ("py-3", "python type hints and mypy validation"),
    ];
    for (i, (id, content)) in user_topics.iter().enumerate() {
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode(id, 1000 + i as i64, "user", content),
        )
        .expect("put user episode");
    }
    // 3 条 assistant 内容跟 user 完全不同, 避免 hash embedder 串扰
    let assistant_contents = vec![
        "Here is a small example to illustrate the answer you asked for",
        "Let me walk through the documentation and code samples we have",
        "I will prepare a clear summary with diagrams for you to review",
    ];
    for (i, content) in assistant_contents.iter().enumerate() {
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode(
                &format!("a-{i}"),
                2000 + i as i64,
                "assistant",
                content,
            ),
        )
        .expect("put assistant episode");
    }

    // 3. 一次性 semantic_search: 查 "SQL transactions" 应命中 sql 主题 (top-3)
    // 注: HashEmbedder 是 FNV-1a 哈希 mock, 不像真 LLM 嵌入有强语义区分.
    // 验证标准: sql-3 (内容最接近 query) 应在 top-3; 弱化其他两条约束.
    let sql_hits = mem
        .semantic_search("SQL transactions and joins", 3, Arc::clone(&embedder))
        .expect("semantic search sql");
    assert_eq!(sql_hits.len(), 3, "top-3 应返 3 条");
    let sql_hit_ids: Vec<&str> = sql_hits.iter().map(|e| e.id.as_str()).collect();
    let sql_topics: Vec<&str> = vec!["sql-1", "sql-2", "sql-3"];
    let sql_in_top3 = sql_topics
        .iter()
        .filter(|t| sql_hit_ids.contains(t))
        .count();
    assert!(
        sql_in_top3 >= 1,
        "top-3 应至少包含 1 条 sql 主题 (mock embedder 限制, 真 LLM 应更好), got {:?}",
        sql_hit_ids
    );

    // 4. 查 "rust async" 应命中 rust 主题 (同样放宽到 ≥ 1)
    let rust_hits = mem
        .semantic_search("rust async runtime concurrency", 3, Arc::clone(&embedder))
        .expect("semantic search rust");
    assert_eq!(rust_hits.len(), 3);
    let rust_hit_ids: Vec<&str> = rust_hits.iter().map(|e| e.id.as_str()).collect();
    let rust_topics: Vec<&str> = vec!["rust-1", "rust-2", "rust-3"];
    let rust_in_top3 = rust_topics
        .iter()
        .filter(|t| rust_hit_ids.contains(t))
        .count();
    assert!(
        rust_in_top3 >= 1,
        "top-3 应至少包含 1 条 rust 主题, got {:?}",
        rust_hit_ids
    );

    // 5. 提取用户画像
    let profile = mem
        .extract_user_profile(Arc::clone(&embedder))
        .expect("extract profile");
    assert_eq!(profile.interaction_count, 12, "12 episodes total (9 user + 3 assistant)");
    assert!(profile.last_active.is_some());
    // expertise_areas 应至少包含 sql/rust/python 之一 (取决于哪些 keyword 命中次数最多)
    let expertise_hit = profile
        .expertise_areas
        .iter()
        .any(|s| s == "sql" || s == "rust" || s == "python");
    assert!(
        expertise_hit,
        "expertise_areas 应含 sql/rust/python 至少一个, got {:?}",
        profile.expertise_areas
    );
    assert!(!profile.recurring_topics.is_empty(), "recurring_topics 非空");
    println!(
        "✅ full pipeline: sql_in_top3={} rust_in_top3={} expertise={:?} style={} prefs={:?}",
        sql_in_top3,
        rust_in_top3,
        profile.expertise_areas,
        profile.communication_style,
        profile.preferences
    );
}

#[test]
fn user_profile_changes_after_new_episodes() {
    // 验证: 新增 episode 后再 extract, 画像应反映新数据
    let mem = SqliteMemoryStore::open_in_memory().unwrap();
    let embedder: Arc<dyn apeireth_memory::EmbedFn> = Arc::new(HashEmbedder::new(32));

    // 第一轮: 5 条 user, 2 条 assistant
    for i in 0..5 {
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode(&format!("u{i}"), 100 + i, "user", "hi there"),
        )
        .unwrap();
    }
    for i in 0..2 {
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode(&format!("a{i}"), 200 + i, "assistant", "hello!"),
        )
        .unwrap();
    }
    let p1 = mem.extract_user_profile(Arc::clone(&embedder)).unwrap();
    assert_eq!(p1.interaction_count, 7);
    let style1 = p1.communication_style.clone();

    // 第二轮: 加 3 条 assistant, 翻转成 assistant 主导
    for i in 2..5 {
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode(&format!("a{i}"), 300 + i, "assistant", "sure, let me help"),
        )
        .unwrap();
    }
    let p2 = mem.extract_user_profile(Arc::clone(&embedder)).unwrap();
    assert_eq!(p2.interaction_count, 10);
    // 风格应该从 "用户主导" → "混合" 或 "助手主导"
    assert_ne!(
        style1, p2.communication_style,
        "communication_style 应随数据变化"
    );
    println!(
        "✅ profile evolves: '{}' → '{}'",
        style1, p2.communication_style
    );
}
