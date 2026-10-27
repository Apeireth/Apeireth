//! `handover` 的测试：把「**换人接手**」变成**机器可判**的判据。
//!
//! 设计稿 §8 **B4**（脱机归档 + 迁移回滚 + handover 演练）+ §6.1 **L5 可继承**
//! （脱机可读 · schema 自说明覆盖 · **handover 演练通过**）+ §2 **P7**
//! （组织中断/人员交接 ⇒ handover 清单）+ §2 **P1**（数据保真优先）。
//!
//! 重点不在「包里有没有字」，而在**纪律不许漂**：
//!
//! * **能读懂**：字段说明必须**覆盖包内数据的全部字段**（漏一个 ⇒ 判失败）；
//! * **知道现状**：状态里的条数必须与归档物里的条数**一致**（不一致 ⇒ 判失败）；
//! * **知道下一步**：每条下一步都得有**可跑的验收判据**（没判据 ⇒ 判失败）；
//! * **知道红线**：必须含 P1 的「**不删原始证据**」（缺 ⇒ 判失败）；
//! * **不粉饰**：已知问题照抄（调用方给什么就写什么）；
//! * **确定性**：同输入 ⇒ 逐字相等（包与演练结果都是）。
//!
//! ⚠️ 本文件**不碰**任何生产路径：用的是**内存库 + 真实 schema**（跑过真实迁移）
//! ⇒ 导出物与生产同构，但**不是**生产库。

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::handover::{
    build_package, default_red_lines, drill_handover, render_package_summary, CurrentState,
    HandoverPackage, NextStep, P1_RED_LINE_MARKER, REQUIRED_FIELD_DOCS,
};
use apeireth_memory::offline_archive::{
    export_archive, to_jsonl, verify_archive, ExportConfig, OfflineArchive,
};
use apeireth_memory::SqliteMemoryStore;

/// 生成时刻（**注入** ⇒ 确定性；测试里固定 ⇒ 两次构建逐字相等）。
const CREATED_AT_MS: i64 = 1_760_000_000_000;

/// 往库里写一条**带链**的记录（生产默认写路径 ⇒ 自动入链）。
fn put(store: &SqliteMemoryStore, id: &str, subject: &str, ts: i64, content: &str) {
    let episode = Episode {
        id: id.to_string(),
        timestamp: ts,
        role: "user".to_string(),
        content: content.to_string(),
        session_id: "sess-handover".to_string(),
    };
    store
        .put_episode_for_subject(&episode, subject)
        .expect("写入带链记录");
}

/// 造一份**真实**的归档物（真实 schema 的内存库 ⇒ 导出物与生产同构）。
fn real_archive() -> OfflineArchive {
    let store = SqliteMemoryStore::open_in_memory().expect("打开内存库");
    put(&store, "ep-1", "cid-a", 1_000, "第一段：此開卷第一回也．");
    put(&store, "ep-2", "cid-a", 2_000, "第二段：賈雨村夤緣复舊職．");
    put(&store, "ep-3", "cid-b", 3_000, "第三段：林黛玉拋父進京都．");
    let conn = store.conn().expect("拿连接");
    export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS).expect("导出")
}

/// 一份**如实**的状态（条数全部从归档物里取 ⇒ 不会自己骗自己）。
fn sample_state(archive: &OfflineArchive) -> CurrentState {
    CurrentState {
        db_path: "/tmp/apeireth-handover-demo.sqlite3".to_string(),
        episode_count: archive.episodes.len(),
        subject_count: archive.chain_heads.len(),
        unmanaged_count: archive.manifest.unmanaged_count,
        current_algorithm: Some("hybrid（BM25 + 向量）".to_string()),
        known_issues: vec![
            "recall@10 = 0.2983（《紅樓夢》120 回 / 476 题实测）—— 73% 的题目标记录没进检索结果"
                .to_string(),
        ],
    }
}

