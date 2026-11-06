//! `wiki_corpus_real` 的契约测试（真实语料 → 挖题器的接线面）。
//!
//! 钉住几件事：
//! 1. **读得对**：多个日记文件按日期字典序读出记录，出处能指回 `文件#序号`；
//! 2. **坏数据不崩**：坏 JSON 文件计入 `skipped_files`，缺 `body` 的元素计入 `skipped_entries`，
//!    目录不存在 ⇒ 空 + 不 panic；
//! 3. **题指回真实记录**：`expect_ids` 都能在记录里找到，`expect_fragments` **真的在原文里**；
//! 4. **对不上的片段被剔除**（不静默保留假的）；
//! 5. **样本可读**：按 `(kind, query)` 排序、每条一行、`limit` 生效；
//! 6. **闭环**：`to_jsonl` 产出的卷子能被 `GoldenSet::load_jsonl` 读回来；
//! 7. **完全确定**：同一输入两次跑，逐字相等。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use apeireth_memory::golden_mining::{MinedCase, MiningConfig};
use apeireth_memory::golden_set::GoldenSet;
use apeireth_memory::wiki_corpus_real::{
    build_cases, default_diary_root, load_diary_records, render_samples, to_jsonl, verify_case,
    DiaryDoc, SourcedRecord,
};

/// 临时日记目录：进程 id + 计数器保证唯一，退出时清理。
struct TempDiary {
    path: PathBuf,
}

impl TempDiary {
    fn new(tag: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "apeireth-wiki-corpus-real-{}-{tag}-{seq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("创建临时日记目录");
        Self { path }
    }

    fn write(&self, name: &str, content: &str) {
        std::fs::write(self.path.join(name), content).expect("写入临时日记文件");
    }

    fn root(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDiary {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// 一天两页的合成语料（够长、够像人话 ⇒ 挖得出题；含跨记录共享词 ⇒ 挖得出跨记录题）。
const DAY_ONE: &str = r#"[
  {"source": "user", "body": "记忆检索的召回延迟今天压到了十毫秒。压测结果显示召回没有掉点。", "timestamp_epoch_ms": 1790000000000},
  {"source": "user", "body": "今天决定把记忆索引换成 sqlite 全文检索。原因是我们需要更稳的召回。", "timestamp_epoch_ms": 1790000100000}
]"#;

/// 第二天两页（与第一天共享「召回延迟 / 压测」⇒ 跨记录题的前提）。
const DAY_TWO: &str = r#"[
  {"source": "user", "body": "召回延迟的压测结果今天也确认了。召回路径没有明显抖动，指标达标。", "timestamp_epoch_ms": 1790086400000},
  {"source": "user", "body": "接入层的限流阈值今天定在每秒两千次。压测跑完没有明显抖动。", "timestamp_epoch_ms": 1790086500000}
]"#;

/// 造一份两天的合成语料目录。
fn two_day_corpus(tag: &str) -> TempDiary {
    let dir = TempDiary::new(tag);
    dir.write("2026-10-25.json", DAY_ONE);
    dir.write("2026-10-26.json", DAY_TWO);
    dir
}

// ============================================================
// 一、读语料
// ============================================================

#[test]
fn loads_two_diary_files_in_date_order() {
    let dir = two_day_corpus("order");
    let (records, skipped_files, skipped_entries) = load_diary_records(dir.root());

    assert_eq!(records.len(), 4, "两天各两条 ⇒ 四条记录");
    assert_eq!(skipped_files, 0, "没有坏文件");
    assert_eq!(skipped_entries, 0, "没有坏元素");

    // 按文件名（日期）字典序：10-25 的两条排在 10-26 的两条前面。
    let origins: Vec<&str> = records.iter().map(|r| r.origin.as_str()).collect();
    assert_eq!(
        origins,
        vec![
            "2026-10-25.json#0",
            "2026-10-25.json#1",
            "2026-10-26.json#0",
            "2026-10-26.json#1",
        ]
    );
    // 记录 id 也按日期递增（同一批语料两次读必得同一批 id）。
    assert!(records[0].record.id < records[1].record.id);
    assert!(records[1].record.id < records[2].record.id);
    assert!(records[2].record.id < records[3].record.id);
}

#[test]
fn skips_malformed_json_file_and_counts_it() {
    let dir = two_day_corpus("badjson");
    dir.write("2026-10-27.json", "{ 这不是 JSON");

    let (records, skipped_files, skipped_entries) = load_diary_records(dir.root());

    assert_eq!(records.len(), 4, "坏文件里的记录一条都不该进来");
    assert_eq!(skipped_files, 1, "坏文件要计入 skipped_files");
    assert_eq!(skipped_entries, 0, "文件级失败不该算元素级跳过");
}

#[test]
fn skips_entries_without_body_and_counts_them() {
    let dir = TempDiary::new("nobody");
    dir.write(
        "2026-10-25.json",
        r#"[
  {"source": "user", "body": "记忆检索的召回延迟今天压到了十毫秒。", "timestamp_epoch_ms": 1790000000000},
  {"source": "user", "timestamp_epoch_ms": 1790000100000},
  {"source": "user", "body": "   ", "timestamp_epoch_ms": 1790000200000},
  "根本不是对象",
  {"source": "user", "body": "接入层的限流阈值今天定在每秒两千次。", "timestamp_epoch_ms": 1790000300000}
]"#,
    );

