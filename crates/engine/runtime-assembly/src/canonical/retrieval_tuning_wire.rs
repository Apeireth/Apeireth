//! 检索维度自进化接线层 (RetrievalTuningWire) —— V4-c 的落地接线。
//!
//! **职责** (引擎本身保持纯确定性、无 IO):
//! - 持有 [`RetrievalTuningEngine`], 把**注入后命中回执** (检索命中注入后、
//!   后续检索轮的选中回执) 解析成使用证据 ([`TuningEvidence`]) 喂给引擎;
//!   本代码库无回合引用标记, 回执是「后续使用」的唯一真值面 (不编造信号);
//! - 引擎每产生一条 [`RetrievalTuningRecord`] → **追加写 `retrieval_tuning.jsonl`**
//!   (数据目录, 落位与 `tuning-log.jsonl` 同目录; 每行一个逐记录信封,
//!   版本戳随记录走) —— 这是「自进化账」(append-only) 的载体;
//! - 把引擎的生效值装进进程内 override, 检索排序使用点实时吃到调权
//!   (调权真生效, 不装学习)。
//!
//! **开关**: `APEIRETH_ENABLE_RETRIEVAL_TUNING=1` 才构造
//! ([`RetrievalTuningWire::from_env`]); 默认关 fail-closed。关 = 无回执、
//! 无证据、无调权、无账。
//!
//! **0 装诚实**:
//! - 持久化失败 fail-open (调整仍生效) 但计入 [`RetrievalTuningWire::log_failures`],
//!   不静默吞掉"账丢了";
//! - 无回执面 (召回路径未提供选中回执) = [`SignalSurface::Absent`] —— **显式
//!   不进化**, 不是"暂时没证据";
//! - 空召回轮不是机会轮 (检索什么都没命中 = 对已注入条目无证据), 证据保持
//!   待决, 绝不拿"没有回执"当负信号。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::stored_doc::{self, DocCompat};
use apeireth_memory::retrieval_tuning::{
    RetrievalTuningEngine, RetrievalTuningRecord, SelectedAttribution, TuningEvidence,
};

/// 自进化账文件名 (数据目录, 与 `tuning-log.jsonl` 同目录落位)。
pub const RETRIEVAL_TUNING_LOG_FILE: &str = "retrieval_tuning.jsonl";

/// 自进化账逐记录信封身份 (与读取端的版本契约一致)。
pub const RETRIEVAL_TUNING_DOC_NAME: &str = "retrieval-tuning-record";
/// 自进化账记录格式版本。
pub const RETRIEVAL_TUNING_DOC_VERSION: u32 = 1;

/// 自进化账的逐记录版本契约 (信封)。
pub fn retrieval_tuning_doc_compat() -> DocCompat {
    DocCompat::exact(RETRIEVAL_TUNING_DOC_NAME, RETRIEVAL_TUNING_DOC_VERSION)
}

/// 检索调权开关 env (默认关, 仅 `1` 开)。
pub const RETRIEVAL_TUNING_ENABLE_ENV: &str = "APEIRETH_ENABLE_RETRIEVAL_TUNING";

/// 开关解析 (纯函数): 仅 `1` 开, 其余一律关 (默认关 fail-closed)。
pub fn retrieval_tuning_enabled_from_env_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.trim() == "1")
}

/// 证据信号面状态 (0 装诚实)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalSurface {
    /// 注入回执面在场: 已观察到有选中回执的检索轮。
    Present {
        /// 有选中回执的检索轮数。
        selective_rounds: u64,
    },
    /// 无注入回执面 (召回路径未提供选中回执): **显式不进化**。
    Absent,
}

impl SignalSurface {
    /// 如实说明 (无面 = 显式不进化, 不编造"被引用/被追问"信号)。
    pub fn note(self) -> &'static str {
        match self {
            Self::Present { .. } => "注入后命中回执面在场 (证据随回执裁决)",
            Self::Absent => "无注入回执面: 显式不进化 (不编造使用信号)",
        }
    }
}

