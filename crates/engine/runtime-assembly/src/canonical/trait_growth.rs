//! 性格成长数据面 (裁定 ④⑤ 落地): 特质滑杆**锁定态持久档** + **成长计既有账**读面。
//!
//! # 两个落位 (同一数据目录, 与 session db 同目录)
//!
//! | 文件 | 是什么 | 写面 |
//! |---|---|---|
//! | `tuning-log.jsonl` | **成长计既有账** (自校准沉积记录 [`TuningRecord`], 逐记录信封) | 只有 [`super::self_tuning_wire::SelfTuningWire`] 追加; 本模块只读 |
//! | `trait-locks.json` | 四杆**锁定态** ( [`TraitLocks`] ) | 显式动作 [`set_trait_lock`] (锁/解锁) |
//!
//! **不新造账**: 周报 (`tool.trait_report`) 与自述性格节的「最近移动」都读
//! `tuning-log.jsonl` 的既有沉积记录; 锁定跳过的沉积**审计留痕**也落在同一账
//! (零移动留痕行, 见 `apeireth_orchestration::self_tuning::TRAIT_LOCKED_SKIP_PREFIX`)。
//!
//! # 读取语义 (0 装诚实)
//!
//! - 成长账缺文件 = **空账** (还没有沉积, 不是错误); 逐记录扫描坏行跳过并计数
//!   ([`TraitGrowthFacts::skipped`]), 不静默吞;
//! - 锁定档缺文件 = **全解锁** (默认态); 锁定档**坏档 = `Err` 即帧** (绝不把
//!   坏档读成"全解锁"去放行移动);
//! - 每次探测**现读** (锁定动作对在跑进程即时生效)。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_core::clock::system_clock;
use apeireth_core::stored_doc;
use apeireth_orchestration::self_tuning::{TraitLocks, TunableParam, TuningRecord, TuningValues};
use apeireth_orchestration::trait_report::{recent_moves, TRAIT_RECENT_MOVES};
use apeireth_tools_canonical::{StatusProbe, TraitLockStatus, TraitMoveStatus, TraitStatus};

use super::self_tuning_wire::{tuning_log_doc_compat, TUNING_LOG_FILE};

/// 特质锁定态持久档文件名 (与成长计既有账同目录)。
pub const TRAIT_LOCKS_FILE: &str = "trait-locks.json";

/// 性格成长数据面的落位: 成长计既有账路径 (锁定档与账同目录派生)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitGrowthPaths {
    /// 成长计既有账 (`tuning-log.jsonl`)。
    pub log_path: PathBuf,
}

impl TraitGrowthPaths {
    /// 以成长计既有账路径为锚 (锁定档 = 同目录 `trait-locks.json`)。
    pub fn new(log_path: impl Into<PathBuf>) -> Self {
        Self {
            log_path: log_path.into(),
        }
    }

    /// 特质锁定态持久档路径 (与账同目录)。
    pub fn locks_path(&self) -> PathBuf {
        trait_locks_path_for(&self.log_path)
    }
}

/// 特质锁定态持久档路径: 与给定成长账同目录的 `trait-locks.json`。
pub fn trait_locks_path_for(log_path: &Path) -> PathBuf {
    log_path.with_file_name(TRAIT_LOCKS_FILE)
}

/// 特质锁定态持久档默认落位 (与 [`super::self_tuning_wire::tuning_log_path`] 同目录)。
pub fn trait_locks_path() -> PathBuf {
    trait_locks_path_for(&super::self_tuning_wire::tuning_log_path())
}

/// 读锁定态: 缺文件 = 全解锁 (默认态); 坏档 = `Err` 即帧 (不放行、不猜)。
pub fn load_trait_locks(locks_path: &Path) -> Result<TraitLocks, String> {
    let text = match std::fs::read_to_string(locks_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(TraitLocks::default());
        }
        Err(error) => return Err(format!("trait lock state refused to read: {error}")),
    };
    serde_json::from_str::<TraitLocks>(&text)
        .map_err(|error| format!("trait lock state refused to parse: {error}"))
}

/// 写锁定态 (整档替换; 写失败即帧)。
pub fn save_trait_locks(locks_path: &Path, locks: &TraitLocks) -> Result<(), String> {
    let line = serde_json::to_string_pretty(locks)
        .map_err(|error| format!("trait lock state refused to serialize: {error}"))?;
    if let Some(parent) = locks_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("trait lock state dir refused to create: {error}"))?;
    }
    std::fs::write(locks_path, line)
        .map_err(|error| format!("trait lock state refused to write: {error}"))
}

/// 显式锁/解锁动作 (裁定 ④: 解锁 = 显式动作): 读-改-写持久档并返回新锁定态。
///
/// 锁定档坏 = `Err` 即帧 (不覆盖坏档、不代行裁决)。
pub fn set_trait_lock(
    locks_path: &Path,
    param: TunableParam,
    locked: bool,
) -> Result<TraitLocks, String> {
    let mut locks = load_trait_locks(locks_path)?;
    locks.set(param, locked);
    save_trait_locks(locks_path, &locks)?;
    Ok(locks)
}