    let (records, skipped_files, skipped_entries) = load_diary_records(dir.root());

    assert_eq!(records.len(), 2, "只有两条有非空 body");
    assert_eq!(skipped_files, 0);
    assert_eq!(
        skipped_entries, 3,
        "缺 body / 空 body / 非对象 ⇒ 三个元素被跳过"
    );
}

#[test]
fn missing_diary_directory_returns_empty_without_panic() {
    let dir = TempDiary::new("missing");
    let missing = dir.root().join("根本没有这个目录");

    let (records, skipped_files, skipped_entries) = load_diary_records(&missing);

    assert!(records.is_empty(), "目录不存在 ⇒ 空结果，不 panic");
    assert_eq!(skipped_files, 0);
    assert_eq!(skipped_entries, 0);
}

#[test]
fn accepts_day_page_object_shape() {
    // 本仓 `FileDiaryStore` 落盘的是「一日一页」对象（`apeireth dream` 写出来的形状）。
    let dir = TempDiary::new("daypage");
    dir.write(
        "2026-10-25.json",
        r#"{
  "date": "2026-10-25",
  "entries": [
    {"source": "dream", "body": "记忆检索的召回延迟今天压到了十毫秒。压测结果没有掉点。", "timestamp_epoch_ms": 1790000000000}
  ]
}"#,
    );

    let (records, skipped_files, skipped_entries) = load_diary_records(dir.root());

    assert_eq!(records.len(), 1);
    assert_eq!(skipped_files, 0);
    assert_eq!(skipped_entries, 0);
    assert_eq!(
        records[0].record.content,
        "记忆检索的召回延迟今天压到了十毫秒。压测结果没有掉点。"
    );
}

#[test]
fn missing_timestamps_use_file_order_increment() {
    // 缺 `timestamp_epoch_ms` ⇒ 不猜绝对时间（0 打底），但按文件内顺序递增值（可排序）。
    let dir = TempDiary::new("nots");
    dir.write(
        "2026-10-25.json",
        r#"[
  {"source": "user", "body": "记忆检索的召回延迟今天压到了十毫秒。"},
  {"source": "user", "body": "接入层的限流阈值今天定在每秒两千次。"}
]"#,
    );

    let (records, ..) = load_diary_records(dir.root());

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].record.timestamp_ms, 0, "第一条打底 0");
    assert_eq!(records[1].record.timestamp_ms, 1, "第二条按文件内顺序递增");
    assert!(records[0].record.timestamp_ms < records[1].record.timestamp_ms);
}

// ============================================================
// 二、挖题 + 出处校验
// ============================================================

