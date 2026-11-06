//! `offline_archive` 的测试：把「脱机归档」的三条纪律**钉死**。
//!
//! 设计稿 §8 **B4**（脱机归档，保原文）+ §6.1 **L5 可继承**（脱机可读 · schema 自说明覆盖）
//! + §2 **P1**（数据保真优先，原始证据永久保留）。
//!
//! 重点不在「导得出东西」，而在**纪律不许漂**：
//!
//! * **保原文**（核心断言）：内容含换行 / 引号 / 反斜杠 / 中文 ⇒ 导出后**逐字相同**；
//! * **确定性**：同一库两次导出 ⇒ manifest 与 episodes **逐字相等**；
//! * **不许说谎**：manifest 的条数 = 实际导出的条数；未纳管的记录**如实标出**；
//! * **可验证**：篡改正文 / 篡改 manifest / 账实不符 ⇒ `verify_archive` **必须**报错；
//! * **可读**：JSONL 往返相等、每行独立可解析、坏 JSONL 如实报错（不 panic）。

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::offline_archive::{
    content_digest, default_field_docs, export_archive, from_jsonl, to_jsonl, verify_archive,
    ArchivedEpisode, ExportConfig, ARCHIVE_FORMAT_VERSION,
};
use apeireth_memory::SqliteMemoryStore;
use rusqlite::{params, Connection};

/// 导出时刻（**注入** ⇒ 确定性；测试里固定 ⇒ 两次导出逐字相等）。
const CREATED_AT_MS: i64 = 1_760_000_000_000;

/// 造一个**空库**（跑过真实迁移 ⇒ 列集与生产一致）。
fn empty_store() -> SqliteMemoryStore {
    SqliteMemoryStore::open_in_memory().expect("打开内存库")
}

/// 往库里写一条**带链**的记录（生产默认写路径 ⇒ 自动入链）。
fn put(store: &SqliteMemoryStore, id: &str, subject: &str, ts: i64, content: &str) {
    let episode = Episode {
        id: id.to_string(),
        timestamp: ts,
        role: "user".to_string(),
        content: content.to_string(),
        session_id: "sess-1".to_string(),
    };
    store
        .put_episode_for_subject(&episode, subject)
        .expect("写入带链记录");
}

/// 往库里塞一条**未纳管**的记录（`chain_hash` 留 NULL）。
///
/// **为什么要手工塞**：真实写路径全部入链 ⇒ 想测「未纳管」这条语义，
/// 只能绕过应用层直接 INSERT（`episodes` 的 append-only 触发器只拦 UPDATE/DELETE）。
fn put_unmanaged(store: &SqliteMemoryStore, id: &str, subject: &str, ts: i64, content: &str) {
    let conn = store.conn().expect("拿连接");
    conn.execute(
        "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, subject, "sess-1", ts, "user", content],
    )
    .expect("插入未纳管记录");
}

// ============================================================
// export_archive：基本形态 + 边界
// ============================================================

#[test]
fn empty_library_exports_an_empty_archive() {
    let store = empty_store();
    let conn = store.conn().expect("拿连接");
    let archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");

    assert!(archive.episodes.is_empty(), "空库 ⇒ 没有记录");
    assert!(archive.chain_heads.is_empty(), "空库 ⇒ 没有链头");
    assert_eq!(archive.manifest.episode_count, 0, "manifest 条数必须是 0");
    assert_eq!(
        archive.manifest.chain_head_count, 0,
        "manifest 链头条数必须是 0"
    );
    assert_eq!(archive.manifest.unmanaged_count, 0, "空库 ⇒ 未纳管 0 条");
    assert_eq!(
        archive.manifest.format_version, ARCHIVE_FORMAT_VERSION,
        "格式版本必须写进 manifest（读方据此判断能不能读）"
    );
    assert_eq!(
        archive.manifest.created_at_ms, CREATED_AT_MS,
        "导出时刻是**注入**的 ⇒ 必须是调用方给的值"
    );
    assert!(
        !archive.manifest.field_docs.is_empty(),
        "自说明表不许为空（随物走 ⇒ 三年后仍可读）"
    );
    verify_archive(&archive).expect("空归档必须自洽");
}

#[test]
fn bare_connection_without_schema_does_not_panic() {
    // 裸连接（连迁移都没跑）⇒ 没有 `episodes` 表 ⇒ 导出**空归档**（不是错，也不是崩）。
    let conn = Connection::open_in_memory().expect("打开裸连接");
    let archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS)
        .expect("没有表 ⇒ 导出空归档，不该报错");
    assert!(archive.episodes.is_empty());
    assert_eq!(archive.manifest.episode_count, 0);
}