/// 一个待决检索轮 (上个有选中回执的注入集, 等下一个机会轮裁决)。
#[derive(Debug, Clone)]
struct PendingRound {
    receipts: Vec<SelectedAttribution>,
}

/// 检索维度自进化接线层: 回执 → 证据 → 引擎调权 → 账落盘 + 生效值落地。
#[derive(Debug)]
pub struct RetrievalTuningWire {
    engine: Mutex<RetrievalTuningEngine>,
    /// 自进化账 (`retrieval_tuning.jsonl`, append-only)。
    log_path: PathBuf,
    /// 账追加失败计数 (0 装诚实: 持久化失败不静默)。
    log_failures: AtomicU64,
    /// 待决注入集 (上个有选中回执的检索轮)。
    pending: Mutex<Option<PendingRound>>,
    /// 检索轮计数 (全部轮)。
    rounds: AtomicU64,
    /// 机会轮计数 (有选中回执的检索轮; > 0 = 信号面在场)。
    selective_rounds: AtomicU64,
}

impl RetrievalTuningWire {
    /// 构造接线层 (引擎以默认评分权重为基线; 生效值在首次调权时落地)。
    pub fn new(log_path: PathBuf) -> Self {
        Self {
            engine: Mutex::new(RetrievalTuningEngine::new()),
            log_path,
            log_failures: AtomicU64::new(0),
            pending: Mutex::new(None),
            rounds: AtomicU64::new(0),
            selective_rounds: AtomicU64::new(0),
        }
    }

    /// 从 env 构造: `APEIRETH_ENABLE_RETRIEVAL_TUNING=1` 才启用 (默认关 → None,
    /// fail-closed)。
    pub fn from_env(log_path: PathBuf) -> Option<Arc<Self>> {
        let raw = std::env::var(RETRIEVAL_TUNING_ENABLE_ENV).ok();
        if !retrieval_tuning_enabled_from_env_value(raw.as_deref()) {
            return None;
        }
        Some(Arc::new(Self::new(log_path)))
    }

    /// 自进化账路径 (自述记忆节与调参面板按同一落位读取)。
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// 注入回执 (机会轮): 一轮召回选中注入的 `(id, 显著维度)` 集合。
    ///
    /// 调用即裁决上个机会轮的注入集: 再次被命中 = 正信号, 机会在场而未再
    /// 命中 = 负信号 (注入后命中回执的最简可用口径); 本集转为待决。
    pub fn record_injection_round(&self, receipts: &[SelectedAttribution], at_epoch_ms: i64) {
        if receipts.is_empty() {
            self.record_empty_round(at_epoch_ms);
            return;
        }
        self.rounds.fetch_add(1, Ordering::Relaxed);
        self.selective_rounds.fetch_add(1, Ordering::Relaxed);
        let previous = self
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .replace(PendingRound {
                receipts: receipts.to_vec(),
            });
        let Some(previous) = previous else {
            return;
        };
        let evidence = resolve_evidence(&previous.receipts, receipts, at_epoch_ms);
        let mut engine = self
            .engine
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let records = engine.observe_batch(&evidence);
        if !records.is_empty() {
            apeireth_memory::retrieval_tuning::install_effective(engine.effective());
        }
        drop(engine);
        for record in &records {
            self.persist(record);
        }
    }

    /// 空召回轮 (无选中回执): 不是机会轮 —— 对已注入条目**无证据**, 待决证据
    /// 保持待决 (绝不拿"没有回执"当负信号)。
    pub fn record_empty_round(&self, _at_epoch_ms: i64) {
        self.rounds.fetch_add(1, Ordering::Relaxed);
    }

    /// 证据信号面现况 (无回执面 = 显式不进化)。
    pub fn signal_surface(&self) -> SignalSurface {
        let selective_rounds = self.selective_rounds.load(Ordering::Relaxed);
        if selective_rounds > 0 {
            SignalSurface::Present { selective_rounds }
        } else {
            SignalSurface::Absent
        }
    }