/// 下一步清单（每条都带**可跑的验收判据** ⇒ 演练才判得过）。
fn sample_next_steps() -> Vec<NextStep> {
    vec![
        NextStep {
            what: "把召回缺口拆到检索层：先查候选池覆盖率".to_string(),
            why: "73% 的题目标记录压根没进候选池 ⇒ 注入层再优化也没用".to_string(),
            acceptance: "cargo run -p apeireth-memory --example golden_diag ⇒ 逐题列出候选池覆盖；\
                         候选池覆盖率 ≥ 0.45（基线 0.2712）"
                .to_string(),
            cost: 2,
        },
        NextStep {
            what: "把 handover 包接到 `memory handover` CLI 子命令".to_string(),
            why: "本批只到库级（NOT IMPLEMENTED），生产 0 调用点 ⇒ 接手者拿不到包".to_string(),
            acceptance: "cargo run -p apeireth-cli -- memory handover --out pkg.json ⇒ \
                         产出的包与库级示例逐字相等"
                .to_string(),
            cost: 1,
        },
    ]
}

/// 造一份**完整**的包（演练应当**全部通过**）。
fn complete_package() -> HandoverPackage {
    let archive = real_archive();
    let state = sample_state(&archive);
    build_package(archive, state, sample_next_steps(), CREATED_AT_MS)
}

// ============================================================
// 一、build_package：产出完整包
// ============================================================

#[test]
fn build_package_produces_a_complete_package() {
    let package = complete_package();

    assert!(
        !package.version.is_empty(),
        "包必须有版本（改了结构就要升）"
    );
    assert_eq!(
        package.created_at_ms, CREATED_AT_MS,
        "生成时刻是**注入**的 ⇒ 必须是调用方给的值"
    );
    assert!(!package.data.is_empty(), "数据引用清单不许为空");
    assert!(!package.next_steps.is_empty(), "下一步清单不许为空");
    assert!(!package.red_lines.is_empty(), "红线清单不许为空");
    assert_eq!(
        package.archive.episodes.len(),
        3,
        "包必须**自带**归档物（接手者不依赖原库也能读）"
    );
    assert_eq!(
        package.state.episode_count,
        package.archive.episodes.len(),
        "状态里的条数必须与归档物一致"
    );
}

#[test]
fn build_package_is_deterministic() {
    // ⚠️ **确定性口径**：同一个**归档物** ⇒ 同一个包。
    //
    // 为什么不能「建两次库再比」：链头账本的 `updated_at_ms` 是**墙钟**
    // （写链时取的当前时刻）⇒ 两个**不同时刻**建出来的库，那一列必然不同
    // ⇒ 那不是包不确定，是**两个不同的输入**。
    // （`offline_archive` 自己的口径也是「**同库**两次导出逐字相等」，与此一致。）
    let archive = real_archive();
    let state = sample_state(&archive);
    let first = build_package(
        archive.clone(),
        state.clone(),
        sample_next_steps(),
        CREATED_AT_MS,
    );
    let second = build_package(archive, state, sample_next_steps(), CREATED_AT_MS);

    assert_eq!(
        first, second,
        "同输入 ⇒ 包**逐字相等**（⇒ 可复核、可 diff）"
    );
    assert_eq!(
        first.created_at_ms, second.created_at_ms,
        "生成时刻是注入的 ⇒ 不许随时间漂"
    );
}

#[test]
fn build_package_carries_the_default_red_lines() {
    let package = complete_package();

    assert_eq!(
        package.red_lines,
        default_red_lines(),
        "每份包都**必须**带上仓库既有的硬约束（红线不许被调用方稀释掉）"
    );
}

#[test]
fn build_package_keeps_known_issues_verbatim() {
    let archive = real_archive();
    let mut state = sample_state(&archive);
    state.known_issues = vec![
        "recall@10 = 0.2983 —— 瓶颈是召回".to_string(),
        "哈希链只覆盖 D0 主表".to_string(),
    ];
    let package = build_package(archive, state, sample_next_steps(), CREATED_AT_MS);

    assert_eq!(
        package.state.known_issues.len(),
        2,
        "已知问题**照抄**（不粉饰、不合并、不代填）"
    );
    assert!(
        package.state.known_issues[0].contains("0.2983"),
        "坏消息必须原样带过去：{:?}",
        package.state.known_issues
    );
}

