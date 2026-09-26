//! 自校准接线层 (SelfTuningWire) —— 「性格养成」第一铲的落地接线。
//!
//! **职责** (引擎本身保持纯确定性、无 IO):
//! - 持有 [`SelfTuningEngine`], 把真实使用信号 ([`TuningSignal`]) 喂给引擎;
//! - 引擎每产生一条 [`TuningRecord`] → **追加写 `tuning-log.jsonl`** (数据目录,
//!   落位与 session db 同目录, 见 [`tuning_log_path`]); 这是「记录透明」的载体;
//! - 把引擎的 [`TuningValues`] 装进进程内 override, 让四个体验常量的使用点
//!   (经 [`TunableParam::effective`]) 实时吃到自动调整 —— 调整真生效, 不装学习。
//!
//! **开关**: `APEIRETH_ENABLE_SELF_TUNING=1` 才构造 ([`SelfTuningWire::from_env`]);
//! 默认关 (重设计·轻默认)。关 = 无信号、无调整、无日志, 四个旋钮只走 env。
//!
//! **0 装诚实**: 持久化失败 fail-open (调整仍生效) 但计入 [`SelfTuningWire::log_failures`],
//! 不静默吞掉"日志丢了"这件事。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_orchestration::self_tuning::{
    install_effective_values, RetrievalHitMissSignal, SelfTuningEngine, TunableParam, TuningEvent,
    TuningRecord, TuningSignal, TuningValues,
};

/// 学习日志文件名 (数据目录, 与 session db 同目录落位)。
pub const TUNING_LOG_FILE: &str = "tuning-log.jsonl";

/// 自学习开关 env (默认关, 仅 `1` 开)。
pub const SELF_TUNING_ENABLE_ENV: &str = "APEIRETH_ENABLE_SELF_TUNING";

/// 自校准接线层: 引擎 + 真接信号源 + 日志落盘 + 生效值落地。
#[derive(Debug)]
pub struct SelfTuningWire {
    engine: Mutex<SelfTuningEngine>,
    /// 真接信号源: 检索命中/未命中 (MemoryRecallModule 召回路径写入)。
    retrieval: RetrievalHitMissSignal,
    log_path: PathBuf,
    log_failures: AtomicU64,
}

impl SelfTuningWire {
    /// 构造接线层: 引擎以给定初值起步 (调参面板/env 旋钮值 = 自动校准的起点),
    /// 并立即把初值装进 override (引擎是启用期间的唯一权威)。
    pub fn new(log_path: PathBuf, initial: TuningValues) -> Self {
        let engine = SelfTuningEngine::with_initial(initial);
        install_effective_values(engine.effective_values());
        Self {
            engine: Mutex::new(engine),
            retrieval: RetrievalHitMissSignal::new(),
            log_path,
            log_failures: AtomicU64::new(0),
        }
    }

    /// 从 env 构造: `APEIRETH_ENABLE_SELF_TUNING=1` 才启用 (默认关 → None);
    /// 初值 = 四个 `APEIRETH_TUNE_*` 旋钮 (非法/未设回各基线)。
    pub fn from_env(log_path: PathBuf) -> Option<Arc<SelfTuningWire>> {
        let enabled = std::env::var(SELF_TUNING_ENABLE_ENV)
            .ok()
            .is_some_and(|value| value.trim() == "1");
        if !enabled {
            return None;
        }
        Some(Arc::new(Self::new(log_path, TuningValues::from_env())))
    }

    /// 学习日志路径 (调参面板经 src-tauri 命令按同一落位读取)。
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// 真接信号: 记录一轮检索命中/未命中 (MemoryRecallModule 召回路径调用);
    /// 凑满一批即观察一次, 有调整则落日志 + 落地生效值。
    pub fn record_retrieval_round(&self, hit: bool, at_epoch_ms: i64) {
        self.retrieval.record_round(hit, at_epoch_ms);
        while self.observe_signal(&self.retrieval).is_some() {}
    }

    /// 从信号源拉一条并观察 (注入点轮询式用法)。
    pub fn observe_signal(&self, signal: &dyn TuningSignal) -> Option<TuningRecord> {
        let mut engine = self
            .engine
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let record = engine.observe_signal(signal)?;
        drop(engine);
        self.persist_and_apply(&record);
        Some(record)
    }

    /// 直接观察一条事件 (未接信号的接口预留入口; 生产只走真接信号)。
    pub fn observe(&self, event: TuningEvent) -> Option<TuningRecord> {
        let mut engine = self
            .engine
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let record = engine.observe(event)?;
        drop(engine);
        self.persist_and_apply(&record);
        Some(record)
    }