// ============================================================
// 保原文（**核心断言**）
// ============================================================

#[test]
fn content_is_preserved_verbatim() {
    let store = empty_store();
    // 故意用**难搞**的内容：换行 / 双引号 / 反斜杠 / 中文 / 制表符 / emoji。
    //
    // ⚠️ 倒数第二行**以反斜杠结尾** ⇒ 字符串里真的存在「反斜杠 + 换行」这个组合
    // （早先那版正文里反斜杠后面跟的是空格 ⇒ 断言 `contains("\\\n")` 永远为假
    //  ⇒ 断言本身写错了，不是导出丢字 ⇒ 这里把内容补成真组合）。
    let nasty = "第一行\n第二行\t带制表符\n\"引号里的内容\" \\ 反斜杠\n以反斜杠结尾\\\n中文段落：记忆归档不许丢字 😀";
    put(&store, "ep-verbatim", "cid-a", 1_700_000_000, nasty);

    let conn = store.conn().expect("拿连接");
    let archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");

    assert_eq!(archive.episodes.len(), 1);
    assert_eq!(
        archive.episodes[0].content, nasty,
        "**保原文**：导出后的正文必须与库里**逐字相同**（不许摘要 / 截断 / 改写）"
    );
    // 逐字 ⇒ 字节长度也一样（中文按 UTF-8 字节算，不是字符数）。
    assert_eq!(
        archive.episodes[0].content.len(),
        nasty.len(),
        "逐字 ⇒ 字节长度必须一致"
    );
    assert!(
        archive.episodes[0].content.contains('\n'),
        "换行必须原样保留（不许被压成空格）"
    );
    assert!(
        archive.episodes[0].content.contains("\\\n"),
        "反斜杠 + 换行的组合必须原样保留"
    );
}

#[test]
fn export_is_deterministic() {
    let store = empty_store();
    put(&store, "ep-b", "cid-a", 2_000, "第二条");
    put(&store, "ep-a", "cid-a", 1_000, "第一条");
    put(&store, "ep-c", "cid-b", 3_000, "第三条");

    let conn = store.conn().expect("拿连接");
    let first = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("第一次导出");
    let second = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("第二次导出");

    assert_eq!(
        first.manifest, second.manifest,
        "同库两次导出 ⇒ manifest 逐字相等"
    );
    assert_eq!(
        first.episodes, second.episodes,
        "同库两次导出 ⇒ 记录逐字相等"
    );
    assert_eq!(
        first
            .episodes
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        vec!["ep-a", "ep-b", "ep-c"],
        "排序按 (timestamp, id) 升序 ⇒ 确定性"
    );
}

// ============================================================
// 过滤与截断（**不许说谎**）
// ============================================================

#[test]
fn subject_filter_selects_only_that_subject() {
    let store = empty_store();
    put(&store, "ep-a1", "cid-a", 1_000, "A 主体第一条");
    put(&store, "ep-a2", "cid-a", 2_000, "A 主体第二条");
    put(&store, "ep-b1", "cid-b", 3_000, "B 主体第一条");

    let conn = store.conn().expect("拿连接");
    let config = ExportConfig {
        subject: Some("cid-a".to_string()),
        ..ExportConfig::new()
    };
    let archive = export_archive(&conn, &config, CREATED_AT_MS).expect("导出");

    assert_eq!(archive.episodes.len(), 2, "只导 cid-a ⇒ 2 条");
    assert!(
        archive.episodes.iter().all(|e| e.continuity_id == "cid-a"),
        "不许混进别的主体"
    );
    assert_eq!(
        archive.manifest.episode_count, 2,
        "manifest 条数 == 实际导出条数"
    );
    assert_eq!(archive.chain_heads.len(), 1, "链头也只导该主体 ⇒ 1 条");
    assert_eq!(archive.chain_heads[0].subject_id, "cid-a");
}

