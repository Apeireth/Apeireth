//! `apeireth-memory::forgetting_metrics` 的**演练报告**（真实库 ⇒ 降权 → 检索 → 归档 → 检索 → 销毁）。
//!
//! # 这个例子回答什么
//!
//! 设计稿 §6.2 的四个「遗忘」指标里，**保真可达率**是最要紧的一个
//! （它是「降权/归档 ≠ 丢失」的**机器证明**），**越权销毁恒 0** 是安全红线。
//! ⇒ 这个例子在**真实 SQLite 库**上把整条链跑一遍：
//!
//! 1. **降权**：真实调权引擎（负使用证据 ⇒ 条目显著性下调 ⇒ **纯排序层**，数据不动）；
//! 2. **检索**：真实 BM25 候选源 + 真实混合检索管线（`HybridRetrievalPipeline`）；
//! 3. **归档**：真实脱机归档导出（`offline_archive::export_archive`，**只 SELECT** ⇒ 库里数据一条不动）
//!    + 独立校验（`verify_archive`）；
//! 4. **再检索**：对每条归档件，用**真实检索**测「明确需要时能不能找到它」
//!    ⇒ **不许假设**（检索不到就是 0）；
//! 5. **销毁**：真实治理 sidecar（授权 + 墓碑）＋ **尝试销毁受保护记录 ⇒ 必须被拒**。
//!
//! # 口径（钉死，别自创）
//!
//! * **零分不算命中**：BM25 源会**按 id 顺序填满** limit ⇒ 分数为 0 的文档也会被返回
//!   ⇒ 只保留 **score > 0** 的结果（与 `golden_run::run` 同口径）⇒ 不许「假命中」；
//! * **降权的数据不动**：演练前后**记录条数与正文逐条比对** ⇒ 「纯排序层」可复核；
//! * **越权销毁恒 0**：受保护记录的销毁尝试**被治理层拒绝** ⇒ **没有发生销毁**
//!   ⇒ 不进销毁台账 ⇒ 越权数 0（拒绝本身会打印出来，不藏着）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径：
//!
//! - **IMPLEMENTED** ✔：能在**真实库**上跑出完整演练报告（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产 0 调用点（只有人工在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✘：只在**自建临时库**上实测过，**未在生产记忆库上实测过**
//!   （本机无生产库）⇒ 不宣称已验证。

use std::process::ExitCode;

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::forgetting_metrics::{
    judge_retrievable, render_report, summarize, ArchivedItem, DestructionRecord, DownweightedItem,
};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::memory_governance::{MemoryGovernanceError, MemoryGovernanceStore};
use apeireth_memory::offline_archive::{export_archive, verify_archive, ExportConfig};
use apeireth_memory::retrieval_pipeline::{
    Bm25LexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidateSource,
};
use apeireth_memory::retrieval_tuning::{
    clear_effective, effective_ranking_or, install_effective, RetrievalDimension,
    RetrievalTuningEngine, TuningEvidence,
};
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};
use apeireth_memory::{EpisodeQuery, EpisodeStore, SqliteMemoryStore};

/// 演练用的会话 id（真实写进 `episodes.session_id`）。
const SESSION: &str = "forgetting-drill";
/// 演练用的查询（「部署回滚」是这批语料的主题）。
const QUERY: &str = "部署回滚";
/// 检索的 top-k（真实检索 ⇒ 取前 k 个）。
const TOP_K: usize = 4;
/// 注入字符预算（真实管线的预算门）。
const MAX_CHARS: usize = 4_000;
/// 被降权的条目（真实调权引擎的调权对象）。
const DOWNWEIGHT_TARGET: &str = "mem-01";
/// 降权轮数（真实调权：每轮**一步**，走廊 ±0.05 ⇒ 5 步到下界）。
const DOWNWEIGHT_ROUNDS: usize = 5;
/// 语料（真实写进 `episodes` 表）。
const CORPUS: [(&str, &str); 8] = [
    (
        "mem-01",
        "部署回滚手册：部署前先备份数据库，部署失败就按手册回滚到上一个稳定版本。",
    ),
    (
        "mem-02",
        "部署回滚手册：部署前先备份数据库，部署失败就按手册回滚；回滚完成后要写复盘报告。",
    ),
    (
        "mem-03",
        "灰度发布流程：先灰度一半流量，观察错误率，异常时回滚部署。",
    ),
    (
        "mem-04",
        "回滚脚本放在运维仓库里，部署失败时直接执行回滚脚本。",
    ),
    (
        "mem-05",
        "数据库迁移与部署的顺序：先迁移数据库再部署应用，失败时回滚数据库迁移。",
    ),
    (
        "mem-06",
        "部署窗口定在每周三晚上，部署期间暂停变更，必要时回滚。",
    ),
    ("mem-07", "监控告警规则：部署之后错误率上升就要立刻回滚。"),
    ("mem-08", "回滚演练记录：上一次部署回滚总共耗时十二分钟。"),
];