/// 读成长计既有账 (逐记录信封 + 旧裸体行只读兼容; 缺文件 = 空账)。
pub fn read_growth_records(
    log_path: &Path,
) -> Result<(Vec<TuningRecord>, usize), stored_doc::StoredDocError> {
    let scan = stored_doc::scan_records_compat::<TuningRecord>(log_path, &tuning_log_doc_compat())?;
    Ok((
        scan.records.iter().map(|row| row.body().clone()).collect(),
        scan.skipped,
    ))
}

/// 一次现读的性格成长账面读数 (工具面与自述性格节同一真值源)。
#[derive(Debug, Clone)]
pub struct TraitGrowthFacts {
    /// 成长计既有账的沉积记录 (含零移动留痕行)。
    pub records: Vec<TuningRecord>,
    /// 扫描跳过的坏行数 (0 装诚实: 不静默吞坏行)。
    pub skipped: usize,
    /// 四杆锁定态。
    pub locks: TraitLocks,
    /// 四杆当前生效值 (进程内自动调整 > env 旋钮 > 基线)。
    pub values: TuningValues,
}

impl TraitGrowthFacts {
    /// 现读一次账面 (当前生效值 + 当前锁定态 + 全账记录)。
    pub fn read(paths: &TraitGrowthPaths) -> Result<Self, String> {
        let (records, skipped) = read_growth_records(&paths.log_path)
            .map_err(|error| format!("growth ledger: {error}"))?;
        let locks = load_trait_locks(&paths.locks_path())?;
        Ok(Self {
            records,
            skipped,
            locks,
            values: TuningValues {
                memory_fade: TunableParam::MemoryFade.effective(),
                curiosity_strength: TunableParam::CuriosityStrength.effective(),
                tone_saturation: TunableParam::ToneSaturation.effective(),
                consolidation_cadence: TunableParam::ConsolidationCadence.effective(),
            },
        })
    }

    /// 自述性格节投影 (锁定态 + 最近移动)。
    pub fn status_at(&self, now_ms: i64) -> TraitStatus {
        TraitStatus {
            locked: TraitLockStatus {
                memory_fade: self.locks.memory_fade,
                curiosity_strength: self.locks.curiosity_strength,
                tone_saturation: self.locks.tone_saturation,
                consolidation_cadence: self.locks.consolidation_cadence,
            },
            recent_moves: recent_moves(&self.records, now_ms, TRAIT_RECENT_MOVES)
                .into_iter()
                .map(|movement| TraitMoveStatus {
                    param: movement.param.id().to_string(),
                    label: movement.label,
                    at_epoch_ms: movement.at_epoch_ms,
                    delta: movement.delta,
                    reason: movement.reason,
                })
                .collect(),
        }
    }
}

/// 成长账面探测口 (工具面 `tool.trait_report` 的只读来源)。
pub type TraitGrowthProbe = Arc<dyn Fn() -> Result<TraitGrowthFacts, String> + Send + Sync>;

/// 组装一个现读的成长账面探测口。
pub fn trait_growth_probe(paths: TraitGrowthPaths) -> TraitGrowthProbe {
    Arc::new(move || TraitGrowthFacts::read(&paths))
}

/// 自述性格节探测口 (`self_status` 调参/性格节的锁定态 + 最近移动)。
pub fn trait_status_probe(paths: TraitGrowthPaths) -> StatusProbe<TraitStatus> {
    Arc::new(move || {
        let facts = TraitGrowthFacts::read(&paths)?;
        Ok(facts.status_at(now_epoch_ms()))
    })
}

/// 当前时刻 (epoch 毫秒, 系统时钟)。
pub fn now_epoch_ms() -> i64 {
    system_clock().now().timestamp_millis()
}