    /// 当前生效的四个体验参数值 (引擎权威)。
    pub fn effective_values(&self) -> TuningValues {
        self.engine
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .effective_values()
    }

    /// 全部调整记录 (进程内镜像; 持久镜像是 tuning-log.jsonl)。
    pub fn history(&self) -> Vec<TuningRecord> {
        self.engine
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .history()
            .to_vec()
    }

    /// 日志追加失败计数 (0 装诚实: 持久化失败不静默)。
    pub fn log_failures(&self) -> u64 {
        self.log_failures.load(Ordering::Relaxed)
    }

    /// 落日志 (JSONL 追加) + 落地生效值 (进程内 override)。
    fn persist_and_apply(&self, record: &TuningRecord) {
        match serde_json::to_string(record) {
            Ok(line) => {
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
            Err(_) => {
                self.log_failures.fetch_add(1, Ordering::Relaxed);
            }
        }
        let engine = self
            .engine
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        install_effective_values(engine.effective_values());
    }
}

/// 学习日志落位: 与 session db 同目录 (路径风格对齐现有 session db 落位 ——
/// `APEIRETH_SESSION_DB` 有值取其目录, 否则默认 `.apeireth/` 下)。
///
/// 桌面侧车由 BackendSupervisor 把 `APEIRETH_SESSION_DB` 锚到 store 目录,
/// 因此日志 = `<store>/tuning-log.jsonl`, 与 src-tauri 读取端同落位。
pub fn tuning_log_path() -> PathBuf {
    tuning_log_path_from_session_db(
        std::env::var("APEIRETH_SESSION_DB")
            .ok()
            .filter(|value| !value.trim().is_empty()),
    )
}

/// [`tuning_log_path`] 的纯函数内核 (可测, 不读 env)。
pub fn tuning_log_path_from_session_db(session_db: Option<String>) -> PathBuf {
    let session_db = session_db.unwrap_or_else(|| ".apeireth/sessions.sqlite3".to_string());
    let session_path = Path::new(&session_db);
    let dir = session_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    dir.join(TUNING_LOG_FILE)
}

/// 整合节奏 (体验旋钮 [`TunableParam::ConsolidationCadence`], env
/// `APEIRETH_TUNE_CONSOLIDATION_CADENCE`): 每 N 回合触发一次记忆整合;
/// 1 = 每回合 (现状, 零变化)。非法/未设回 1。
pub fn consolidation_cadence_turns() -> u64 {
    let cadence = TunableParam::ConsolidationCadence.effective();
    let rounded = cadence.round();
    if rounded.is_finite() {
        rounded.clamp(1.0, 10.0) as u64
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuning_log_lands_beside_session_db() {
        assert_eq!(
            tuning_log_path_from_session_db(Some(r"C:\data\sessions.sqlite3".into())),
            PathBuf::from(r"C:\data").join(TUNING_LOG_FILE)
        );
        assert_eq!(
            tuning_log_path_from_session_db(None),
            PathBuf::from(".apeireth").join(TUNING_LOG_FILE)
        );
    }

    #[test]
    fn wire_appends_jsonl_and_installs_effective_values() {
        let dir = std::env::temp_dir().join(format!("apeireth-tuning-wire-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let log_path = dir.join(TUNING_LOG_FILE);
        let _ = std::fs::remove_file(&log_path);

        let wire = SelfTuningWire::new(log_path.clone(), TuningValues::baseline());
        // 全命中批 → MemoryFade 上调一档 (真信号路径: 批 4 轮一发)。
        for _ in 0..4 {
            wire.record_retrieval_round(true, 1_000);
        }
        let values = wire.effective_values();
        assert!(
            values.memory_fade > TunableParam::MemoryFade.baseline(),
            "真信号应触发调整: {}",
            values.memory_fade
        );
        assert_eq!(wire.log_failures(), 0, "日志追加应成功");
        let line = std::fs::read_to_string(&log_path).expect("tuning-log.jsonl 已写");
        let record: TuningRecord = serde_json::from_str(line.trim()).expect("JSONL 行可反序列化");
        assert_eq!(record.param, TunableParam::MemoryFade);
        assert_eq!(record.seq, 1);
        // 生效值 override 落地: 各常量使用点取到引擎值。
        assert_eq!(
            TunableParam::MemoryFade.effective(),
            values.memory_fade,
            "调整必须即时生效 (override 落地)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cadence_knob_defaults_to_every_turn() {
        apeireth_orchestration::self_tuning::clear_effective_overrides();
        let cadence = TunableParam::ConsolidationCadence.parse_env_value(None);
        assert_eq!(cadence, 1.0, "未设旋钮 = 每回合 = 现行为");
        assert!(consolidation_cadence_turns() >= 1);
    }
}