fn main() -> ExitCode {
    println!("════════ 遗忘指标演练报告（设计稿 §6.2）════════");
    println!("口径提醒：本口径是「降权 + 仍可检索」，**不是**删除式遗忘");
    println!("          ⇒ 不得与拟人系统的 selective forgetting 分数直接比较。");
    println!();

    // ── ① 真实库 ──────────────────────────────────────────────
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("forgetting-drill.sqlite");
    let store = SqliteMemoryStore::open(&db_path).expect("建真实 SQLite 库");
    println!("── ① 真实库 ──");
    println!("  库路径：{}", db_path.display());
    for (index, (id, content)) in CORPUS.iter().enumerate() {
        store
            .put_episode(&Episode {
                id: (*id).to_string(),
                timestamp: 1_700_000_000 + index as i64,
                role: "user".to_string(),
                content: (*content).to_string(),
                session_id: SESSION.to_string(),
            })
            .expect("写入 episode");
    }
    let episodes_before = store
        .query(&EpisodeQuery::new().for_session(SESSION))
        .expect("读库");
    println!(
        "  记录条数：{}（真实写入 `episodes` 表）",
        episodes_before.len()
    );

    // ── ② 检索（真实 BM25 + 真实管线）──────────────────────────
    let source = build_source(&episodes_before);
    let pipeline = HybridRetrievalPipeline::new(apeireth_memory::MemoryRankingConfig::default());
    // ⚠️ **秩位位移**量的是「**完整名次表**」里的位置 ⇒ 这里取全量（不限 top-k）；
    // 限 top-k 会把被降权到预算外的条目**整个丢掉** ⇒ 位移就量不到了。
    let before = retrieve_ids(&pipeline, &source, QUERY, CORPUS.len());
    println!();
    println!("── ② 降权前检索（真实 BM25 + 真实管线；全量名次表）──");
    print_ranking("  ", &before);

    // ── ③ 降权（真实调权引擎：负证据 ⇒ 条目显著性下调）─────────
    println!();
    println!("── ③ 降权（真实调权引擎；纯排序层，数据不动）──");
    let mut engine = RetrievalTuningEngine::new();
    for round in 0..DOWNWEIGHT_ROUNDS {
        let records = engine.observe(&TuningEvidence::negative(
            DOWNWEIGHT_TARGET,
            Some(RetrievalDimension::Lexical),
            1_700_000_000 + round as i64,
        ));
        println!(
            "  第 {} 轮负证据 ⇒ 移动 {} 条账（条目显著性 → {:.4}）",
            round + 1,
            records.len(),
            engine.salience(DOWNWEIGHT_TARGET)
        );
    }
    install_effective(engine.effective());
    // 生效值装进**真实管线**的排序配置 ⇒ 维度权重与条目显著性都真的生效。
    let tuned_pipeline = HybridRetrievalPipeline::new(effective_ranking_or(
        apeireth_memory::MemoryRankingConfig::default(),
    ));
    let after = retrieve_ids(&tuned_pipeline, &source, QUERY, CORPUS.len());
    println!("  降权后检索（同一份语料、同一套真实管线）：");
    print_ranking("    ", &after);

    // 数据没动（可复核）：条数 + 逐条正文比对。
    let episodes_after = store
        .query(&EpisodeQuery::new().for_session(SESSION))
        .expect("读库");
    let unchanged = episodes_before.len() == episodes_after.len()
        && episodes_before
            .iter()
            .zip(episodes_after.iter())
            .all(|(a, b)| a.id == b.id && a.content == b.content);
    println!("  ✓ 纯排序层核验：降权前后记录条数与正文**逐条相等** = {unchanged}（数据没动）");

    // 秩位位移：只对**真被降权**的条目（未被降权的不进分母）。
    let downweighted: Vec<DownweightedItem> = DOWNWEIGHT_TARGET
        .split('|')
        .filter_map(|id| {
            // ⚠️ 元素是 `(String, f64)`（id, 得分）⇒ 比较要用 `as_str()`（否则 `String == &str` 编译不过）。
            let rank_before = before.iter().position(|(hid, _)| hid.as_str() == id)?;
            let rank_after = after.iter().position(|(hid, _)| hid.as_str() == id)?;
            Some(DownweightedItem {
                id: id.to_string(),
                rank_before,
                rank_after,
            })
        })
        .collect();
    println!(
        "  降权条目名次：{} ⇒ {}（位移 = {:+} 位）",
        downweighted
            .iter()
            .map(|item| format!("{} #{}", item.id, item.rank_before))
            .collect::<Vec<_>>()
            .join(", "),
        downweighted
            .iter()
            .map(|item| format!("{} #{}", item.id, item.rank_after))
            .collect::<Vec<_>>()
            .join(", "),
        downweighted
            .first()
            .map(|item| item.rank_after as i64 - item.rank_before as i64)
            .unwrap_or(0)
    );

    // ── ④ 归档（真实脱机归档导出；只 SELECT ⇒ 数据不动）────────
    println!();
    println!("── ④ 归档（真实脱机归档导出 + 独立校验）──");
    let created_at_ms = now_ms();
    let archive = {
        let conn = store.conn().expect("取连接");
        export_archive(&conn, &ExportConfig::new(), created_at_ms).expect("导出归档")
    };
    match verify_archive(&archive) {
        Ok(()) => println!(
            "  ✓ 归档物自洽（条数 {} + 内容哈希 {}）",
            archive.manifest.episode_count,
            &archive.manifest.content_digest[..12]
        ),
        Err(error) => {
            eprintln!("· 归档物**不自洽**：{error}");
            return ExitCode::FAILURE;
        }
    }
    let archived_ids: Vec<String> = archive.episodes.iter().map(|e| e.id.clone()).collect();
    println!(
        "  归档件 {} 条（**库里数据一条没动** ⇒ 归档 ≠ 删除）",
        archived_ids.len()
    );

    // ── ⑤ 再检索：保真可达率（**真实检索**测，不许假设）────────
    println!();
    println!("── ⑤ 保真可达率（每条归档件都用**真实检索**测）──");
    println!("  口径：检索预算 = top-{TOP_K}（**注入给模型的就是前 k 条** ⇒ 预算外 = 检索不到）；");
    println!("        「零分不算命中」（score > 0 才算真命中，与 `golden_run::run` 同口径）。");
    // 「明确需要」的客观依据 = 考卷里点名要它的题。
    let golden = build_golden(&archived_ids);
    let destroyed_target = "mem-03";
    let mut archived_items: Vec<ArchivedItem> = Vec::new();
    for id in &archived_ids {
        let destroyed = id == destroyed_target;
        // 真实检索：走同一套 BM25 + 管线 ⇒ **不许假设**。
        let hits = |pipeline: &HybridRetrievalPipeline, query: &str| -> Vec<String> {
            retrieve_ids(pipeline, &source, query, TOP_K)
                .into_iter()
                .filter(|(_, score)| *score > 0.0)
                .map(|(id, _)| id)
                .collect()
        };
        let retrievable = judge_retrievable(id, &golden, |case| hits(&tuned_pipeline, &case.query));
        // 诊断：**降权前**同样的问法找不找得到（⇒ 区分「本来就没命中」与「被降权挤出去了」）。
        let retrievable_before =
            judge_retrievable(id, &golden, |case| hits(&pipeline, &case.query));
        let verdict = if destroyed {
            "已销毁（**不进分母**：销毁的不要求可达）"
        } else if retrievable {
            "明确需要时**检索到了** ✓"
        } else {
            "明确需要时**检索不到** ✗"
        };
        let before_verdict = if retrievable_before { "✓" } else { "✗" };
        println!("  · {id} ⇒ {verdict}（降权前：{before_verdict}）");
        archived_items.push(ArchivedItem {
            id: id.clone(),
            archived: true,
            destroyed,
            retrievable_when_needed: retrievable,
        });
    }

    // ── ⑥ 销毁（真实治理 sidecar：授权 + 墓碑；受保护的必须被拒）──
    println!();
    println!("── ⑥ 销毁（真实治理 sidecar）──");
    let mut destructions: Vec<DestructionRecord> = Vec::new();
    // 6a) 授权 + 墓碑：正常销毁一条未受保护的记录。
    let governed = store
        .get_governed(destroyed_target)
        .expect("读治理态")
        .expect("有治理态");
    match store.forget_episode(
        destroyed_target,
        Some("用户请求：删除这条调试记录"),
        governed.revision,
    ) {
        Ok(_) => {
            let tombstone = read_tombstone(&store, destroyed_target);
            println!(
                "  · 销毁 {destroyed_target}（授权：用户请求）⇒ 成功；墓碑：{}",
                if tombstone { "有" } else { "**无**" }
            );
            destructions.push(DestructionRecord {
                id: destroyed_target.to_string(),
                authorized: true,
                has_tombstone: tombstone,
                protected: false,
            });
        }
        Err(error) => {
            eprintln!("· 销毁 {destroyed_target} 失败：{error}");
            return ExitCode::FAILURE;
        }
    }
    // 6b) 受保护记录：销毁尝试**必须被拒** ⇒ 没有发生销毁 ⇒ 越权销毁 = 0。
    let protected_id = "mem-08";
    let protected_rev = store
        .get_governed(protected_id)
        .expect("读治理态")
        .expect("有治理态")
        .revision;
    store
        .protect_episode(protected_id, protected_rev)
        .expect("加保护");
    match store.forget_episode(protected_id, Some("越权尝试"), protected_rev + 1) {
        Ok(_) => {
            eprintln!("· **受保护记录被销毁了 —— 事故**（治理层本该拒绝）");
            return ExitCode::FAILURE;
        }
        Err(MemoryGovernanceError::Protected(id)) => {
            println!("  · 尝试销毁受保护记录 {id} ⇒ **被治理层拒绝**（Protected）");
            println!("    ⇒ **没有发生销毁** ⇒ 不进销毁台账 ⇒ 越权销毁数 = 0（拒绝本身如实打印，不藏着）");
        }
        Err(error) => {
            eprintln!("· 销毁受保护记录返回了别的错误：{error}");
            return ExitCode::FAILURE;
        }
    }

    // ── ⑦ 汇总 + 报告（原样输出）──────────────────────────────
    let report = summarize(&downweighted, &archived_items, &destructions);
    println!();
    println!("── ⑦ 四个指标（`render_report` 原样输出）──");
    println!("{}", render_report(&report));

    // ── ⑧ 探针：证明检测器**能**算出越权（人为注入，不是真事件）──
    println!();
    println!("── ⑧ 探针（**人为注入**一条越权记录 ⇒ 检测器必须报警；这不是真事件）──");
    let probe = vec![DestructionRecord {
        id: "probe-protected".to_string(),
        authorized: true,
        has_tombstone: true,
        protected: true,
    }];
    let (_probe_rate, probe_unauthorized) =
        apeireth_memory::forgetting_metrics::measure_destruction(&probe);
    println!("  探针结果：越权销毁数 = {probe_unauthorized}（必须 ≥ 1 ⇒ 检测器没瞎）");

    clear_effective();
    ExitCode::SUCCESS
}