#[test]
fn build_package_keeps_the_archive_untouched() {
    let archive = real_archive();
    let original = archive.clone();
    let state = sample_state(&archive);
    let package = build_package(archive, state, sample_next_steps(), CREATED_AT_MS);

    assert_eq!(
        package.archive, original,
        "包只**携带**归档物，不许改它一个字节（保原文）"
    );
    assert_eq!(
        package.archive.episodes[0].content, "第一段：此開卷第一回也．",
        "**保原文**：正文必须逐字不变"
    );
}

#[test]
fn build_package_describes_every_data_ref() {
    let package = complete_package();

    for item in &package.data {
        assert!(!item.what.trim().is_empty(), "数据引用要说清「是什么」");
        assert!(!item.where_.trim().is_empty(), "数据引用要说清「在哪」");
        assert!(
            !item.how_to_read.trim().is_empty(),
            "数据引用要说清「怎么读」（接手者不用猜）"
        );
        assert!(
            !item.trust.trim().is_empty(),
            "数据引用要说清「可信度」（不然接手者不知道能不能信）"
        );
    }
    // 至少一条引用指向**包内**的归档物 ⇒ 原库没了也能读。
    assert!(
        package
            .data
            .iter()
            .any(|item| item.where_.contains("archive")),
        "必须有一条引用指向包内归档物：{:?}",
        package.data
    );
}

// ============================================================
// 二、default_red_lines：仓库硬约束
// ============================================================

#[test]
fn default_red_lines_are_non_empty_and_cover_the_house_rules() {
    let lines = default_red_lines();

    assert!(!lines.is_empty(), "红线清单不许为空");
    let joined = lines
        .iter()
        .map(|line| line.forbidden.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        joined.contains("不删原始证据"),
        "必须含 P1 的「不删原始证据」（实际：{joined}）"
    );
    assert!(
        joined.contains("不改基线"),
        "必须含「不改基线」（实际：{joined}）"
    );
    assert!(
        joined.contains("口径不混写"),
        "必须含「口径不混写」（实际：{joined}）"
    );
    assert!(
        joined.contains("不猜着读数据"),
        "必须含「不猜着读数据」（实际：{joined}）"
    );
}

#[test]
fn default_red_lines_explain_why() {
    for line in default_red_lines() {
        assert!(
            !line.because.trim().is_empty(),
            "每条红线都得说清「为什么」（否则接手者会绕过它）：{}",
            line.forbidden
        );
    }
}

#[test]
fn default_red_lines_are_deterministic() {
    assert_eq!(
        default_red_lines(),
        default_red_lines(),
        "红线是**纪律**不是随机数 ⇒ 两次取必须逐字相等"
    );
}

// ============================================================
// 三、drill_handover：四件事（核心）
// ============================================================

#[test]
fn drill_passes_for_a_complete_package() {
    let package = complete_package();

    let (passed, notes) = drill_handover(&package);

    assert!(passed, "完整包必须**全部通过**（实际说明：{notes:?}）");
    assert_eq!(notes.len(), 4, "逐项说明恒为 4 条（一项一条）");
    for note in &notes {
        assert!(note.starts_with("✅"), "每条都要给出通过与否：{note}");
    }
}

#[test]
fn drill_detects_a_mismatched_episode_count() {
    let mut package = complete_package();
    package.state.episode_count = 99;

    let (passed, notes) = drill_handover(&package);

    assert!(!passed, "状态里的条数与归档物不符 ⇒ 必须判失败");
    assert_eq!(notes.len(), 4);
    assert!(
        notes[1].contains("❌") && notes[1].contains("条数"),
        "第 ② 项必须明确说「条数不符」：{}",
        notes[1]
    );
    // 其余三项仍应通过（⇒ 失败是**精确定位**的，不是「整包报废」）。
    assert!(notes[0].starts_with("✅"), "第 ① 项应仍通过：{}", notes[0]);
    assert!(notes[2].starts_with("✅"), "第 ③ 项应仍通过：{}", notes[2]);
    assert!(notes[3].starts_with("✅"), "第 ④ 项应仍通过：{}", notes[3]);
}