#[test]
fn synthetic_corpus_samples_for_owner_review() {
    // 真实语料在多数机器上只有个位数条目 ⇒ 这里用**合成语料**先给 owner 看
    // 「题像不像人话、样本长什么样」（B0-b 的验收动作：先看 20 条样本）。
    let dir = two_day_corpus("samples");
    let (records, ..) = load_diary_records(dir.root());

    let cases = build_cases(&records, MiningConfig::default());
    let samples = render_samples(&cases, 20);

    eprintln!("—— 合成语料样本 ——\n{samples}");
    assert!(!samples.is_empty(), "合成语料应当挖得出样本");
}

#[test]
fn built_cases_point_back_to_real_records() {
    let dir = two_day_corpus("pointback");
    let (records, ..) = load_diary_records(dir.root());

    let cases = build_cases(&records, MiningConfig::default());

    assert!(!cases.is_empty(), "合成语料应当挖得出题");
    for case in &cases {
        assert!(!case.expect_ids.is_empty(), "每题至少一条标准答案");
        for id in &case.expect_ids {
            assert!(
                records.iter().any(|r| &r.record.id == id),
                "标准答案 id 必须能在记录里找到：{id}"
            );
        }
    }
}

#[test]
fn expect_fragments_really_appear_in_source_text() {
    let dir = two_day_corpus("fragments");
    let (records, ..) = load_diary_records(dir.root());

    let cases = build_cases(&records, MiningConfig::default());

    assert!(!cases.is_empty());
    for case in &cases {
        assert_eq!(
            case.expect_fragments.len(),
            case.expect_ids.len(),
            "片段与记录 id 一一对应"
        );
        for (id, fragment) in case.expect_ids.iter().zip(case.expect_fragments.iter()) {
            let record = records
                .iter()
                .find(|r| &r.record.id == id)
                .expect("标准答案指向的记录必须存在");
            assert!(
                record.record.content.contains(fragment.as_str()),
                "片段必须真的在原文里：{fragment}"
            );
        }
    }
}

#[test]
fn mismatched_fragment_is_dropped_by_verify_case() {
    // 构造一个「必然对不上」的场景：第二条片段根本不在原文里。
    let records = vec![SourcedRecord {
        record: apeireth_memory::golden_mining::MineRecord {
            id: "r1".to_string(),
            content: "记忆检索的召回延迟今天压到了十毫秒。".to_string(),
            title: None,
            timestamp_ms: 0,
            thread_id: None,
        },
        origin: "2026-10-25.json#0".to_string(),
    }];
    let bad_case = MinedCase {
        id: "gm-keyword-0001".to_string(),
        query: "关于召回延迟，我们当时记了什么？".to_string(),
        expect_ids: vec!["r1".to_string(), "r2".to_string()],
        expect_fragments: vec![
            "记忆检索的召回延迟今天压到了十毫秒。".to_string(),
            "这段文字根本不在原文里".to_string(),
        ],
        kind: "keyword".to_string(),
    };

    let verified = verify_case(&bad_case, &records);

    assert!(
        !verified
            .expect_fragments
            .iter()
            .any(|fragment| fragment.contains("根本不在原文里")),
        "对不上的片段必须被剔除，不静默保留"
    );
    assert_eq!(verified.expect_fragments.len(), 1);
    assert_eq!(
        verified.expect_ids.len(),
        1,
        "片段与 id 一一对应 ⇒ 配对的 id 一起剔"
    );
    assert_eq!(verified.expect_ids[0], "r1");
    assert_eq!(
        verified.expect_fragments[0],
        "记忆检索的召回延迟今天压到了十毫秒。"
    );
    assert_eq!(verified.query, bad_case.query, "题目文本不受校验影响");
    assert_eq!(verified.kind, bad_case.kind);
}