/// 学习日志默认文件名 (与 [`super::self_tuning_wire::TUNING_LOG_FILE`] 同名同物)。
pub const GROWTH_LEDGER_FILE: &str = TUNING_LOG_FILE;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::self_tuning_wire::{TUNING_LOG_DOC_NAME, TUNING_LOG_DOC_VERSION};
    use apeireth_orchestration::self_tuning::TRAIT_LOCKED_SKIP_PREFIX;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-trait-growth-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn record(seq: u64, param: TunableParam, at_epoch_ms: i64) -> TuningRecord {
        TuningRecord {
            seq,
            param,
            previous: 1.0,
            next: 1.25,
            reason: "测试沉积".to_string(),
            at_epoch_ms,
        }
    }

    /// 一行成长账 (逐记录信封, 与写端同格式)。
    fn envelope(row: &TuningRecord) -> String {
        serde_json::to_string(&stored_doc::StoredDoc::new(
            TUNING_LOG_DOC_NAME,
            TUNING_LOG_DOC_VERSION,
            vec![TUNING_LOG_DOC_VERSION],
            row,
        ))
        .expect("serialize")
    }

    // 锁定态持久 (裁定 ④): 锁/解锁是显式动作, 落盘后重读保持 (跨进程即生效)。
    #[test]
    fn trait_lock_state_persists_and_survives_reload() {
        let dir = temp_dir("locks");
        let paths = TraitGrowthPaths::new(dir.join(TUNING_LOG_FILE));
        assert_eq!(paths.locks_path(), dir.join(TRAIT_LOCKS_FILE));

        // 缺档 = 全解锁 (默认态)。
        let locks = load_trait_locks(&paths.locks_path()).expect("缺档回默认");
        assert!(!locks.any_locked());

        // 显式锁一根 → 持久 → 重读保持。
        let locks = set_trait_lock(&paths.locks_path(), TunableParam::ToneSaturation, true)
            .expect("锁定落盘");
        assert!(locks.tone_saturation);
        let reloaded = load_trait_locks(&paths.locks_path()).expect("重读");
        assert!(reloaded.tone_saturation, "锁定态必须持久");
        assert!(!reloaded.memory_fade, "只锁一根不误伤别杆");

        // 显式解锁 → 持久 → 重读恢复。
        let locks = set_trait_lock(&paths.locks_path(), TunableParam::ToneSaturation, false)
            .expect("解锁落盘");
        assert!(!locks.any_locked(), "解锁恢复");
        let reloaded = load_trait_locks(&paths.locks_path()).expect("重读");
        assert!(!reloaded.any_locked());

        // 坏档 = 即帧 (绝不读成全解锁放行)。
        std::fs::write(paths.locks_path(), "{ not json").expect("坏档");
        assert!(
            load_trait_locks(&paths.locks_path()).is_err(),
            "坏档必须 Err 即帧"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // 成长账读面: 缺文件 = 空账; 跳过留痕行照读 (锁定审计痕在账上可查)。
    #[test]
    fn growth_ledger_reads_missing_as_empty_and_keeps_lock_skips() {
        let dir = temp_dir("ledger");
        let paths = TraitGrowthPaths::new(dir.join(TUNING_LOG_FILE));
        let (records, skipped) = read_growth_records(&paths.log_path).expect("缺账 = 空账");
        assert!(records.is_empty());
        assert_eq!(skipped, 0);

        let moved = record(1, TunableParam::MemoryFade, 1_000);
        let skipped_row = TuningRecord {
            seq: 2,
            param: TunableParam::MemoryFade,
            previous: 1.0,
            next: 1.0,
            reason: format!("{TRAIT_LOCKED_SKIP_PREFIX} (审计留痕): 测试意图"),
            at_epoch_ms: 2_000,
        };
        let append = |row: &TuningRecord| {
            use std::io::Write;
            let line = envelope(row);
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&paths.log_path)
                .expect("open");
            writeln!(file, "{line}").expect("append");
        };
        append(&moved);
        append(&skipped_row);

        let (records, skipped) = read_growth_records(&paths.log_path).expect("读账");
        assert_eq!(records.len(), 2, "沉积记录与锁定跳过留痕同账可查");
        assert_eq!(skipped, 0);
        assert!(apeireth_orchestration::self_tuning::is_trait_locked_skip(
            &records[1]
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // 自述性格节投影: 锁定态 + 最近移动 (新 → 旧) 同账真值。
    #[test]
    fn trait_status_projects_lock_state_and_recent_moves() {
        let dir = temp_dir("status");
        let paths = TraitGrowthPaths::new(dir.join(TUNING_LOG_FILE));
        set_trait_lock(&paths.locks_path(), TunableParam::MemoryFade, true).expect("锁定");
        let rows = [
            record(1, TunableParam::MemoryFade, 1_000),
            record(2, TunableParam::ToneSaturation, 2_000),
        ];
        let docs: Vec<String> = rows.iter().map(envelope).collect();
        std::fs::write(&paths.log_path, docs.join("\n")).expect("写账");

        let facts = TraitGrowthFacts::read(&paths).expect("现读");
        let status = facts.status_at(10_000);
        assert!(status.locked.memory_fade, "锁定态如实");
        assert!(!status.locked.tone_saturation);
        assert_eq!(status.recent_moves.len(), 2, "两条实际移动都算");
        assert_eq!(status.recent_moves[0].param, "tone_saturation", "新 → 旧");
        assert_eq!(status.recent_moves[0].label, "语气情绪饱和度");
        assert_eq!(status.recent_moves[1].param, "memory_fade");
        assert_eq!(status.recent_moves[0].delta, 0.25);
        // 未来时刻不计 (now < 记录时刻时不出"最近移动")。
        let future = facts.status_at(500);
        assert!(future.recent_moves.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