#[test]
fn limit_truncates_and_the_manifest_tells_the_truth() {
    let store = empty_store();
    for index in 0..5 {
        put(
            &store,
            &format!("ep-{index}"),
            "cid-a",
            1_000 + index,
            &format!("第 {index} 条"),
        );
    }

    let conn = store.conn().expect("拿连接");
    let config = ExportConfig {
        limit: 2,
        ..ExportConfig::new()
    };
    let archive = export_archive(&conn, &config, CREATED_AT_MS).expect("导出");

    assert_eq!(archive.episodes.len(), 2, "limit = 2 ⇒ 只导 2 条");
    assert_eq!(
        archive.manifest.episode_count, 2,
        "**不许说谎**：manifest 的条数必须是实际导出的条数（不是库里总条数）"
    );
    assert_eq!(
        archive
            .episodes
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        vec!["ep-0", "ep-1"],
        "截断取的是**前** N 条（排序确定 ⇒ 取哪几条也是确定的）"
    );
}

#[test]
fn include_unmanaged_false_skips_unmanaged_records() {
    let store = empty_store();
    put(&store, "ep-chained", "cid-a", 1_000, "带链的记录");
    put_unmanaged(&store, "ep-loose", "cid-a", 2_000, "没链的记录");

    let conn = store.conn().expect("拿连接");
    let config = ExportConfig {
        include_unmanaged: false,
        ..ExportConfig::new()
    };
    let archive = export_archive(&conn, &config, CREATED_AT_MS).expect("导出");

    assert_eq!(archive.episodes.len(), 1, "未纳管的记录必须被跳过");
    assert_eq!(archive.episodes[0].id, "ep-chained");
    assert_eq!(
        archive.manifest.unmanaged_count, 0,
        "口径：unmanaged_count 恒等于**导出物里**的无链条数（被跳过的就不在导出物里）"
    );
    assert_eq!(
        archive.manifest.episode_count, 1,
        "manifest 条数 == 实际导出条数"
    );
    verify_archive(&archive).expect("跳过后仍必须自洽");
}

#[test]
fn unmanaged_records_are_flagged_in_the_manifest() {
    let store = empty_store();
    put(&store, "ep-chained", "cid-a", 1_000, "带链的记录");
    put_unmanaged(&store, "ep-loose", "cid-a", 2_000, "没链的记录");

    let conn = store.conn().expect("拿连接");
    let archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");

    assert_eq!(
        archive.episodes.len(),
        2,
        "默认**要**导出未纳管的记录（如实）"
    );
    assert_eq!(
        archive.manifest.unmanaged_count, 1,
        "**如实标出**：1 条记录没在哈希链保护范围内"
    );
    let loose = archive
        .episodes
        .iter()
        .find(|episode| episode.id == "ep-loose")
        .expect("未纳管记录必须在导出物里");
    assert!(
        loose.chain_hash.is_none(),
        "未纳管的记录在导出物里必须**如实标成**没有链摘要（不许假装有链）"
    );
    assert!(loose.chain_prev_hash.is_none());
}

// ============================================================
// 链字段与链头（账实相符的原料）
// ============================================================

#[test]
fn chain_fields_and_chain_heads_are_exported_verbatim() {
    let store = empty_store();
    put(&store, "ep-1", "cid-a", 1_000, "第一条");
    put(&store, "ep-2", "cid-a", 2_000, "第二条");

    let conn = store.conn().expect("拿连接");
    let archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");

    for episode in &archive.episodes {
        assert!(
            episode.chain_hash.is_some(),
            "生产写路径的记录必须带链摘要（导出物里也要如实带出来）"
        );
        assert!(episode.chain_prev_hash.is_some(), "链前驱也要导出来");
    }
    assert_eq!(archive.chain_heads.len(), 1, "一个主体 ⇒ 一条链头");
    let head = &archive.chain_heads[0];
    assert_eq!(head.subject_id, "cid-a");
    assert_eq!(head.entry_count, 2, "链头记的条目数 == 链上条数");
    assert_eq!(
        head.last_hash,
        archive.episodes[1].chain_hash.clone().unwrap(),
        "链头摘要 == 最后一条记录的链摘要"
    );
    assert_eq!(archive.manifest.chain_head_count, 1);
}

// ============================================================
// content_digest：能查出「改了」与「重排」
// ============================================================

/// 造一条记录（单元测试用，不必真库）。
fn episode(id: &str, content: &str) -> ArchivedEpisode {
    ArchivedEpisode {
        id: id.to_string(),
        continuity_id: "cid-a".to_string(),
        session_id: Some("sess-1".to_string()),
        timestamp: 1_000,
        role: "user".to_string(),
        content: content.to_string(),
        valid_from_ms: Some(1_000_000),
        valid_until_ms: None,
        created_ms: Some(1_000_000),
        provenance: Some("dialog".to_string()),
        chain_hash: Some("hash-1".to_string()),
        chain_prev_hash: Some("hash-0".to_string()),
    }
}