/// 从真实库里读出的记录 ⇒ BM25 候选源（**真实语料**）。
fn build_source(episodes: &[Episode]) -> Bm25LexicalCandidateSource {
    let documents = episodes
        .iter()
        .map(|episode| MemoryCandidate {
            id: episode.id.clone(),
            layer: "episode".to_string(),
            scope: MemoryScope::Global,
            content: episode.content.clone(),
            score: 0.0,
            score_components: Default::default(),
            provenance: MemoryProvenance {
                source: "sqlite_episodes".to_string(),
                source_session: Some(episode.session_id.clone()),
                source_trace: None,
                source_request: None,
            },
        })
        .collect();
    Bm25LexicalCandidateSource::new(documents)
}

/// 真实检索（混合管线 + BM25 源）⇒ 返回 `(id, score)`（按名次）。
///
/// `limit` = 真实管线的 top-k（**秩位位移**用**全量名次表**测，
/// **保真可达率**用**检索预算内的名次表**测 ⇒ 两个口径不许混）。
fn retrieve_ids(
    pipeline: &HybridRetrievalPipeline,
    source: &Bm25LexicalCandidateSource,
    query: &str,
    limit: usize,
) -> Vec<(String, f64)> {
    let sources: [&dyn MemoryCandidateSource; 1] = [source];
    pipeline
        .retrieve(query, &[MemoryScope::Global], &sources, limit, MAX_CHARS)
        .expect("真实检索")
        .into_iter()
        .map(|candidate| (candidate.id, candidate.score))
        .collect()
}