#[test]
fn drill_fails_when_next_steps_are_empty() {
    let mut package = complete_package();
    package.next_steps.clear();

    let (passed, notes) = drill_handover(&package);

    assert!(!passed, "没有下一步 ⇒ 接手者不知道该干什么 ⇒ 必须判失败");
    assert!(
        notes[2].contains("❌") && notes[2].contains("下一步"),
        "第 ③ 项必须点名：{}",
        notes[2]
    );
}

#[test]
fn drill_fails_when_a_next_step_has_no_acceptance() {
    let mut package = complete_package();
    package.next_steps[1].acceptance = String::new();

    let (passed, notes) = drill_handover(&package);

    assert!(!passed, "没验收判据的下一步**不算下一步** ⇒ 必须判失败");
    assert!(
        notes[2].contains("❌") && notes[2].contains("验收判据"),
        "第 ③ 项必须点名「验收判据」：{}",
        notes[2]
    );
}

#[test]
fn drill_fails_when_a_next_step_has_no_what() {
    let mut package = complete_package();
    package.next_steps[0].what = "   ".to_string();

    let (passed, notes) = drill_handover(&package);

    assert!(!passed, "没写「做什么」的下一步 ⇒ 必须判失败");
    assert!(notes[2].contains("❌"), "第 ③ 项必须判失败：{}", notes[2]);
}

#[test]
fn drill_fails_when_the_p1_red_line_is_missing() {
    let mut package = complete_package();
    package
        .red_lines
        .retain(|line| !line.forbidden.contains(P1_RED_LINE_MARKER));

    let (passed, notes) = drill_handover(&package);

    assert!(!passed, "缺 P1 的「不删原始证据」⇒ 必须判失败");
    assert!(
        notes[3].contains("❌") && notes[3].contains(P1_RED_LINE_MARKER),
        "第 ④ 项必须点名缺了哪条：{}",
        notes[3]
    );
}

#[test]
fn drill_fails_when_field_docs_miss_a_field() {
    let mut package = complete_package();
    package.archive.manifest.field_docs.remove("content");

    let (passed, notes) = drill_handover(&package);

    assert!(!passed, "字段说明**要覆盖全部字段** ⇒ 漏一个字段就判失败");
    assert!(
        notes[0].contains("❌") && notes[0].contains("content"),
        "第 ① 项必须点名缺了哪个字段：{}",
        notes[0]
    );
}

#[test]
fn drill_fails_when_a_field_doc_is_blank() {
    let mut package = complete_package();
    package
        .archive
        .manifest
        .field_docs
        .insert("content".to_string(), "  ".to_string());

    let (passed, notes) = drill_handover(&package);

    assert!(!passed, "空串的字段说明等于没说明 ⇒ 必须判失败");
    assert!(
        notes[0].contains("❌") && notes[0].contains("content"),
        "第 ① 项必须点名：{}",
        notes[0]
    );
}

#[test]
fn drill_fails_when_the_data_refs_are_incomplete() {
    let mut package = complete_package();
    package.data[0].how_to_read = String::new();

    let (passed, notes) = drill_handover(&package);

    assert!(!passed, "没说清「怎么读」⇒ 接手者第一件事就得问人 ⇒ 判失败");
    assert!(
        notes[0].contains("❌") && notes[0].contains("怎么读"),
        "第 ① 项必须点名：{}",
        notes[0]
    );
}

#[test]
fn drill_reports_exactly_one_line_per_check() {
    let package = complete_package();

    let (passed, notes) = drill_handover(&package);

    assert!(passed);
    assert_eq!(notes.len(), 4, "四件事 ⇒ 四条说明");
    assert!(notes[0].contains("能读懂"), "第 ① 项：{}", notes[0]);
    assert!(notes[1].contains("知道现状"), "第 ② 项：{}", notes[1]);
    assert!(notes[2].contains("知道下一步"), "第 ③ 项：{}", notes[2]);
    assert!(notes[3].contains("知道红线"), "第 ④ 项：{}", notes[3]);
}

#[test]
fn drill_is_deterministic() {
    let package = complete_package();

    let first = drill_handover(&package);
    let second = drill_handover(&package);

    assert_eq!(
        first, second,
        "同一份包 ⇒ 演练结果逐字相等（⇒ 可当回归基线）"
    );
}