    /// 进化次数 (真实调权移动数)。
    pub fn evolution_count(&self) -> u64 {
        self.engine
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .evolution_count()
    }

    /// 最近一次调权真值 (何时/因何证据/何维度/前后权重)。
    pub fn last_move(&self) -> Option<RetrievalTuningRecord> {
        self.engine
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .last_move()
            .cloned()
    }

    /// 当前生效层权重 (调权后真值)。
    pub fn effective_ranking(&self) -> apeireth_memory::MemoryRankingConfig {
        self.engine
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .ranking()
    }

    /// 账追加失败计数 (0 装诚实: 持久化失败不静默)。
    pub fn log_failures(&self) -> u64 {
        self.log_failures.load(Ordering::Relaxed)
    }

    /// 待决注入集 (诊断/测试用)。
    pub fn pending_receipts(&self) -> Vec<SelectedAttribution> {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .map(|pending| pending.receipts.clone())
            .unwrap_or_default()
    }

    /// 账追加 (JSONL 追加, 每行一个逐记录信封; append-only)。
    fn persist(&self, record: &RetrievalTuningRecord) {
        let doc = stored_doc::StoredDoc::new(
            RETRIEVAL_TUNING_DOC_NAME,
            RETRIEVAL_TUNING_DOC_VERSION,
            vec![RETRIEVAL_TUNING_DOC_VERSION],
            record,
        );
        let Ok(line) = serde_json::to_string(&doc) else {
            self.log_failures.fetch_add(1, Ordering::Relaxed);
            return;
        };
        use std::io::Write;
        let appended = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .and_then(|mut file| writeln!(file, "{line}"));
        if appended.is_err() {
            self.log_failures.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// 证据裁决 (注入后命中回执): 上个机会轮的注入条目, 在本机会轮再命中 = 正
/// 信号, 未再命中 = 负信号 (机会在场)。
fn resolve_evidence(
    previous: &[SelectedAttribution],
    current: &[SelectedAttribution],
    at_epoch_ms: i64,
) -> Vec<TuningEvidence> {
    previous
        .iter()
        .map(|receipt| {
            if current.iter().any(|hit| hit.id == receipt.id) {
                TuningEvidence::positive(&receipt.id, receipt.dominant_dimension, at_epoch_ms)
            } else {
                TuningEvidence::negative(&receipt.id, receipt.dominant_dimension, at_epoch_ms)
            }
        })
        .collect()
}

/// 自进化账读回结果 (自述记忆节真值源: 进化次数 / 最近调权真值)。
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalTuningLedgerSummary {
    /// 账面记录 (文件序, 新 → 旧即追加序)。
    pub records: Vec<RetrievalTuningRecord>,
    /// 被跳过的坏行数 (坏行跳过并计数, 不静默)。
    pub skipped: usize,
}

impl RetrievalTuningLedgerSummary {
    /// 进化次数 (账面真实移动行数)。
    pub fn evolution_count(&self) -> u64 {
        self.records.len() as u64
    }

    /// 最近一次调权真值。
    pub fn last_move(&self) -> Option<&RetrievalTuningRecord> {
        self.records.last()
    }
}

/// 逐行读回自进化账 (append-only 账面真值; 缺档 = 空账如实, 非错误)。
pub fn read_retrieval_tuning_log(path: &Path) -> Result<RetrievalTuningLedgerSummary, String> {
    let scan = stored_doc::scan_records_compat::<RetrievalTuningRecord>(
        path,
        &retrieval_tuning_doc_compat(),
    )
    .map_err(|error| format!("retrieval tuning log read failed: {error}"))?;
    Ok(RetrievalTuningLedgerSummary {
        records: scan
            .records
            .into_iter()
            .map(|record| record.into_body())
            .collect(),
        skipped: scan.skipped,
    })
}

/// 自述记忆节真值映射 (纯函数): 自进化账摘要 → 记忆节「进化次数 / 最近调权」。
///
/// 进化次数 = 账面真实移动行数 (空账 = 0 如实); 最近调权 `None` = 尚未调权
/// (显式 null, 不编移动)。
pub fn memory_ledger_tuning_fields(
    summary: &RetrievalTuningLedgerSummary,
) -> (
    Option<u64>,
    Option<apeireth_tools_canonical::RetrievalTuningMove>,
) {
    let last = summary
        .last_move()
        .map(|record| apeireth_tools_canonical::RetrievalTuningMove {
            target: record.target.clone(),
            signal: record.signal.clone(),
            previous: record.previous,
            next: record.next,
            at_epoch_ms: record.at_epoch_ms,
            evidence: record.evidence.clone(),
        });
    (Some(summary.evolution_count()), last)
}

/// 自进化账落位: 与 `tuning-log.jsonl` 同目录 (session db 同目录口径)。
pub fn retrieval_tuning_log_path() -> PathBuf {
    retrieval_tuning_log_path_from_session_db(
        std::env::var("APEIRETH_SESSION_DB")
            .ok()
            .filter(|value| !value.trim().is_empty()),
    )
}

/// [`retrieval_tuning_log_path`] 的纯函数内核 (可测, 不读 env)。
pub fn retrieval_tuning_log_path_from_session_db(session_db: Option<String>) -> PathBuf {
    let session_db = session_db.unwrap_or_else(|| ".apeireth/sessions.sqlite3".to_string());
    let session_path = Path::new(&session_db);
    let dir = session_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    dir.join(RETRIEVAL_TUNING_LOG_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_memory::retrieval_tuning::RetrievalDimension;

    /// 生效值落地是进程级全局槽 + env 读取是进程级状态: 触碰两者的用例串行。
    static WIRE_TEST_GUARD: Mutex<()> = Mutex::new(());

    fn lock_wire() -> std::sync::MutexGuard<'static, ()> {
        WIRE_TEST_GUARD
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn temp_log(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-retrieval-tuning-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(RETRIEVAL_TUNING_LOG_FILE)
    }

    fn receipt(id: &str, dimension: Option<RetrievalDimension>) -> SelectedAttribution {
        SelectedAttribution::new(id, dimension)
    }

    #[test]
    fn the_switch_is_fail_closed_except_the_exact_opt_in() {
        assert!(!retrieval_tuning_enabled_from_env_value(None), "默认关");
        for raw in ["0", "true", "yes", "on", "", " 1x"] {
            assert!(
                !retrieval_tuning_enabled_from_env_value(Some(raw)),
                "{raw:?} 不许开 (fail-closed)"
            );
        }
        assert!(retrieval_tuning_enabled_from_env_value(Some("1")));
        assert!(retrieval_tuning_enabled_from_env_value(Some(" 1 ")));

        let _guard = lock_wire();
        let log = temp_log("switch");
        std::env::set_var(RETRIEVAL_TUNING_ENABLE_ENV, "0");
        assert!(RetrievalTuningWire::from_env(log.clone()).is_none());
        std::env::set_var(RETRIEVAL_TUNING_ENABLE_ENV, "1");
        assert!(RetrievalTuningWire::from_env(log.clone()).is_some());
        std::env::remove_var(RETRIEVAL_TUNING_ENABLE_ENV);
        assert!(
            RetrievalTuningWire::from_env(log.clone()).is_none(),
            "未设 = 关"
        );
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// 正负信号判定 (有信号面): 下个机会轮再命中 = 正, 机会在场未再命中 = 负。
    #[test]
    fn receipts_resolve_into_positive_and_negative_evidence() {
        let _guard = lock_wire();
        let log = temp_log("resolve");
        let wire = RetrievalTuningWire::new(log.clone());

        wire.record_injection_round(
            &[
                receipt("mem-a", Some(RetrievalDimension::Semantic)),
                receipt("mem-b", Some(RetrievalDimension::Lexical)),
            ],
            1_000,
        );
        assert_eq!(wire.evolution_count(), 0, "首轮回执待决, 无证据不调权");
        assert_eq!(
            wire.signal_surface(),
            SignalSurface::Present {
                selective_rounds: 1
            }
        );

        wire.record_injection_round(
            &[
                receipt("mem-a", Some(RetrievalDimension::Semantic)),
                receipt("mem-c", None),
            ],
            2_000,
        );
        // mem-a 再命中 → 正 (条目 + semantic); mem-b 未再命中 → 负 (条目 + lexical)。
        assert_eq!(wire.evolution_count(), 4, "{:?}", wire.last_move());
        assert_eq!(wire.last_move().unwrap().at_epoch_ms, 2_000);
        assert!((wire.effective_ranking().semantic_weight - 0.26).abs() < 1e-9);
        assert!((wire.effective_ranking().lexical_weight - 0.19).abs() < 1e-9);
        let pending = wire.pending_receipts();
        assert_eq!(
            pending.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["mem-a", "mem-c"],
            "本机会轮注入集转待决"
        );
        assert_eq!(wire.log_failures(), 0);
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// 无信号面 = 显式不进化: 不调权、不落账、状态照实说。
    #[test]
    fn without_a_signal_surface_the_wire_reports_absence_and_never_evolves() {
        let _guard = lock_wire();
        let log = temp_log("no-surface");
        let wire = RetrievalTuningWire::new(log.clone());

        for at in [1_000, 2_000, 3_000] {
            wire.record_empty_round(at);
        }
        assert_eq!(wire.signal_surface(), SignalSurface::Absent);
        assert_eq!(wire.evolution_count(), 0);
        assert!(wire.last_move().is_none());
        assert!(!log.exists(), "无证据 = 无账 (不造账)");
        assert!(SignalSurface::Absent.note().contains("显式不进化"));
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// 空召回轮不是机会轮: 待决证据保持待决, 直到下个有选中回执的轮。
    #[test]
    fn empty_rounds_leave_evidence_pending_until_the_next_selective_round() {
        let _guard = lock_wire();
        let log = temp_log("pending");
        let wire = RetrievalTuningWire::new(log.clone());

        wire.record_injection_round(&[receipt("mem-d", Some(RetrievalDimension::Graph))], 1_000);
        wire.record_empty_round(2_000);
        wire.record_empty_round(3_000);
        assert_eq!(wire.evolution_count(), 0, "空轮无证据, 不裁决");

        wire.record_injection_round(&[receipt("mem-e", None)], 4_000);
        assert_eq!(wire.evolution_count(), 2, "机会轮到来才裁决 (mem-d 负)");
        assert!(wire.effective_ranking().graph_weight < 0.05);
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// 自进化账 append-only: 只追加不改写, 前行字节不变, seq 单调前后值接龙。
    #[test]
    fn the_tuning_ledger_is_append_only_and_traceable() {
        let _guard = lock_wire();
        let log = temp_log("ledger");
        let wire = RetrievalTuningWire::new(log.clone());

        // 机会轮 2 裁决轮 1 (mem-f 负 → 2 行), 机会轮 3 裁决轮 2 (mem-g 负 → 2 行)。
        wire.record_injection_round(
            &[receipt("mem-f", Some(RetrievalDimension::Recency))],
            1_000,
        );
        wire.record_injection_round(
            &[receipt("mem-g", Some(RetrievalDimension::Recency))],
            2_000,
        );
        let after_two = std::fs::read_to_string(&log).expect("账已落");
        let lines_after_two: Vec<&str> = after_two.lines().collect();
        assert_eq!(lines_after_two.len(), 2, "{after_two}");

        wire.record_injection_round(
            &[receipt("mem-h", Some(RetrievalDimension::Recency))],
            3_000,
        );
        let after_three = std::fs::read_to_string(&log).expect("账已落");
        let lines_after_three: Vec<&str> = after_three.lines().collect();
        assert_eq!(lines_after_three.len(), 4);
        assert_eq!(
            &lines_after_three[..2],
            &lines_after_two[..],
            "append-only: 前行字节不改写"
        );

        let summary = read_retrieval_tuning_log(&log).expect("读回");
        assert_eq!(summary.evolution_count(), 4);
        assert_eq!(summary.skipped, 0);
        for (index, record) in summary.records.iter().enumerate() {
            assert_eq!(record.seq, index as u64 + 1, "seq 单调递增");
            assert!(
                (record.next - record.previous).abs() <= 0.01 + 1e-12,
                "步长有界"
            );
            if record.target.starts_with("entry:") {
                assert!(
                    record.next.abs() <= 0.05 + 1e-12,
                    "条目显著性走廊不越界: {}",
                    record.target
                );
            } else {
                assert!(
                    (record.next - 0.10).abs() <= 0.05 + 1e-12,
                    "维度走廊 (基线 ±0.05) 不越界: {}",
                    record.target
                );
            }
        }
        let recency: Vec<_> = summary
            .records
            .iter()
            .filter(|record| record.target == "dimension:recency")
            .collect();
        for pair in recency.windows(2) {
            assert!(
                (pair[0].next - pair[1].previous).abs() < 1e-12,
                "前后值接龙可回溯"
            );
        }
        assert!(summary
            .last_move()
            .unwrap()
            .evidence
            .contains("注入后命中回执"));
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// 账读回: 缺档 = 空账如实 (非错误); 坏行跳过并计数。
    #[test]
    fn ledger_reads_back_empty_when_absent_and_counts_bad_lines() {
        let _guard = lock_wire();
        let log = temp_log("scan");
        let summary = read_retrieval_tuning_log(&log).expect("缺档 = 空账");
        assert_eq!(summary.evolution_count(), 0);
        assert!(summary.last_move().is_none());

        std::fs::write(&log, "{ not json\n").unwrap();
        let summary = read_retrieval_tuning_log(&log).expect("坏行不致命");
        assert_eq!(summary.evolution_count(), 0);
        assert_eq!(summary.skipped, 1, "坏行跳过并计数 (不静默)");
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// 自述记忆节真值映射: 进化次数 / 最近调权取自账面真值 (与账同源)。
    #[test]
    fn memory_ledger_tuning_fields_map_the_ledger_truth() {
        let _guard = lock_wire();
        let log = temp_log("mapping");
        let wire = RetrievalTuningWire::new(log.clone());
        // 空账: 进化 0 + 最近调权显式 None (不编移动)。
        let summary = read_retrieval_tuning_log(&log).expect("读回");
        let (count, last) = memory_ledger_tuning_fields(&summary);
        assert_eq!(count, Some(0));
        assert!(last.is_none());

        wire.record_injection_round(
            &[receipt("mem-x", Some(RetrievalDimension::Novelty))],
            1_000,
        );
        wire.record_injection_round(&[receipt("mem-y", None)], 2_000);
        let summary = read_retrieval_tuning_log(&log).expect("读回");
        let (count, last) = memory_ledger_tuning_fields(&summary);
        assert_eq!(
            count,
            Some(2),
            "mem-x 注入后未再命中 → 负, 条目 + 维度各一行"
        );
        let last = last.expect("最近调权真值");
        // 同批记录按目标 id 序落账, 最后一行 = 条目显著性那行。
        assert_eq!(last.target, "entry:mem-x");
        assert_eq!(last.signal, "negative");
        assert_eq!(last.at_epoch_ms, 2_000);
        assert!(last.evidence.contains("注入后命中回执"));
        assert_eq!(
            wire.evolution_count(),
            count.unwrap(),
            "自述计数与接线层同账"
        );
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    #[test]
    fn the_ledger_path_sits_beside_the_session_db() {
        #[cfg(windows)]
        assert_eq!(
            retrieval_tuning_log_path_from_session_db(Some(r"C:\data\sessions.sqlite3".into())),
            PathBuf::from(r"C:\data").join(RETRIEVAL_TUNING_LOG_FILE)
        );
        assert_eq!(
            retrieval_tuning_log_path_from_session_db(None),
            PathBuf::from(".apeireth").join(RETRIEVAL_TUNING_LOG_FILE)
        );
    }
}