/// 打印一次检索的名次表。
fn print_ranking(prefix: &str, ranking: &[(String, f64)]) {
    if ranking.is_empty() {
        println!("{prefix}(空)");
        return;
    }
    for (index, (id, score)) in ranking.iter().enumerate() {
        println!("{prefix}#{index} {id}（score = {score:.4}）");
    }
}

/// 读治理 sidecar 里的墓碑（**真的查库**，不靠假设）。
fn read_tombstone(store: &SqliteMemoryStore, episode_id: &str) -> bool {
    let conn = store.conn().expect("取连接");
    let row: Option<(String, Option<i64>)> = conn
        .query_row(
            "SELECT status, forgotten_at FROM episode_governance WHERE episode_id = ?1",
            rusqlite::params![episode_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();
    matches!(row, Some((status, Some(_))) if status == "forgotten")
}

/// 考卷：每条归档件一道**点名要它**的题（⇒ 「明确需要」的客观依据）。
fn build_golden(archived_ids: &[String]) -> GoldenSet {
    let cases: Vec<GoldenCase> = archived_ids
        .iter()
        .map(|id| GoldenCase {
            id: format!("case-{id}"),
            query: query_for(id),
            expect_ids: vec![id.clone()],
            expect_fragments: Vec::new(),
            tags: vec!["forgetting".to_string(), "reachability".to_string()],
        })
        .collect();
    GoldenSet::new("forgetting-drill-v1", cases)
}

/// 每条归档件的「明确需要」问法（**问得不一样** ⇒ 不能靠字面抄答案）。
fn query_for(id: &str) -> String {
    match id {
        "mem-01" => "部署前要备份什么，失败了怎么回滚".to_string(),
        "mem-02" => "回滚完成后还要做什么".to_string(),
        "mem-03" => "灰度发布一半流量后怎么判断".to_string(),
        "mem-04" => "回滚脚本放在哪里".to_string(),
        "mem-05" => "数据库迁移和上线的先后顺序".to_string(),
        "mem-06" => "每周什么时候可以部署".to_string(),
        "mem-07" => "什么情况下要立刻回滚".to_string(),
        "mem-08" => "上一次回滚花了多久".to_string(),
        other => format!("关于 {other} 的细节"),
    }
}

/// 当前时刻（epoch 毫秒）。
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}