#[test]
fn cases_with_unknown_expect_id_are_dropped_from_the_volume() {
    // 标准答案指向一条**不存在**的记录 ⇒ 该题整题丢弃（没有标准答案的题不进卷子）。
    let records = vec![SourcedRecord {
        record: apeireth_memory::golden_mining::MineRecord {
            id: "r1".to_string(),
            content: "记忆检索的召回延迟今天压到了十毫秒。".to_string(),
            title: None,
            timestamp_ms: 0,
            thread_id: None,
        },
        origin: "2026-10-25.json#0".to_string(),
    }];
    let orphan = MinedCase {
        id: "gm-keyword-0002".to_string(),
        query: "关于召回延迟，我们当时记了什么？".to_string(),
        expect_ids: vec!["不存在的记录".to_string()],
        expect_fragments: vec!["记忆检索的召回延迟今天压到了十毫秒。".to_string()],
        kind: "keyword".to_string(),
    };

    let verified = verify_case(&orphan, &records);

    assert!(verified.expect_ids.is_empty());
    assert!(verified.expect_fragments.is_empty());
}

// ============================================================
// 三、样本导出
// ============================================================

/// 造一条样本题（测试用简写）。
fn sample(id: &str, kind: &str, query: &str, id_list: &[&str]) -> MinedCase {
    MinedCase {
        id: id.to_string(),
        query: query.to_string(),
        expect_ids: id_list.iter().map(|value| value.to_string()).collect(),
        expect_fragments: vec!["原文片段".to_string()],
        kind: kind.to_string(),
    }
}

#[test]
fn render_samples_sorted_by_kind_then_query_one_line_each() {
    let cases = vec![
        sample("c3", "keyword", "关于延迟，我们当时记了什么？", &["r3"]),
        sample("c1", "conflict", "关于限流阈值后来是怎么改的？", &["r1"]),
        sample("c2", "keyword", "关于召回，我们当时记了什么？", &["r2"]),
        sample("c4", "conflict", "关于召回后来是怎么改的？", &["r4"]),
    ];

    let rendered = render_samples(&cases, 10);
    let lines: Vec<&str> = rendered.lines().collect();

    assert_eq!(lines.len(), cases.len(), "每条一行");
    assert!(rendered.contains('⇒'), "样本行必须含 ⇒");
    assert!(rendered.contains("expect_ids="), "样本行必须含 expect_ids=");
    assert!(rendered.contains("fragments=["));

    // 按 (kind, query) 排序：conflict 在 keyword 前；同 kind 内按 query 字典序。
    assert!(lines[0].starts_with("[conflict] 关于召回后来是怎么改的？"));
    assert!(lines[1].starts_with("[conflict] 关于限流阈值后来是怎么改的？"));
    assert!(lines[2].starts_with("[keyword] 关于召回，我们当时记了什么？"));
    assert!(lines[3].starts_with("[keyword] 关于延迟，我们当时记了什么？"));
}

#[test]
fn render_samples_respects_limit() {
    let cases = vec![
        sample("c1", "keyword", "关于召回，我们当时记了什么？", &["r1"]),
        sample("c2", "keyword", "关于延迟，我们当时记了什么？", &["r2"]),
        sample(
            "c3",
            "decision",
            "关于限流阈值这件事最后是怎么定的？",
            &["r3"],
        ),
    ];

    assert_eq!(render_samples(&cases, 0), "", "limit = 0 ⇒ 空串");
    assert_eq!(
        render_samples(&cases, 1).lines().count(),
        1,
        "limit = 1 ⇒ 一行"
    );
    assert_eq!(
        render_samples(&cases, 2).lines().count(),
        2,
        "limit = 2 ⇒ 两行"
    );
    assert_eq!(
        render_samples(&cases, 99).lines().count(),
        cases.len(),
        "limit 超过总数 ⇒ 全部"
    );
}

#[test]
fn to_jsonl_roundtrips_through_golden_set() {
    let dir = two_day_corpus("jsonl");
    let (records, ..) = load_diary_records(dir.root());
    let cases = build_cases(&records, MiningConfig::default());
    assert!(!cases.is_empty());

    let jsonl = to_jsonl(&cases);
    let set = GoldenSet::load_jsonl(&jsonl).expect("考卷 JSONL 必须能被读回来");

    assert_eq!(set.len(), cases.len(), "闭环后题数一致");
    for (case, golden) in cases.iter().zip(set.cases.iter()) {
        assert_eq!(golden.id, case.id);
        assert_eq!(golden.query, case.query);
        assert_eq!(golden.expect_ids, case.expect_ids);
        assert_eq!(golden.expect_fragments, case.expect_fragments);
        assert!(
            golden.tags.iter().any(|tag| tag == "internal"),
            "内部卷必须打 internal 标签（与外部卷区分）"
        );
        assert!(golden.tags.iter().any(|tag| tag == "wiki_corpus_real"));
        assert!(
            golden
                .tags
                .iter()
                .any(|tag| tag == &format!("kind:{}", case.kind)),
            "题型标签必须落进 tags"
        );
    }
}