#[test]
fn content_digest_is_stable_for_the_same_content() {
    let left = vec![episode("e1", "你好"), episode("e2", "世界")];
    let right = vec![episode("e1", "你好"), episode("e2", "世界")];
    assert_eq!(
        content_digest(&left),
        content_digest(&right),
        "同内容 ⇒ 同哈希（确定性 ⇒ 可复核）"
    );
    assert_eq!(content_digest(&left).len(), 64, "SHA-256 ⇒ 64 个 hex 字符");
    assert!(
        content_digest(&left)
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "哈希必须是**小写 hex**"
    );
}

#[test]
fn content_digest_changes_when_one_character_changes() {
    let before = vec![episode("e1", "今天天气很好")];
    let after = vec![episode("e1", "今天天气很糟")];
    assert_ne!(
        content_digest(&before),
        content_digest(&after),
        "**改一个字** ⇒ 哈希必须变（能查出改动）"
    );
}

#[test]
fn content_digest_changes_when_the_order_changes() {
    let left = vec![episode("e1", "第一条"), episode("e2", "第二条")];
    let right = vec![episode("e2", "第二条"), episode("e1", "第一条")];
    assert_ne!(
        content_digest(&left),
        content_digest(&right),
        "**换顺序** ⇒ 哈希必须变（能查出重排）"
    );
}