#[test]
fn drill_fails_when_everything_is_wrong_at_once() {
    let mut package = complete_package();
    package.state.episode_count = 0;
    package.next_steps.clear();
    package.red_lines.clear();
    package.archive.manifest.field_docs.clear();

    let (passed, notes) = drill_handover(&package);

    assert!(!passed, "全错 ⇒ 必须判失败");
    assert_eq!(notes.len(), 4);
    for note in &notes {
        assert!(note.starts_with("❌"), "每项都要判失败：{note}");
    }
}

// ============================================================
// 四、render_package_summary：人话报告
// ============================================================

#[test]
fn render_package_summary_mentions_state_issues_red_lines_and_next_steps() {
    let package = complete_package();

    let summary = render_package_summary(&package);

    // 状态
    assert!(
        summary.contains("/tmp/apeireth-handover-demo.sqlite3"),
        "摘要必须写清库路径：{summary}"
    );
    assert!(
        summary.contains("记录条数：3"),
        "摘要必须写清记录条数：{summary}"
    );
    assert!(
        summary.contains("hybrid（BM25 + 向量）"),
        "摘要必须写清当前检索算法：{summary}"
    );
    // 已知问题（**不隐瞒坏消息**）
    assert!(
        summary.contains("已知问题（1 条）"),
        "摘要必须给出已知问题的条数：{summary}"
    );
    assert!(
        summary.contains("recall@10 = 0.2983"),
        "摘要必须把坏消息原样写出来：{summary}"
    );
    // 红线与下一步的**条数**
    assert!(
        summary.contains("红线（4 条）"),
        "摘要必须给出红线条数：{summary}"
    );
    assert!(
        summary.contains("下一步（2 条）"),
        "摘要必须给出下一步条数：{summary}"
    );
    // 红线内容与下一步的验收判据都要在（否则「先看这个」没用）
    assert!(
        summary.contains("不删原始证据"),
        "摘要必须写清红线：{summary}"
    );
    assert!(
        summary.contains("候选池覆盖率 ≥ 0.45"),
        "摘要必须写清验收判据：{summary}"
    );
}

#[test]
fn render_package_summary_is_deterministic() {
    let package = complete_package();

    assert_eq!(
        render_package_summary(&package),
        render_package_summary(&package),
        "同一份包 ⇒ 摘要逐字相等"
    );
}

#[test]
fn render_package_summary_says_so_when_there_are_no_known_issues() {
    let archive = real_archive();
    let mut state = sample_state(&archive);
    state.known_issues.clear();
    let package = build_package(archive, state, sample_next_steps(), CREATED_AT_MS);

    let summary = render_package_summary(&package);

    assert!(
        summary.contains("已知问题（0 条）"),
        "没有已知问题也要**如实写出来**（不许让空清单看着像「没问题」）：{summary}"
    );
}

// ============================================================
// 五、包的自足性（**不依赖原库**也能读、能核对）
// ============================================================

#[test]
fn the_package_is_self_contained() {
    let package = complete_package();

    verify_archive(&package.archive).expect("包内归档物必须自洽（接手者可独立核对）");
    let text = to_jsonl(&package.archive);
    assert!(
        text.contains("此開卷第一回也"),
        "包内归档物必须带**原文**（⇒ 脱机可 grep）"
    );
    assert!(
        text.lines().count() >= 5,
        "JSONL 至少含 manifest + 3 条记录 + 1 条链头：{}",
        text.lines().count()
    );
}

#[test]
fn required_field_docs_match_the_archive_schema() {
    let package = complete_package();
    let docs = &package.archive.manifest.field_docs;

    for field in REQUIRED_FIELD_DOCS {
        assert!(
            docs.contains_key(*field),
            "归档物自带的字段说明必须覆盖 `{field}`（实际有：{:?}）",
            docs.keys().collect::<Vec<_>>()
        );
    }
    assert!(
        REQUIRED_FIELD_DOCS.len() >= 22,
        "判据必须覆盖记录 12 + 链头 4 + 清单 6 个字段（实际 {}）",
        REQUIRED_FIELD_DOCS.len()
    );
}