#[test]
fn deterministic_across_two_runs() {
    let dir = two_day_corpus("determinism");
    let config = MiningConfig::default();

    let first = {
        let (records, files, entries) = load_diary_records(dir.root());
        let cases = build_cases(&records, config);
        (
            records,
            files,
            entries,
            render_samples(&cases, 20),
            to_jsonl(&cases),
        )
    };
    let second = {
        let (records, files, entries) = load_diary_records(dir.root());
        let cases = build_cases(&records, config);
        (
            records,
            files,
            entries,
            render_samples(&cases, 20),
            to_jsonl(&cases),
        )
    };

    assert_eq!(first.0, second.0, "记录逐条相等");
    assert_eq!(first.1, second.1);
    assert_eq!(first.2, second.2);
    assert_eq!(first.3, second.3, "样本文本逐字相等");
    assert_eq!(first.4, second.4, "JSONL 逐字相等");
}

#[test]
fn parse_diary_docs_reports_skipped_entries() {
    let text = r#"[
  {"body": "记忆检索的召回延迟今天压到了十毫秒。"},
  {"body": ""},
  {"nothing": true}
]"#;

    let (docs, skipped) = apeireth_memory::wiki_corpus_real::parse_diary_docs(text, "2026-10-25")
        .expect("合法 JSON ⇒ 能解析");

    assert_eq!(skipped, 2);
    assert_eq!(docs.len(), 1);
    assert_eq!(
        docs[0],
        DiaryDoc {
            date: "2026-10-25".to_string(),
            body: "记忆检索的召回延迟今天压到了十毫秒。".to_string(),
        }
    );

    // 非法 JSON ⇒ 文件级失败（None）⇒ 调用方计入 skipped_files。
    assert!(
        apeireth_memory::wiki_corpus_real::parse_diary_docs("{ 坏文件", "2026-10-25").is_none()
    );
}

// ============================================================
// 四、真实语料探查（给 owner 过目的样本出口）
// ============================================================

#[test]
fn real_corpus_probe_prints_samples() {
    // 本机**可能**没有真实日记（没跑过 `apeireth dream`）⇒ 缺席不是失败，只是没样本可看。
    let root = default_diary_root();
    if !root.is_dir() {
        eprintln!(
            "（本机无真实日记目录：{} ⇒ 本次无真实样本）",
            root.display()
        );
        return;
    }

    let (records, skipped_files, skipped_entries) = load_diary_records(&root);
    let cases = build_cases(&records, MiningConfig::default());
    let samples = render_samples(&cases, 20);

    eprintln!(
        "真实语料：{}（记录 {} 条 / 跳过文件 {} / 跳过元素 {}）",
        root.display(),
        records.len(),
        skipped_files,
        skipped_entries
    );
    eprintln!("—— 样本 ——\n{samples}");

    // 顺带把卷子形状给 owner 看一行（JSONL ⇒ 可直接喂 `GoldenSet::load_jsonl`）。
    let jsonl = to_jsonl(&cases);
    if let Some(first) = jsonl.lines().next() {
        eprintln!("—— 考卷 JSONL 首行 ——\n{first}");
    }

    // 真实语料上也必须守住「片段在原文里」这条硬契约。
    for case in &cases {
        for (id, fragment) in case.expect_ids.iter().zip(case.expect_fragments.iter()) {
            let record = records
                .iter()
                .find(|r| &r.record.id == id)
                .expect("标准答案指向的记录必须存在");
            assert!(
                record.record.content.contains(fragment.as_str()),
                "真实语料上的片段必须真的在原文里：{fragment}"
            );
        }
    }
}