#[test]
fn content_digest_of_an_empty_archive_is_the_empty_sha256() {
    // SHA-256(空) = e3b0c442...（**不是空串** ⇒ 空与非空能区分）。
    assert_eq!(
        content_digest(&[]),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

// ============================================================
// verify_archive：独立校验（能查出改动）
// ============================================================

#[test]
fn verify_archive_accepts_a_self_consistent_archive() {
    let store = empty_store();
    put(&store, "ep-1", "cid-a", 1_000, "第一条");
    put(&store, "ep-2", "cid-a", 2_000, "第二条");

    let conn = store.conn().expect("拿连接");
    let archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");
    verify_archive(&archive).expect("刚导出的归档物必须自洽");
}

#[test]
fn verify_archive_detects_tampered_content() {
    let store = empty_store();
    put(&store, "ep-1", "cid-a", 1_000, "原始内容");

    let conn = store.conn().expect("拿连接");
    let mut archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");
    archive.episodes[0].content = "篡改后的内容".to_string();

    let error = verify_archive(&archive).expect_err("篡改正文必须被查出来");
    assert!(
        error.contains("内容哈希"),
        "错误必须说清差在哪（实际：{error}）"
    );
}

#[test]
fn verify_archive_detects_a_tampered_manifest_count() {
    let store = empty_store();
    put(&store, "ep-1", "cid-a", 1_000, "第一条");

    let conn = store.conn().expect("拿连接");
    let mut archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");
    archive.manifest.episode_count = 99;

    let error = verify_archive(&archive).expect_err("篡改 manifest 条数必须被查出来");
    assert!(
        error.contains("条数"),
        "错误必须说清差在哪（实际：{error}）"
    );
}

#[test]
fn verify_archive_detects_a_chain_ledger_mismatch() {
    let store = empty_store();
    put(&store, "ep-1", "cid-a", 1_000, "第一条");
    put(&store, "ep-2", "cid-a", 2_000, "第二条");

    let conn = store.conn().expect("拿连接");
    let mut archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");
    // 改链头的账（条目数）⇒ 导出物里的链上条数与账不符。
    archive.chain_heads[0].entry_count = 5;

    let error = verify_archive(&archive).expect_err("账实不符必须被查出来");
    assert!(
        error.contains("链账不符"),
        "错误必须说清差在哪（实际：{error}）"
    );
}

#[test]
fn verify_archive_detects_a_tampered_unmanaged_count() {
    let store = empty_store();
    put(&store, "ep-1", "cid-a", 1_000, "第一条");
    put_unmanaged(&store, "ep-loose", "cid-a", 2_000, "没链的记录");

    let conn = store.conn().expect("拿连接");
    let mut archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");
    archive.manifest.unmanaged_count = 0;

    let error = verify_archive(&archive).expect_err("篡改未纳管条数必须被查出来");
    assert!(
        error.contains("未纳管"),
        "错误必须说清差在哪（实际：{error}）"
    );
}

#[test]
fn verify_archive_detects_a_tampered_chain_head_count() {
    let store = empty_store();
    put(&store, "ep-1", "cid-a", 1_000, "第一条");

    let conn = store.conn().expect("拿连接");
    let mut archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");
    archive.manifest.chain_head_count = 99;

    let error = verify_archive(&archive).expect_err("篡改链头条数必须被查出来");
    assert!(
        error.contains("链头条数"),
        "错误必须说清差在哪（实际：{error}）"
    );
}

#[test]
fn truncated_archive_fails_the_chain_ledger_check() {
    // **如实记录一条设计后果**：截断导出（limit < 全量）按设计**通不过**链账核对
    // ⇒ 这正是「这份归档不完整」的信号（不是误报）。
    let store = empty_store();
    for index in 0..3 {
        put(
            &store,
            &format!("ep-{index}"),
            "cid-a",
            1_000 + index,
            &format!("第 {index} 条"),
        );
    }
    let conn = store.conn().expect("拿连接");
    let config = ExportConfig {
        limit: 2,
        ..ExportConfig::new()
    };
    let archive = export_archive(&conn, &config, CREATED_AT_MS).expect("导出");
    let error = verify_archive(&archive).expect_err("截断的归档物不该自称完整");
    assert!(error.contains("链账不符"), "实际：{error}");
}

// ============================================================
// JSONL：可 grep、可 jq、可被程序读回来
// ============================================================

#[test]
fn jsonl_roundtrip_preserves_the_archive() {
    let store = empty_store();
    put(&store, "ep-1", "cid-a", 1_000, "第一行\n第二行");
    put_unmanaged(&store, "ep-loose", "cid-b", 2_000, "没链的记录");

    let conn = store.conn().expect("拿连接");
    let archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");

    let text = to_jsonl(&archive);
    let restored = from_jsonl(&text).expect("读回来");
    assert_eq!(restored, archive, "往返必须**逐字相等**");
}

#[test]
fn jsonl_lines_are_independently_parsable() {
    let store = empty_store();
    put(&store, "ep-1", "cid-a", 1_000, "第一条");
    put(&store, "ep-2", "cid-a", 2_000, "第二条");

    let conn = store.conn().expect("拿连接");
    let archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");
    let text = to_jsonl(&archive);
    let lines: Vec<&str> = text.lines().collect();

    assert_eq!(lines.len(), 4, "1 行 manifest + 2 行记录 + 1 行链头");
    for line in &lines {
        serde_json::from_str::<serde_json::Value>(line)
            .unwrap_or_else(|error| panic!("每行都必须能独立解析（可 grep / 可 jq）：{error}"));
    }
    let first: serde_json::Value = serde_json::from_str(lines[0]).expect("解析首行");
    assert_eq!(
        first.get("kind").and_then(|v| v.as_str()),
        Some("manifest"),
        "**第一行必须是 manifest**（读方据此知道字段含义与版本）"
    );
    assert!(
        first.get("field_docs").is_some(),
        "manifest 必须自带字段说明（自说明 ⇒ 随物走）"
    );
    // 正文出现在文本里 ⇒ 可以直接 `grep` 到原文。
    assert!(
        text.contains("第二条"),
        "正文必须以**原文**出现在 JSONL 里（⇒ 可 grep）"
    );
}

#[test]
fn from_jsonl_rejects_broken_input() {
    // 坏 JSON ⇒ 如实报错（不 panic）。
    let broken = "{\"kind\": \"manifest\", oops}\n";
    let error = from_jsonl(broken).expect_err("坏 JSONL 必须报错");
    assert!(error.contains("JSON"), "实际：{error}");

    // 缺 manifest ⇒ 报错。
    // （用一条**字段齐全**的 episode 行 ⇒ 它本身能解析 ⇒ 才会走到「缺 manifest」这条判据。）
    let episode_only = concat!(
        "{\"kind\": \"episode\", \"id\": \"x\", \"continuity_id\": \"c\", \"session_id\": null, ",
        "\"timestamp\": 1, \"role\": \"user\", \"content\": \"c\", \"valid_from_ms\": null, ",
        "\"valid_until_ms\": null, \"created_ms\": null, \"provenance\": null, ",
        "\"chain_hash\": null, \"chain_prev_hash\": null}\n"
    );
    let error = from_jsonl(episode_only).expect_err("缺 manifest 必须报错");
    assert!(error.contains("manifest"), "实际：{error}");

    // 未知 kind ⇒ 报错（**如实**，不猜）。
    let manifest_line = concat!(
        "{\"kind\": \"manifest\", \"format_version\": \"v\", \"created_at_ms\": 0, ",
        "\"episode_count\": 0, \"chain_head_count\": 0, \"unmanaged_count\": 0, ",
        "\"field_docs\": {}, \"content_digest\": \"d\"}\n"
    );
    let error = from_jsonl(&format!("{manifest_line}{{\"kind\": \"weird\"}}\n"))
        .expect_err("未知 kind 必须报错");
    assert!(error.contains("weird"), "实际：{error}");

    // 空文本 ⇒ 报错。
    assert!(from_jsonl("").is_err(), "空文本必须报错");
}

#[test]
fn from_jsonl_rejects_an_unknown_format_version() {
    // 原则 **P4**（格式演进 ⇒ **未知版本显式拒**，不猜着读）：
    // 读到一个不认识的 `format_version` ⇒ 如实报错，不做「尽力而为」的猜测解析。
    let foreign = concat!(
        "{\"kind\": \"manifest\", \"format_version\": \"apeireth-offline-archive/v999\", ",
        "\"created_at_ms\": 0, \"episode_count\": 0, \"chain_head_count\": 0, ",
        "\"unmanaged_count\": 0, \"field_docs\": {}, \"content_digest\": \"d\"}\n"
    );
    let error = from_jsonl(foreign).expect_err("未知版本必须被拒");
    assert!(
        error.contains("版本"),
        "错误必须说清差在哪（实际：{error}）"
    );
}

// ============================================================
// 自说明覆盖（§6.1 L5 的另一半）
// ============================================================

#[test]
fn default_field_docs_contains_the_key_fields() {
    let docs = default_field_docs();
    assert!(!docs.is_empty(), "字段说明表不许为空");
    for key in ["content", "chain_hash", "valid_from_ms"] {
        assert!(
            docs.contains_key(key),
            "关键字段 {key} 必须有说明（实际有：{:?}）",
            docs.keys().collect::<Vec<_>>()
        );
        assert!(
            !docs
                .get(key)
                .map(String::as_str)
                .unwrap_or_default()
                .is_empty(),
            "字段 {key} 的说明不许是空串"
        );
    }
    // 时间语义必须写清楚（秒 vs 毫秒 ⇒ 读方不会读错）。
    assert!(
        docs.get("timestamp")
            .is_some_and(|text| text.contains("epoch 秒")),
        "timestamp 的时间单位必须写明"
    );
    assert!(
        docs.get("created_ms")
            .is_some_and(|text| text.contains("epoch 毫秒")),
        "created_ms 的时间单位必须写明"
    );
    // 未纳管的语义必须写清楚（读方才知道哪些数据没链保护）。
    assert!(
        docs.get("chain_hash")
            .is_some_and(|text| text.contains("未纳管")),
        "chain_hash 的「未纳管」语义必须写明"
    );
}

#[test]
fn every_export_carries_the_field_docs() {
    let store = empty_store();
    put(&store, "ep-1", "cid-a", 1_000, "第一条");
    let conn = store.conn().expect("拿连接");
    let archive = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");
    assert_eq!(
        archive.manifest.field_docs,
        default_field_docs(),
        "**每次导出都带上**字段说明（不许省略）"
    );
}

// ============================================================
// 只读纪律（导出不得改库）
// ============================================================

#[test]
fn export_does_not_touch_the_library() {
    let store = empty_store();
    put(&store, "ep-1", "cid-a", 1_000, "第一条");
    put_unmanaged(&store, "ep-loose", "cid-a", 2_000, "没链的记录");

    let before = {
        let conn = store.conn().expect("拿连接");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
            .expect("数记录");
        let content: String = conn
            .query_row(
                "SELECT content FROM episodes WHERE id = 'ep-1'",
                [],
                |row| row.get(0),
            )
            .expect("读正文");
        (count, content)
    };

    {
        let conn = store.conn().expect("拿连接");
        let _ = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出");
        let _ = export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("再导出");
    }

    let after = {
        let conn = store.conn().expect("拿连接");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
            .expect("数记录");
        let content: String = conn
            .query_row(
                "SELECT content FROM episodes WHERE id = 'ep-1'",
                [],
                |row| row.get(0),
            )
            .expect("读正文");
        (count, content)
    };
    assert_eq!(before, after, "导出是**只读**的 ⇒ 不许改库里任何东西");
}
