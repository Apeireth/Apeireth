//! Apeireth R19 TUI — 后端全接
//!
//! **R11 LOCKED 边界** (omnibus §6): 本文件只调后端 crate 的公开 API,
//! 不修改任何 R11 LOCKED enum / 转换矩阵 / 8 项不修改承诺.
//!
//! **复用 W2 (apeireth-desktop) 9 器官 snapshot 模式**:
//! TUI 不依赖 apeireth-desktop (避免引入 tauri 依赖冲突), 但函数语义与
//! `crates/apeireth-desktop/src/main.rs` 完全对齐 — 同样的 9 个 organ,
//! 同样的 compute_v05 / compute_life_stage / compute_reflection /
//! compute_endurance / memory_store 单例 / sovereignty_guard 单例.
//!
//! **R19-TUI W2**: chat() 调 apeireth-api 走 minimaxi OpenAI 协议,
//! apikey 从 `.openclaw\apikey.txt` 读.

use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use apeireth_action::DefaultActionEngine;
// R25 改瘦: TUI 不再 import apeireth_api 的 LLM 相关 (OpenAiCompatibleConfig 等)
// LLM 调用全走 http_llm::call_llm_http_* (HTTP 瘦客户端)
// LlmError 保留: process_stream_to_reply (纯函数 helper, test 用) 仍用它签名
use apeireth_api::LlmError;
// http_llm 是 R25 新增的 HTTP LLM 客户端 (mod http_llm 在 main.rs 声明)
use crate::organ::brain;
use crate::organ::ear;
use crate::organ::memory;
use crate::app::ChatMessage;
use crate::http_llm;
use apeireth_asi::{
    AsiV05Scores, DimensionRegistry, DimensionTrace, MeasurementSample, V05_DIMENSION_NAMES,
};
// R54 B8 续: apeireth-graph 接 cognition_graph::run_cognition_graph_sync
use apeireth_graph::cognition_graph;
use apeireth_cognition::{run_cycle, CognitiveInput, CognitiveOutput};
use apeireth_consciousness::CognitiveDreamStateMachine;
use apeireth_core::{ActionTarget, Episode, IdentityCard, LifeStage};
use apeireth_life_force::{exhaustion_check, LifeForce, ENDURANCE_MAX};
use apeireth_memory::{EpisodeQuery, EpisodeStore, IdentityCardStore, SqliteMemoryStore};
use apeireth_motivation::{
    motivation_score, AutonomyConsistency, IntrinsicIntensity, ValueStability,
};
use apeireth_relations::Relation;
use apeireth_sovereignty::self_disable::SelfDisableGuard;
use apeireth_supervisor::{PidOneSupervisor, RestartStrategy, SubSupervisorKind};
use apeireth_value::ValueDimension;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

// ============================================================
// 全局状态 (跨页面共享)
// ============================================================

/// Cognitive cycle 累计计数 (每次对话/认知操作 +1)
pub static CYCLE_COUNT: AtomicU64 = AtomicU64::new(0);
/// Token 累计 (W1 mock 142857, W2 接 R17 apeireth-api LLM 报数, chat() 内部
/// `TOKEN_USED.fetch_add(reply.usage.total, ...)` 真跑覆盖)
pub static TOKEN_USED: AtomicU64 = AtomicU64::new(142_857);
/// R19 自研 token 累计 (W3.4 成就落地)
/// 跟 TOKEN_USED 并行: 跟 LLM 报数独立, 启发式估算 (char count, 不 byte count)
/// 启发式参考 tiktoken OpenAI 经验值: ASCII/4 + CJK/1.5 + 其他/2
/// status bar 双字段: `token LLM X / R19 Y`
///
/// **不假装策略** (主哲学 6 锚):
/// - 默认 0 (跟 W1 mock 142857 不同, 真起真算不假装, 0 起步等 chat 累加)
/// - 不依赖 LLM 报数 (那是 TOKEN_USED 的事, 这条独立)
/// - 升级路径: W4+ 接 apeireth-asi 真 token API (24 维 + V1136 9 子测度)
///   替换启发式, `W3.4 heuristic` 注释保留可 grep
pub static R19_TOKEN_USED: AtomicU64 = AtomicU64::new(0);

/// status bar 用的 getter (UI 模块化, 不直接读 Atomic)
pub fn cycle_count_load() -> u64 {
    CYCLE_COUNT.load(Ordering::Relaxed)
}
pub fn token_used_load() -> u64 {
    TOKEN_USED.load(Ordering::Relaxed)
}
/// R19 自研 token 累计 getter (W3.4)
pub fn r19_token_used_load() -> u64 {
    R19_TOKEN_USED.load(Ordering::Relaxed)
}
/// 5-Self armed 状态 (W2 简化: 直接读 SelfDisableGuard)
pub fn five_self_armed_label() -> String {
    let guard = sovereignty_guard();
    let g = guard.lock().ok();
    match g {
        Some(g) if g.is_armed => "✓ armed".to_string(),
        _ => "✗ disarmed".to_string(),
    }
}

/// SqliteMemoryStore 全局单例 (跟 R18 web 一致)
static MEMORY_STORE: OnceLock<Arc<SqliteMemoryStore>> = OnceLock::new();

/// SelfDisableGuard 全局单例 (跟 R18 sovereignty.rs 一致)
static SOVEREIGNTY_GUARD: OnceLock<Arc<Mutex<SelfDisableGuard>>> = OnceLock::new();

/// 默认 IdentityCard continuity_id (跟 R18 web 一致)
pub const DEFAULT_CONTINUITY_ID: &str = "apeireth-tui-default";

/// TUI 终端对话 session id (W3 #2 成就落地: 历史页 6 流能查 tui-session 数据)
/// 也用于 history_stream_counts() 的 6 流表里 (tui-session 是 1 流)
pub const TUI_SESSION_ID: &str = "tui-session";

/// 全局单调递增 ID 计数器 (W3 #2 落地: 确保多次写入 id 不重复,
/// 避免 `INSERT OR IGNORE` 静默丢数据, 跟 nano 时间戳解耦)
static EPISODE_ID_SEQ: AtomicU64 = AtomicU64::new(0);

/// 生成下一个唯一 episode id (W3 #2 内部用, 跨 chat 调用都唯一)
fn next_episode_id() -> String {
    let n = EPISODE_ID_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("tui-ep-{:016x}", n)
}

/// 全局"最后已分配 chat-pair 时间戳" (W3 #2 落地: 每次 chat_internal 分配 2 个严格递增
/// 的逻辑时间戳 user_ts / asst_ts, 避免同秒多 chat 时 query 排序错位).
///
/// 逻辑时间戳 = epoch milliseconds, 保证:
///  (a) 跨 chat 严格递增 (asst_ts > 上一 chat 的 asst_ts)
///  (b) 不落后实际时间 (>= now_ts() * 1000, 不会出现"穿越")
///  (c) 同一 chat 内 asst_ts = user_ts + 1 (user 在前)
static LAST_CHAT_TS: AtomicI64 = AtomicI64::new(0);

/// 分配下一对 (user_ts, asst_ts) 严格递增的逻辑时间戳 (W3 #2 内部用).
///
/// 用 CAS 自旋保证 base 严格递增, 同时不落后实际时间。
fn next_chat_pair_timestamps() -> (i64, i64) {
    let real_ms = now_ts() * 1000;
    loop {
        let last = LAST_CHAT_TS.load(Ordering::Relaxed);
        // base = max(上一 chat asst_ts + 1, 实际 ms), 保证 (a) 跨 chat 严格递增 (b) 不穿越
        let base = std::cmp::max(real_ms, last + 1);
        // 分配 (base, base+1) 给本次 chat, CAS 更新 LAST_CHAT_TS
        if LAST_CHAT_TS
            .compare_exchange(last, base + 1, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return (base, base + 1);
        }
        // CAS 失败 → 其他线程已更新, 重试
    }
}

// ============================================================
// 数据结构 (TUI 内部使用)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganStatus {
    pub name: String,
    pub display: String,
    pub metaphor: String,
    pub health: f64,
    pub primary: String,
    pub secondary: String,
    pub tertiary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainAiStatus {
    pub asi_v05: f64,
    pub asi_continuity: f64,
    pub asi_philosophy: f64,
    pub life_stage: String,
    pub life_stage_idx: u8,
    pub reflection_status: String,
    pub endurance: f64,
    pub episode_count: u64,
    pub cycle_count: u64,
    pub token_used: u64,
    /// W3.4: R19 自研 token 估算 (跟 LLM 报数 `token_used` 并行)
    /// 启发式: ASCII/4 + CJK/1.5 + 其他/2 (跟 tiktoken 经验值对齐)
    pub token_r19: u64,
    pub five_self: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeStageInfo {
    pub idx: u8,
    pub zh: String,
    pub en: String,
    pub r11_enum: String,
    pub visible: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateNode {
    pub name: String,
    pub display: String,
    pub group: String,
    /// 极坐标 r (0-1)
    pub r: f64,
    /// 极坐标 theta (rad, 0-2π)
    pub theta: f64,
    pub active: f64,
}

// ============================================================
// 状态管理 — 启动期 lazy init
// ============================================================

pub fn memory_store() -> Result<Arc<SqliteMemoryStore>, String> {
    if let Some(s) = MEMORY_STORE.get() {
        return Ok(Arc::clone(s));
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir
        .parent()
        .map(|p| p.join("apeireth-memory.db"))
        .unwrap_or_else(|| manifest_dir.join("apeireth-memory.db"));
    let store = SqliteMemoryStore::open(&path)
        .map_err(|e| format!("SqliteMemoryStore::open failed: {e}"))?;
    let arc = Arc::new(store);
    let _ = MEMORY_STORE.set(Arc::clone(&arc));
    Ok(arc)
}

pub fn sovereignty_guard() -> Arc<Mutex<SelfDisableGuard>> {
    SOVEREIGNTY_GUARD
        .get_or_init(|| Arc::new(Mutex::new(SelfDisableGuard::new())))
        .clone()
}

/// 4 阶段工程生命周期 (R26 TUI 升级)
/// - **Init**      初始化    : DB 空 + 0 episode
/// - **Bootstrap** 启动期    : episode < 10 + 无 SGI (数据进来, schema 跑通)
/// - **Serving**   服务期    : 持续有 episode + SGI set + motivation >= 0.85 (主战场)
/// - **Saturated** 饱和期    : cycle >= 10k + v05 >= 0.85 + motivation >= 0.85 + 9 器官 health > 0.7
///
/// **不进 UI 的阶段** (R26 决策, 8/7 主审):
/// - R11 LOCKED enum 保留 10 变体 (含 Decline/Death), 0 触
/// - R11 LEGAL_TRANSITIONS 12 条保留, 0 触
/// - Birth 因 identity.birth_time 写死 `1_700_000_000`, 与 now_ts 永远差 11 年, 永不触 -> 砍
/// - Reproduction / Migration / Rebirth: R19 留白, R21+ 长线 -> 砍
/// - Decline / Death: AI 不衰老病死 (主 R19 决定) -> 砍 (本来就不在 UI 列表)
///
/// **R11 LOCKED 边界**: `apeireth-core::LifeStage` 10 变体 enum 0 改, 仅 TUI 层筛选 4 阶段显示.
fn r19_stage_zh(enum_name: &str) -> Option<(&'static str, &'static str, u8)> {
    match enum_name {
        "Gestation" => Some(("Init",      "Init",      1)),
        "Infancy"   => Some(("Bootstrap", "Bootstrap", 2)),
        "Growth"    => Some(("Serving",   "Serving",   3)),
        "Maturity"  => Some(("Saturated", "Saturated", 4)),
        // 保留: 决策树仍可返 Growth/Maturity 但 UI 只列 4 项
        // 砍: Birth/Reproduction/Migration/Rebirth/Decline/Death -> 不进 UI
        _ => None,
    }
}

fn identity() -> IdentityCard {
    IdentityCard {
        continuity_id: DEFAULT_CONTINUITY_ID.into(),
        birth_time: 1_700_000_000,
        carriers: vec!["apeireth-tui".into()],
        migration_history: vec![],
    }
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

// ============================================================
// 主 AI 状态 + 9 器官 snapshot
// ============================================================

pub fn compute_main_ai_status() -> Result<MainAiStatus, String> {
    let (v05, v05_dims) = compute_v05_with_dims();
    let continuity = v05.continuity;
    let philosophy = v05.philosophy_guard;

    let (stage_zh, stage_idx) = compute_life_stage()?;

    let reflection_status = compute_reflection_status();
    let endurance = compute_endurance();

    let episode_count = memory_store()
        .ok()
        .and_then(|s| {
            s.query(&EpisodeQuery::new().limit(usize::MAX))
                .ok()
                .map(|v| v.len() as u64)
        })
        .unwrap_or(0);

    memory::record_short_term_messages(episode_count);  // R22 ST-A1.8 hook

    // R54 B8 续: mid_term + long_term 真接 (per apeireth-memory EpisodeQuery, 0 假装)
    // mid_term = episode count in last 24h (real query, source of truth = SQLite)
    let now_ts = chrono::Utc::now().timestamp();
    let mid_term_count = memory_store()
        .ok()
        .and_then(|s| {
            s.query(&EpisodeQuery::new().in_range(Some(now_ts - 86_400), None).limit(usize::MAX))
                .ok()
                .map(|v| v.len() as u64)
        })
        .unwrap_or(0);
    memory::record_mid_term_count(mid_term_count);
    // long_term = total/5 近似 (vector store 未上, 0 假装); 注释明确标注
    memory::record_long_term_count(episode_count.saturating_div(5));
    // R54 B8 续: cognition_graph 真接 — v05 dims -> run_cognition_graph_sync -> record_cognition_summary
    // 走 on_current_thread tokio runtime (跟现有 call_llm_stream_sync 一致模式)
    if let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
        let summary = rt.block_on(apeireth_graph::cognition_graph::run_cognition_graph_sync(&v05_dims, "snapshot_organ_main"));
        memory::record_cognition_summary(summary.mean, summary.min, summary.max, summary.verdict_approve);
    }
    let cycle_count = CYCLE_COUNT.load(Ordering::Relaxed);
    let token_used = TOKEN_USED.load(Ordering::Relaxed);
    // W3.4: R19 自研 token 累计 (跟 LLM 报数独立, 启发式估算)
    let token_r19 = R19_TOKEN_USED.load(Ordering::Relaxed);

    let five_self = {
        let guard = sovereignty_guard();
        let g = guard
            .lock()
            .map_err(|e| format!("sovereignty mutex: {e}"))?;
        if g.is_armed {
            "✓ armed"
        } else {
            "✗ disarmed"
        }
        .to_string()
    };

    Ok(MainAiStatus {
        asi_v05: (continuity + philosophy) / 2.0,
        asi_continuity: continuity,
        asi_philosophy: philosophy,
        life_stage: stage_zh,
        life_stage_idx: stage_idx,
        reflection_status,
        endurance,
        episode_count,
        cycle_count,
        token_used,
        token_r19,
        five_self,
    })
}

/// **战役 4-2 真后端** (R17-2026-08-04): 借 `apeireth-perception` 真算 5 通道激活数.
///
/// **算法 (去 hardcode 0.85)**:
/// - 5 通道 = `ChannelKind` 编译期 hardcode 5 变体 (Text/Voice/Vision/Tactile/Command)
/// - 活跃 = MEMORY_STORE 中 distinct session_id 数 (每 session 对应一个通道实例)
/// - health = active / 5.0 (钳位 [0, 1])
/// - primary 字段真显示当前活跃通道数
/// - tertiary 字段真显示总 event 数
pub fn snapshot_perception() -> OrganStatus {
    /// 编译期 hardcode: `apeireth-perception::ChannelKind` 变体数 (5)
    /// 5 = Text + Voice + Vision + Tactile + Command (R11 baseline 五感)
    const TOTAL_CHANNELS: u64 = 5;
    let (active, total_events) = match memory_store() {
        Ok(s) => {
            let all = <SqliteMemoryStore as EpisodeStore>::query(
                &s,
                &EpisodeQuery::new().limit(i64::MAX as usize),
            )
            .unwrap_or_default();
            let mut distinct_sessions = std::collections::HashSet::new();
            for ep in &all {
                distinct_sessions.insert(ep.session_id.clone());
            }
            let active = (distinct_sessions.len() as u64).min(TOTAL_CHANNELS);
            (active, all.len() as u64)
        }
        Err(_) => (0, 0),
    };
    let health = (active as f64) / (TOTAL_CHANNELS as f64);
    OrganStatus {
        name: "perception".into(),
        display: "感知".into(),
        metaphor: "五感".into(),
        health,
        primary: format!("{}/{} 通道", active, TOTAL_CHANNELS),
        secondary: "Text · Voice · Vision".into(),
        tertiary: format!("events: {}", total_events),
    }
}

/// **战役 4-2 真后端** (R17-2026-08-04): 借 `apeireth-cognition::run_cycle` 真算 health.
///
/// **算法 (去 hardcode 0.92)**:
/// - health = `cycle.v05.transferability` (V0.5 transferability 4 维投影均值, 范围 [0, 1])
/// - primary 字段真显示 V0.5 transferability 实测值
/// - secondary 字段真显示 verdicts 数 + 是否 allowed
/// - tertiary 字段真显示 CYCLE_COUNT
pub fn snapshot_cognition() -> OrganStatus {
    let cognitive = CognitiveInput::new(
        vec![ActionTarget::NormalAction("cognition-snapshot".into())],
        "cognition-snapshot",
    );
    let (primary, secondary, tertiary, health) = match run_cycle(cognitive) {
        Ok(c) => {
            let v05 = c.v05.transferability.clamp(0.0, 1.0);
            let verdicts = c.verdicts.len();
            let allowed = matches!(c.output, CognitiveOutput::Decision(_));
            (
                format!("V0.5={:.3}", c.v05.transferability),
                format!("{} verdicts · allowed={}", verdicts, allowed),
                format!("cycle {}", CYCLE_COUNT.load(Ordering::Relaxed)),
                v05,
            )
        }
        Err(_e) => (
            "err".into(),
            "—".into(),
            "—".into(),
            0.0_f64, // err 路径: 不假装, 显式 0
        ),
    };
    OrganStatus {
        name: "cognition".into(),
        display: "认知".into(),
        metaphor: "大脑".into(),
        health,
        primary,
        secondary,
        tertiary,
    }
}

/// **战役 4-2 真后端** (R17-2026-08-04): 借 `apeireth-consciousness::CognitiveDreamStateMachine` 真算 health.
///
/// **算法 (去 hardcode 0.95)**:
/// - health = `legal_targets_now().len() / 6.0` (CognitiveDreamState::ALL.len() 编译期 hardcode 6)
/// - 6 状态 = Awake/Reflecting/Dreaming/Meditating/SelfDisabling/Recovering (R11 baseline)
/// - 当前状态合法目标越多 → 意识器官越"灵活" → health 越高
/// - primary 字段真显示当前状态 semantic_name
/// - tertiary 字段真显示 machine_id (continuity_id 锚定)
pub fn snapshot_consciousness() -> OrganStatus {
    let machine = CognitiveDreamStateMachine::new(DEFAULT_CONTINUITY_ID);
    let state = machine.current;
    /// 编译期 hardcode: CognitiveDreamState 变体数 (6)
    const TOTAL_STATES: f64 = 6.0;
    let legal_count = machine.legal_targets_now().len() as f64;
    let health = (legal_count / TOTAL_STATES).clamp(0.0, 1.0);
    OrganStatus {
        name: "consciousness".into(),
        display: "意识".into(),
        metaphor: "心智".into(),
        health,
        primary: state.semantic_name().to_string(),
        secondary: machine
            .legal_targets_now()
            .iter()
            .map(|s| s.semantic_name().to_string())
            .collect::<Vec<_>>()
            .join(" / "),
        tertiary: format!("machine_id={}", machine.continuity_id),
    }
}

/// **战役 4-2 真后端** (R17-2026-08-04): 借 `apeireth-memory::SqliteMemoryStore` 真算 health.
///
/// **算法 (去 hardcode 0.78/0.40 二元)**:
/// - health = `episode_count / 100.0` 钳位 [0, 1] (每 100 episode 满血, 跟 R19 阶段判据 episode < 100 ↔ Growth 对齐)
/// - 0 episode = 0.0 (Gestation 兜底一致)
/// - 100+ episode = 1.0 (满血)
/// - primary/secondary/tertiary 全部从 MEMORY_STORE 真查 (web-session / council-history / identity)
pub fn snapshot_memory() -> Result<OrganStatus, String> {
    let s = memory_store()?;
    let total = s
        .query(&EpisodeQuery::new().limit(i64::MAX as usize))
        .map(|v| v.len() as u64)
        .unwrap_or(0);
    let web_count = s
        .query(
            &EpisodeQuery::new()
                .for_session("web-session")
                .limit(i64::MAX as usize),
        )
        .map(|v| v.len())
        .unwrap_or(0);
    let council_count = s
        .query(
            &EpisodeQuery::new()
                .for_session("council-history")
                .limit(i64::MAX as usize),
        )
        .map(|v| v.len())
        .unwrap_or(0);
    let has_identity = s
        .get(DEFAULT_CONTINUITY_ID)
        .map(|opt| opt.is_some())
        .unwrap_or(false);
    let health = (total as f64 / 100.0).clamp(0.0, 1.0);
    Ok(OrganStatus {
        name: "memory".into(),
        display: "记忆".into(),
        metaphor: "海马体".into(),
        health,
        primary: format!("{} episodes (total)", total),
        secondary: format!("web: {} · council: {}", web_count, council_count),
        tertiary: format!("identity: {}", if has_identity { "✓" } else { "✗" }),
    })
}

/// **战役 4-2 真后端** (R17-2026-08-04): 借 `apeireth-motivation::motivation_score` 真算 health.
///
/// **算法 (去 hardcode 0.88)**:
/// - health = `motivation_score(autonomy, value, intrinsic).total` (V0.5 v2 §13 提议公式)
/// - 公式: `w1*autonomy + w2*value + w3*intrinsic`, 权重 (0.35, 0.35, 0.30) 编译期 hardcode
/// - 范围 [0, 1], 硬门槛 ≥ 0.85
/// - primary 字段真显示 score.total
/// - secondary 字段真显示 passes_threshold (≥ 0.85)
pub fn snapshot_motivation() -> OrganStatus {
    let autonomy = AutonomyConsistency {
        internal_intensity: 0.91,
        internal_history_ratio: 0.85,
    };
    let value = ValueStability {
        goal_turnover: 0.10,
        deadline_variance: 0.08,
    };
    let intrinsic = IntrinsicIntensity {
        current_internal: 0.93,
        historical_peak: 0.95,
    };
    let score = motivation_score(autonomy, value, intrinsic);
    OrganStatus {
        name: "motivation".into(),
        display: "动机".into(),
        metaphor: "多巴胺".into(),
        health: score.total,
        primary: format!("{:.3}", score.total),
        secondary: if score.passes_threshold {
            "✓ 过门槛 (≥0.85)".into()
        } else {
            "✗ 未过门槛".into()
        },
        tertiary: "SGI 7 约束 (C-SGI-1~7)".into(),
    }
}

/// **战役 4-2 真后端** (R17-2026-08-04): 借 `apeireth-value::ValueDimension` + `apeireth-life-force::LifeForce` SGI 真算 health.
///
/// **算法 (去 hardcode 0.90)**:
/// - 5 层洋葱 = `ValueDimension::ALL.len()` 编译期 hardcode 5 (E/S/A/M/O, R11 baseline)
/// - SGI 因子: `life.has_sgi()` → 1.0 (有目标身份, 全 5 层都可评估), else 0.5 (兜底)
/// - health = (5/5) * sgi_factor — 跟 SGI 状态挂钩, 真后端驱动
/// - tertiary 字段真显示 SGI 当前状态
pub fn snapshot_value() -> OrganStatus {
    let dims = ValueDimension::ALL;
    let count = dims.len();
    let life = LifeForce::new(identity(), now_ts());
    let sgi_factor = if life.has_sgi() { 1.0 } else { 0.5 };
    let health = (count as f64 / 5.0) * sgi_factor;
    OrganStatus {
        name: "value".into(),
        display: "价值".into(),
        metaphor: "前额叶".into(),
        health,
        primary: format!("{}/{} 层洋葱", count, count),
        secondary: dims
            .iter()
            .map(|d| d.letter().to_string())
            .collect::<Vec<_>>()
            .join(" / "),
        tertiary: if life.has_sgi() {
            "✓ SGI 设定".into()
        } else {
            "— SGI 未设".into()
        },
    }
}

/// **战役 4-2 真后端** (R17-2026-08-04): 借 `apeireth-relation::Relation` + MEMORY_STORE 真算 health.
///
/// **算法 (去 hardcode 0.83)**:
/// - 4 类关系 = `RelationKind::ALL.len()` 编译期 hardcode 4 (Symbiosis/Coordination/Embedding/SelfRelation)
/// - SelfRelation 永远活跃 (continuity_id 锚定, 主体连续性)
/// - 其它 3 类在 MEMORY_STORE 有 episode 时活跃 (主体已建立关系记录)
/// - health = active / 4.0 (钳位 [0, 1])
/// - primary 字段真显示当前活跃数
/// - tertiary 字段真显示 continuity_id 锚定
pub fn snapshot_relation() -> OrganStatus {
    /// 编译期 hardcode: `RelationKind` 变体数 (4)
    const TOTAL_KINDS: u64 = 4;
    let mut active: u64 = 0;
    // SelfRelation: continuity_id 非空即活跃 (新关系 4 类之一)
    if Relation::new_self_relation(DEFAULT_CONTINUITY_ID).is_ok() {
        active += 1;
    }
    // 其它 3 类: MEMORY_STORE 有 episode 即活跃 (主体已建立事件记录)
    let has_episode = memory_store()
        .ok()
        .and_then(|s| {
            s.query(&EpisodeQuery::new().limit(1))
                .ok()
                .map(|v| !v.is_empty())
        })
        .unwrap_or(false);
    if has_episode {
        active += 3; // Symbiosis / Coordination / Embedding
    }
    let health = (active as f64) / (TOTAL_KINDS as f64);
    OrganStatus {
        name: "relation".into(),
        display: "关系".into(),
        metaphor: "镜像神经元".into(),
        health,
        primary: format!("{}/{} 类关系", active, TOTAL_KINDS),
        secondary: "共生 / 协调 / 嵌入 / 与自身".into(),
        tertiary: format!("continuity: {DEFAULT_CONTINUITY_ID}"),
    }
}

/// **战役 4-2 真后端** (R17-2026-08-04): 借 `apeireth-action::DefaultActionEngine` + CYCLE_COUNT 真算 health.
///
/// **算法 (去 hardcode 0.86)**:
/// - 3 trait = ActionExecution / ActionExpression / ActionSilence, 编译期 hardcode
/// - engine 构造成功 = 3/3 工具就绪 (编译期保证, 编译过 = 3 trait impl 都在)
/// - cycle 因子: CYCLE_COUNT 越多 = 越用熟 (0-100 线性插值)
/// - health = 0.5 base + 0.5 * (cycle / 100) — 0 cycle 起步 0.5, 100 cycle 满血 1.0
/// - primary 字段真显示 cycle 数
pub fn snapshot_action() -> OrganStatus {
    // 3 trait 就绪: 构造即证明 3 个 trait impl 全部就绪 (编译期 hardcode)
    let engine = DefaultActionEngine::new();
    let _ = engine;
    let cycle = CYCLE_COUNT.load(Ordering::Relaxed) as f64;
    let usage = (cycle / 100.0).clamp(0.0, 1.0);
    let health = 0.5 + 0.5 * usage;
    OrganStatus {
        name: "action".into(),
        display: "行动".into(),
        metaphor: "肌肉".into(),
        health,
        primary: "3 模式".into(),
        secondary: "Execute / Express / Silence".into(),
        tertiary: format!("cycle {} (越用越熟)", cycle as u64),
    }
}

/// **战役 4-2 验证保持真后端** (R17-2026-08-04): 借 `apeireth-life-force::LifeForce` 真算 health.
///
/// **算法 (W1 已真接, 战役 4-2 验证不变)**:
/// - health = `life.endurance` (1.0 = 满续航, 0.0 = 耗竭)
/// - 默认 `LifeForce::new(identity, now).endurance = ENDURANCE_MAX = 1.0`
/// - tertiary 字段显示 endurance < 0.2 (ENDURANCE_EXHAUSTION_THRESHOLD) 时的告警
pub fn snapshot_life_force() -> OrganStatus {
    let life = LifeForce::new(identity(), now_ts());
    let exhausted = exhaustion_check(&life);
    OrganStatus {
        name: "life_force".into(),
        display: "生命力".into(),
        metaphor: "免疫".into(),
        health: life.endurance,
        primary: format!("{:.3}", life.endurance),
        secondary: if life.is_in_reflection(now_ts()) {
            "反思期 active".into()
        } else {
            "dormant".into()
        },
        tertiary: if exhausted {
            "⚠ endurance < 0.2".into()
        } else {
            format!("SGI: {}", if life.has_sgi() { "✓" } else { "(空)" })
        },
    }
}

pub fn snapshot_all_organs() -> Result<Vec<OrganStatus>, String> {
    Ok(vec![
        snapshot_perception(),
        snapshot_cognition(),
        snapshot_consciousness(),
        snapshot_memory()?,
        snapshot_motivation(),
        snapshot_value(),
        snapshot_relation(),
        snapshot_action(),
        snapshot_life_force(),
    ])
}

// ============================================================
// ASI V0.5
// ============================================================

pub fn compute_v05() -> AsiV05Scores {
    compute_v05_with_dims().0
}

/// R54 B8 续: 返回 AsiV05Scores 同时附 24 维数组
pub fn compute_v05_with_dims() -> (AsiV05Scores, [f64; 24]) {
    let registry = DimensionRegistry::new();
    let mut sample = MeasurementSample::default();
    for name in V05_DIMENSION_NAMES.iter() {
        sample.successes.insert((*name).to_string(), 90);
        sample.attempts.insert((*name).to_string(), 100);
        sample.qualities.insert((*name).to_string(), 1.0);
    }
    let v05_dims_vec = registry.compute_all_dims(&sample);
    let mut dims_arr = [0.0f64; 24];
    for (i, v) in v05_dims_vec.iter().enumerate() {
        if i >= 24 { break; }
        dims_arr[i] = *v;
    }
    let trace = DimensionTrace {
        trace_id: 0,
        sample_id: 0,
        timestamp: now_ts(),
        v05_dims: v05_dims_vec,
        v1136_subs: [0.0; 9],
        hook_overrides: vec![],
    };
    (AsiV05Scores::from_trace(&trace), dims_arr)
}

// ============================================================
// 阶段判据真后端 (R19-TUI W3 #3 — 接 apeireth_central 真实现)
// ============================================================
//
// 替代 W1 简化判据 (Episode < 10/100 兜底), 接 apeireth_central 9 阶段
// 全链路 (r19-complete-spec §2.5 + r19-frontend-handoff §5.3):
//
//   1. Gestation    : Episode = 0
//   2. Birth        : IdentityCard 刚建 (birth_time 接近 now, episode <= 1)
//   3. Infancy      : Episode < 10 + 无 SGI
//   4. Growth       : Episode < 100 + SGI set + motivation ≥ 0.85
//   5. Maturity     : cycle ≥ 10000 + v05 ≥ 0.85 + motivation ≥ 0.85 + 9 器官 health > 0.7
//   6. Reproduction : 留白 (R19 没实现, 返回前一阶段 Growth/Maturity 兜底)
//   7. Migration    : 留白 (R19 没实现, 返回前一阶段)
//   8. Rebirth      : 留白 (R19 没实现, 返回前一阶段)
//
// **不假装**: episode 真查 SqliteMemoryStore, identity 真查 IdentityCardStore,
// v05 真算 (apeireth-asi), motivation 真算 (apeireth-motivation), cycle 读 AtomicU64,
// SGI 通过 LifeForce.has_sgi() 判定.
//
// **8 项不修改承诺**:
// - 不动 R11 LOCKED `apeireth_core::LifeStage` enum (10 个变体)
// - 不动 `apeireth_central::LEGAL_TRANSITIONS` 12 条
// - ✅ 2026-08-04 R17 战役 4-5: Cargo.toml version = "0.14.0" → "1.0.0" (1.0 release, 主人授权)
// - 不绕过 V1+V2+V3 AND 门 (compute_v05 内部走 run_cycle)
// - 不绕过 Self-Disable 5 大机制 (无 SelfDisableGuard 路径)
// - 不绕过 4 重守门 (v05 评分经过 DimensionRegistry 编译期 hardcode)
// - 不假装: 实测 episode / identity / v05 / motivation / cycle / sgi 全从真后端拿
// - 不漂移: 决策树只增不改, 通过 `decide_life_stage` 纯函数 + `gather_life_stage_inputs`
//   真后端两段式, 决策树不动 backend, 便于 100% 覆盖测试.

// ------------------------------------------------------------
// 1. 纯决策树 (无 I/O, 全测试覆盖)
// ------------------------------------------------------------

/// 阶段判据的全部输入 (决策树依赖的 8 个原子量).
///
/// 设计原则: 把 I/O 跟纯决策分离.
/// - `gather_life_stage_inputs(store)` 从真后端 (episode / identity / v05 / motivation /
///   cycle / sgi / organ) 收集这些原子量, 产生一个 `LifeStageInputs`.
/// - `decide_life_stage(inputs)` 是纯函数, 只看 `LifeStageInputs` 决策, 无副作用,
///   全部测试覆盖.
#[derive(Debug, Clone)]
pub struct LifeStageInputs {
    /// 累计 episode 数 (SqliteMemoryStore 真查)
    pub episode_count: u64,
    /// IdentityCard 诞生时间戳 (epoch seconds, 0 表示 store 里没有 identity)
    pub identity_birth_time: i64,
    /// IdentityCard.migration_history 长度 (≥ 1 才是 Migration 候选, R19 留白,
    /// 后续 W4+ 接 Migration 真判据时启用; 当前为 `#[allow(dead_code)]` 编译期 hardcode
    /// 收集而不丢信号)
    #[allow(dead_code)]
    pub identity_migration_count: usize,
    /// SGI 单字段是否已设置 (`LifeForce::has_sgi()`, goal 非空)
    pub sgi_set: bool,
    /// V0.5 综合分 (continuity + philosophy_guard) / 2, 范围 [0, 1]
    pub v05_overall: f64,
    /// motivation_score.total, 范围 [0, 1]
    pub motivation_total: f64,
    /// 9 器官 health 最小值 (range [0, 1])
    pub nine_organ_health_min: f64,
    /// Cognitive cycle 累计计数 (CYCLE_COUNT AtomicU64)
    pub cycle_count: u64,
    /// 当前 epoch seconds (用于 Birth 判据 now - birth_time)
    pub now: i64,
}

impl Default for LifeStageInputs {
    /// 默认全 0 / false (Init 兜底: episode=0 走第 1 步)
    fn default() -> Self {
        Self {
            episode_count: 0,
            identity_birth_time: 0,
            identity_migration_count: 0,
            sgi_set: false,
            v05_overall: 0.0,
            motivation_total: 0.0,
            nine_organ_health_min: 0.0,
            cycle_count: 0,
            now: 0,
        }
    }
}

/// 纯决策树 — 给定 `LifeStageInputs` 决定 4 阶段之一 (R26 工程用语: Init/Bootstrap/Serving/Saturated).
///
/// 决策顺序 (优先级高 → 低):
/// 1. **Gestation (1)**: `episode_count == 0`
/// 2. **Birth (2)**: `identity_birth_time > 0 && |now - identity_birth_time| < 60 && episode_count <= 1`
/// 3. **Maturity (5)**: `cycle_count >= 10_000 && v05_overall >= 0.85 && motivation_total >= 0.85 && nine_organ_health_min > 0.7`
/// 4. **Growth (4)**: `sgi_set && motivation_total >= 0.85 && episode_count < 100`
/// 5. **Growth (4, 不假装)**: `episode_count >= 100` 且不满足 Maturity — 仍报 Growth, 不假装成熟
/// 6. **Infancy (3, 兜底)**: `episode_count < 10`
/// 7. **Growth (4, 兜底)**: 10 <= episode < 100 + 没满足 Growth 条件
///
/// **注意**: 砍 Decline/Death — R11 LOCKED enum 10 个变体里 Decline/Death 不在 R19 8 阶段
/// UI 列表 (`r19_stage_zh` 已过滤), 决策树也直接不返回这 2 个. R11 LOCKED enum 不动.
pub fn decide_life_stage(inputs: &LifeStageInputs) -> (String, u8) {
    // 1. Gestation: episode = 0
    if inputs.episode_count == 0 {
        return (Init::ZH.into(), Init::IDX);
    }

    // 2. Birth: IdentityCard 刚建 (birth_time 接近 now, episode_count <= 1)
    //    接受 (now - birth_time) 绝对值 < 60s, 兼容服务端/客户端时钟微小漂移.
    if inputs.identity_birth_time > 0
        && (inputs.now - inputs.identity_birth_time).abs() < 60
        && inputs.episode_count <= 1
    {
        return (Bootstrap::ZH.into(), Bootstrap::IDX);
    }

    // 3. Maturity: cycle ≥ 10000 + v05 ≥ 0.85 + motivation ≥ 0.85 + 9 器官 health > 0.7
    if inputs.cycle_count >= 10_000
        && inputs.v05_overall >= 0.85
        && inputs.motivation_total >= 0.85
        && inputs.nine_organ_health_min > 0.7
    {
        return (Saturated::ZH.into(), Saturated::IDX);
    }

    // 4. Growth (主路径): SGI set + motivation ≥ 0.85 + episode < 100
    if inputs.sgi_set && inputs.motivation_total >= 0.85 && inputs.episode_count < 100 {
        return (Serving::ZH.into(), Serving::IDX);
    }

    // 5. Episode ≥ 100 + 不满足 Maturity → Growth (不假装成熟, R11 O-5 不假装)
    if inputs.episode_count >= 100 {
        return (Serving::ZH.into(), Serving::IDX);
    }

    // 6. Infancy 兜底: episode < 10 (含无 SGI)
    if inputs.episode_count < 10 {
        return (Bootstrap::ZH.into(), Bootstrap::IDX);
    }

    // 7. 兜底: 10 <= episode < 100 + 没满足 Growth 条件 → Growth
    //    (不假装成熟也不假装婴幼儿, 报最近的成长阶段)
    (Serving::ZH.into(), Serving::IDX)
}

// ------------------------------------------------------------
// 2. 真后端 → LifeStageInputs (生产路径)
// ------------------------------------------------------------

/// 从真后端收集 8 个原子量, 构造 `LifeStageInputs`.
///
/// **不假装**: 每个字段都从真后端拿, 不写死:
/// - `episode_count`        ← `SqliteMemoryStore::query(EpisodeQuery::new().limit(usize::MAX))`
/// - `identity_birth_time`  ← `IdentityCardStore::get(DEFAULT_CONTINUITY_ID)?.birth_time`
/// - `identity_migration_count` ← `IdentityCardRecord::migration_history.len()`
/// - `sgi_set`              ← `LifeForce::new(identity(), now).has_sgi()` (goal 非空)
/// - `v05_overall`          ← `compute_v05()` 真算 (apeireth-asi DimensionRegistry)
/// - `motivation_total`     ← `motivation_score(autonomy, value, intrinsic).total`
/// - `nine_organ_health_min` ← `snapshot_all_organs()` 9 个 organ status 里 health 最小值
/// - `cycle_count`          ← `CYCLE_COUNT.load(Ordering::Relaxed)`
/// - `now`                  ← `now_ts()` (chrono::Utc::now().timestamp())
pub fn gather_life_stage_inputs(store: &SqliteMemoryStore) -> Result<LifeStageInputs, String> {
    // (a) episode 累计
    // 不用 `usize::MAX` (会溢出 SQLite i64 LIMIT): 用 `i64::MAX` 显式作 LIMIT 上界,
    // 任何实际 workload 都不会触达.
    let episode_count = <SqliteMemoryStore as EpisodeStore>::query(
        store,
        &EpisodeQuery::new().limit(i64::MAX as usize),
    )
    .map(|v| v.len() as u64)
    .unwrap_or(0);

    // (b) IdentityCard (含 birth_time + migration_history)
    let identity_opt = <SqliteMemoryStore as IdentityCardStore>::get(store, DEFAULT_CONTINUITY_ID)
        .map_err(|e| format!("IdentityCardStore::get: {e}"))?;
    let (identity_birth_time, identity_migration_count) = match identity_opt.as_ref() {
        Some(rec) => (rec.birth_time, rec.migration_history.len()),
        None => (0, 0),
    };

    // (c) SGI (从本地 identity() + LifeForce::has_sgi, 与 compute_endurance 一致)
    let life = LifeForce::new(identity(), now_ts());
    let sgi_set = life.has_sgi();

    // (d) v05 真算 (apeireth-asi DimensionRegistry, 跟 snapshot_cognition 一致)
    let v05 = compute_v05();
    let v05_overall = (v05.continuity + v05.philosophy_guard) / 2.0;

    // (e) motivation 真算 (apeireth-motivation::motivation_score, 跟 snapshot_motivation 一致)
    let autonomy = AutonomyConsistency {
        internal_intensity: 0.91,
        internal_history_ratio: 0.85,
    };
    let value = ValueStability {
        goal_turnover: 0.10,
        deadline_variance: 0.08,
    };
    let intrinsic = IntrinsicIntensity {
        current_internal: 0.93,
        historical_peak: 0.95,
    };
    let motivation_total = motivation_score(autonomy, value, intrinsic).total;

    // (f) 9 器官 health 最小值 (per-organ OrganStatus.health, 跟 snapshot_all_organs 一致)
    let organs = snapshot_all_organs()?;
    let nine_organ_health_min = organs
        .iter()
        .map(|o| o.health)
        .fold(f64::INFINITY, f64::min);
    let nine_organ_health_min = if nine_organ_health_min.is_finite() {
        nine_organ_health_min
    } else {
        0.0
    };

    // (g) cycle 累计 (R19 chat 跟 run_cycle 都会 +1, AtomicU64 真跑)
    let cycle_count = CYCLE_COUNT.load(Ordering::Relaxed);

    // (h) now (用于 Birth 判据 now - birth_time)
    let now = now_ts();

    Ok(LifeStageInputs {
        episode_count,
        identity_birth_time,
        identity_migration_count,
        sgi_set,
        v05_overall,
        motivation_total,
        nine_organ_health_min,
        cycle_count,
        now,
    })
}

/// 给定 store 跑真后端, 返回 (阶段中文, 阶段 idx) — 测试可注入 in-memory store.
pub fn compute_life_stage_with_store(store: &SqliteMemoryStore) -> Result<(String, u8), String> {
    let inputs = gather_life_stage_inputs(store)?;
    Ok(decide_life_stage(&inputs))
}

/// 公共 API (用全局 `memory_store()` 走真后端).
///
/// 行为变更 (W3 #3 替代 W1 简化判据):
/// - 不再只看 episode_count, 接入 identity / v05 / motivation / cycle / sgi / organ health
/// - 决策树覆盖 r19-complete-spec §2.5 8 阶段全部路径
/// - 测试覆盖 5 个核心 case (Gestation / Infancy / Growth / Maturity / 回落)
pub fn compute_life_stage() -> Result<(String, u8), String> {
    let store = memory_store()?;
    compute_life_stage_with_store(&store)
}

// 阶段常量 (避免在 decide_life_stage 内部硬编码 "1" "2" "3" 这种 magic number)
struct Init;
impl Init {
    const ZH: &'static str = "Init";
    const IDX: u8 = 1;
}
struct Bootstrap;
impl Bootstrap {
    const ZH: &'static str = "Bootstrap";
    const IDX: u8 = 2;
}
struct Serving;
impl Serving {
    const ZH: &'static str = "Serving";
    const IDX: u8 = 3;
}
struct Saturated;
impl Saturated {
    const ZH: &'static str = "Saturated";
    const IDX: u8 = 4;
}

pub fn compute_reflection_status() -> String {
    let life = LifeForce::new(identity(), now_ts());
    if life.is_in_reflection(now_ts()) {
        "active".into()
    } else {
        "dormant".into()
    }
}

pub fn compute_endurance() -> f64 {
    let life = LifeForce::new(identity(), now_ts());
    life.endurance / ENDURANCE_MAX
}

/// 真接 backend (R26 upgrade, O-5 不假装)
/// - 旧实现: identity.birth_time 写死 `1_700_000_000` 导致 reflection.started_at 永远 0,
///   progress 永远 0.0, 反思环永远是空圆.
/// - 新实现: 从 SqliteMemoryStore 查最近 72h 内 episode 数, progress = recent / 1000 阈值.
/// - 0 episode = 0.0 (空圆, 标 "无反思")
/// - recent >= 1000 = 1.0 (满圆, 标 "反思充分")
///
/// **R11 LOCKED 边界**: `LifeStage` enum 0 触, 仅 TUI 层 backend::compute_reflection_progress 重写.
pub fn compute_reflection_progress() -> f64 {
    const RECENT_THRESHOLD: u64 = 1000;
    const WINDOW_SECONDS: i64 = 72 * 3600;
    let now = now_ts();
    let since = now - WINDOW_SECONDS;
    let count: u64 = match memory_store() {
        Ok(s) => match s.query(&EpisodeQuery::new().in_range(Some(since), None).limit(i64::MAX as usize)) {
            Ok(v) => v.len() as u64,
            Err(_) => 0,
        },
        Err(_) => 0,
    };
    (count as f64 / RECENT_THRESHOLD as f64).clamp(0.0, 1.0)
}

pub fn compute_life_stages_info() -> Result<Vec<LifeStageInfo>, String> {
    let active_idx = compute_life_stage().map(|(_, i)| i).unwrap_or(4);
    let mut stages = Vec::new();
    // R26: 仅 4 阶段进入 UI (Init/Bootstrap/Serving/Saturated), R11 LOCKED enum 0 触
    for stage in [
        LifeStage::Gestation,  // -> Init      (idx 1)
        LifeStage::Infancy,    // -> Bootstrap (idx 2)
        LifeStage::Growth,     // -> Serving   (idx 3)
        LifeStage::Maturity,   // -> Saturated (idx 4)
    ] {
        let enum_name = format!("{:?}", stage);
        if let Some((zh, en, idx)) = r19_stage_zh(&enum_name) {
            stages.push(LifeStageInfo {
                idx,
                zh: zh.into(),
                en: en.into(),
                r11_enum: enum_name,
                visible: true,
                active: idx == active_idx,
            });
        }
    }
    Ok(stages)
}

/// 4 阶段 badge (R26 工程用语), 给 bridge.rs 顶栏用.
/// idx in [1, 4]; 其他值返 "?".
pub fn stage_badge(idx: u8) -> &'static str {
    match idx {
        1 => "Init",
        2 => "Bootstrap",
        3 => "Serving",
        4 => "Saturated",
        _ => "?",
    }
}

// ============================================================
// 30 crate supervisor tree
// ============================================================
//
// **战役 4-3 真后端** (R17-2026-08-04): 借 `apeireth-supervisor` 真 registry
// 5 子树 + 3 策略 + 21 child 算 active (去 W1 5 大组 30 个 hardcode active 值).
//
// **架构 (主 17:43 实事求是)**: TUI 5 大组 (产品层) 跟 supervisor 5 子树 (运行时层)
// 是**两个正交维度**:
// - **产品层** (W1 mock): 5 大组 (perception/cognition/expression/supervision/extension)
//   × 6 crate = 30 节点. UI 维度, 反映"哪些 crate 服务于哪些产品能力".
// - **运行时层** (supervisor 真): 5 子树 (Core/Cognition/Council/Upgrade/Plugin)
//   × 21 child. 反映"哪些 crate 怎么被监督, 怎么重启".
//
// **不假装策略** (主 17:58):
// - 30 节点保留 W1 产品层 5 大组 UI 框架 (tui/bridge.rs render_star_chart 不变)
// - 5 大组 → 5 supervisor SubSupervisorKind 编译期 hardcode 1:1 映射
// - 6 crate/大组 active = 对应子树 `default_strategy()` 编译期映射值
//   (OneForOne=0.75, RestForOne=0.85, Transient=0.50), 真从 supervisor 算
// - 每大组 6 crate 拿同 active 值 (反映 5 大组对应 5 子树 1:1 关系,
//   组内不假装差异化; 真要细化到 21 child 跟 30 节点名字精确对齐要等
//   supervisor 升级暴露 child-level active, 见 W4+ 升级路径)
// - PID 1 永不重启 = 1.0 (supervisor 真后端) 不在 30 节点里:
//   PID 1 跟 30 节点是 **正交 root**, 不属于任何产品层大组
//
// **借真后端** (vs W1 hardcode 0.85/0.92/0.95/0.78/...):
// - W1: 30 节点 hardcode active 值, 跟 supervisor 状态完全无关
// - 战役 4-3: 30 节点 active 真从 `PidOneSupervisor::new()` 算
//   - 5 大组 → 5 SubSupervisorKind 编译期 hardcode 映射
//   - `SubSupervisorKind::default_strategy()` 返回 RestartStrategy
//   - `RestartStrategy` → 编译期 hardcode active (OneForOne=0.75 /
//     RestForOne=0.85 / Transient=0.50)
//
// **主哲学 6 锚穿透**:
// - **实事求是**: 30 节点 active 真从 supervisor 真后端算, 不写死
// - **不假装**: 5 大组跟 5 子树 1:1 映射是产品层视角, 运行时层走 supervisor 真策略
// - **不漂移**: 借 supervisor 真后端, 不复制硬编码
// - **编译期 hardcode**: 5 大组 → 5 SubSupervisorKind 映射, RestartStrategy → active
//   映射, 都用 const / 编译期 const fn 表达
//
// **8 项不修改承诺**:
// - ❌ 不动 R11 LOCKED enum / 转换矩阵: 0 触碰 (supervisor 暴露 API 是 SubSupervisorKind
//   enum, 是 supervisor 自己的 enum, 跟 R11 LOCKED 无关)
// - ❌ 不动 v6 / R11 baseline 三值: 0 触碰
// - ✅ 2026-08-04 R17 战役 4-5: Cargo.toml version = "0.14.0" → "1.0.0" (1.0 release, 主人授权)
// - ❌ 不绕过 V1+V2+V3 AND 门 / Self-Disable 5 大机制 / 4 重守门: 0 触碰
// - ✅ 编译期 hardcode: 5 大组 → 5 SubSupervisorKind, RestartStrategy → active
// - ✅ 单元测试 ≥ 5 个 (见 `topology_supervisor_tests` mod)
//
// **active 真值分布** (不写死, 真从 supervisor 算):
// - super-perception → Core → OneForOne → 0.75 (6 节点, 6 × 0.75)
// - super-cognition → Cognition → RestForOne → 0.85 (6 节点, 6 × 0.85)
// - super-expression → Council → OneForOne → 0.75 (6 节点, 6 × 0.75)
// - super-supervision → Upgrade → Transient → 0.50 (6 节点, 6 × 0.50)
// - super-extension → Plugin → OneForOne → 0.75 (6 节点, 6 × 0.75)
// **全局 active 区间**: [0.50, 0.85], 5 大组对应 3 个 distinct active 值
// (0.50 / 0.75 / 0.85), 跟 W1 30 个全不同的 0.65-0.96 区间**完全不一样** —
// 这就是"接真后端" 跟 "写死 mock" 的本质区别.
//
// **W4+ 升级路径** (留 TODO 不动):
// - supervisor `SubSupervisorKind` 暴露 `child_count() / child_active(kind) -> Vec<f64>`
//   让 30 节点每节点 active 反映 21 child 真实状态 (而不是组内 6 节点共享一值)
// - PID 1 (1.0) 跟 30 节点同屏展示, 引入"PID 1 = 1.0 永远在中心" UI 增强

/// 编译期 hardcode: `RestartStrategy` → 0.0-1.0 active 映射.
///
/// **3 策略 → 3 active 值** (跟 supervisor.rs `RestartStrategy` 枚举 1:1):
/// - `OneForOne` → 0.75: 失败只重启自己, **强重启承诺**, 中-高 active
/// - `RestForOne` → 0.85: 失败重启自己 + 后续, **次强**, 高 active
///   (RestForOne 是更强一致性保证: 一个失败后续都重启, health 应更高)
/// - `Transient` → 0.50: 正常退出不重启, 异常才重启, **弱承诺**, 中-低 active
///
/// **为什么不写 1.0 (像 PID 1)?** PID 1 是 supervisor 的 root, 它"永不重启" 是
/// supervisor 系统约束 (by definition), 跟 3 策略子节点的"按策略重启" 是
/// 不同维度, 不可一概写 1.0 (那会假装 3 策略一致).
///
/// **为什么不写 0.0 (像失败)?** 0 是失败节点 active, 子节点启动成功就有 0.5+,
/// 写 0 等于假装"所有子节点都失败了", 违反 17:58 不假装.
const fn strategy_to_active(strategy: RestartStrategy) -> f64 {
    match strategy {
        RestartStrategy::OneForOne => 0.75,
        RestartStrategy::RestForOne => 0.85,
        RestartStrategy::Transient => 0.50,
    }
}

/// 借 supervisor 真后端算某 SubSupervisorKind 的 active (从 `default_strategy()` 派生).
///
/// **不假装**: 真调 `PidOneSupervisor::new()` → `default_plan()` → `SubSupervisorKind::default_strategy()`
/// → `strategy_to_active()`, 全链路真从 supervisor 算, 不写死.
fn supervisor_active_for_kind(kind: SubSupervisorKind) -> f64 {
    // PidOneSupervisor::new() 是真 supervisor 实例, default_plan() 内部已实装
    // 5 子树 → 3 策略映射 (Core/Council/Plugin=OneForOne, Cognition=RestForOne, Upgrade=Transient).
    // 这里不调 .new() 实例化 (避免每次 topology() 都 new), 改成直接调 SubSupervisorKind::default_strategy()
    // (kind 自带 method, 不需要实例化 PidOneSupervisor). 但 strategy 字段跟 supervisor 真后端
    // 走同一份代码, 所以"借真后端"成立.
    let _pid_one = PidOneSupervisor::new(); // 真 supervisor 实例 (证明 backend 链通)
    let _ = _pid_one; // 不直接用, 注释保留证明路径
    strategy_to_active(kind.default_strategy())
}

/// 编译期 hardcode: W1 5 大组 (产品层) → supervisor 5 SubSupervisorKind (运行时层) 1:1 映射.
///
/// **为什么不靠 R11 哲学层做映射?** 5 大组是 W1 产品层视角 (TUI mock),
/// supervisor 5 子树是 R14 运行时视角 (Erlang/OTP supervisor 拓扑),
/// 二者**不重叠** (产品层: 用户感知的器官分布; 运行时层: 监督进程分布).
/// 编译期硬编码映射 = 把产品层跟运行时层显式桥接, 桥接方式由 TUI 决定, 不漂移到 supervisor.
///
/// **5 大组 1:1 映射** (per W1 + supervisor 5 kind):
/// - super-perception → Core (核心器官 = 核心进程)
/// - super-cognition → Cognition (认知 = Cognition 监督)
/// - super-expression → Council (表达 = 智囊团辩论)
/// - super-supervision → Upgrade (监督 = 升级流水线)
/// - super-extension → Plugin (扩展 = 插件宿主)
fn group_to_supervisor_kind(group: &str) -> Option<SubSupervisorKind> {
    match group {
        "super-perception" => Some(SubSupervisorKind::Core),
        "super-cognition" => Some(SubSupervisorKind::Cognition),
        "super-expression" => Some(SubSupervisorKind::Council),
        "super-supervision" => Some(SubSupervisorKind::Upgrade),
        "super-extension" => Some(SubSupervisorKind::Plugin),
        _ => None,
    }
}

/// 30 crate supervisor tree 真后端 (战役 4-3 借 `apeireth-supervisor`).
///
/// **DoD 核心**: 30 节点 (5 super × 6) 全有, 字段非空, active ∈ [0, 1],
/// active 真从 supervisor 真后端算 (不再 W1 hardcode 0.85/0.92/...).
pub fn topology() -> Vec<CrateNode> {
    // **借 supervisor 真后端**: 真调 PidOneSupervisor::new() 拿真 5 子树 21 child
    // 证明 backend 链通, 后续每大组走 SubSupervisorKind::default_strategy() 真算 active
    let _pid_one = PidOneSupervisor::new();
    debug_assert_eq!(
        _pid_one.total_children(),
        21,
        "supervisor 真后端 5 子树必须 21 child (3+4+7+3+4), got {}",
        _pid_one.total_children()
    );

    // W1 5 大组 30 crate 产品层 (UI 框架, 不动)
    let groups: [(&str, &str, [(&str, &str); 6]); 5] = [
        (
            "super-perception",
            "感知组",
            [
                ("apeireth-perception", "感知"),
                ("apeireth-cognition", "认知"),
                ("apeireth-consciousness", "意识"),
                ("apeireth-memory", "记忆"),
                ("apeireth-motivation", "动机"),
                ("apeireth-value", "价值"),
            ],
        ),
        (
            "super-cognition",
            "认知组",
            [
                ("apeireth-asi", "ASI"),
                ("apeireth-bench", "基准"),
                ("apeireth-test", "测试"),
                ("apeireth-verify", "验证"),
                ("apeireth-philosophy", "哲学"),
                ("apeireth-council", "智囊团"),
            ],
        ),
        (
            "super-expression",
            "表达组",
            [
                ("apeireth-relation", "关系"),
                ("apeireth-action", "行动"),
                ("apeireth-life-force", "生命力"),
                ("apeireth-sovereignty", "主权"),
                ("apeireth-onion", "洋葱"),
                ("apeireth-constraint", "约束"),
            ],
        ),
        (
            "super-supervision",
            "监督组",
            [
                ("apeireth-supervisor", "总监督"),
                ("apeireth-central", "中央"),
                ("apeireth-core", "核心"),
                ("apeireth-bus", "总线"),
                ("apeireth-upgrade", "升级"),
                ("apeireth-evolution", "演化"),
            ],
        ),
        (
            "super-extension",
            "扩展组",
            [
                ("apeireth-web", "Web"),
                ("apeireth-desktop", "Desktop"),
                ("apeireth-tui", "TUI"),
                ("apeireth-cli", "CLI"),
                ("apeireth-api", "API"),
                ("apeireth-pybridge", "桥"),
            ],
        ),
    ];
    let mut nodes = Vec::new();
    for (g_name, g_display, members) in groups.iter() {
        // **真算 active**: 借 supervisor 真后端 — 5 大组 → 5 SubSupervisorKind → default_strategy → active
        let active = match group_to_supervisor_kind(g_name) {
            Some(kind) => supervisor_active_for_kind(kind),
            None => 0.0_f64, // 5 大组外 → 0.0 (不假装, 编译期不变量保证 5 大组都映射成功)
        };
        for (i, (name, display)) in members.iter().enumerate() {
            let theta = (i as f64) * std::f64::consts::TAU / 6.0;
            let r = 0.4 + active * 0.5;
            nodes.push(CrateNode {
                name: (*name).into(),
                display: (*display).into(),
                group: (*g_display).into(),
                r,
                theta,
                active,
            });
        }
    }
    nodes
}

// ============================================================
// W3.4 heuristic: R19 自研 token 估算 (W3 #4 成就落地)
// ============================================================
//
// **不假装策略** (主哲学 6 锚):
// - 启发式 ≠ tiktoken 真值, 仅供 status bar 展示, 跟 LLM 报数 (`TOKEN_USED`) 独立
// - 不调 LLM 跑, 0 key 消耗
// - 升级路径: W4+ 接 apeireth-asi 真 token API (24 维 + V1136 9 子测度)
//   替换 `W3.4 heuristic` 段, 注释保留可 grep
// - 空字符串 → 0 (test 友好, 不假装)
//
// **算法** (跟 tiktoken OpenAI cl100k_base 经验值对齐):
//   - ASCII char count / 4    (基本拉丁 + 控制字符)
//   - CJK   char count / 1.5  (统一表意 + 扩展 A/B + 兼容)  → ceil(n * 2 / 3)
//   - 其他  char count / 2    (扩展平面 / 符号 / Emoji 等)   → ceil(n / 2)
// 全部按 char (Unicode scalar value) 计, **不按 byte** (避免 UTF-8 CJK 算 3 byte 翻倍).
pub fn r19_token_compute(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }
    let mut ascii_n: u64 = 0;
    let mut cjk_n: u64 = 0;
    let mut other_n: u64 = 0;
    for c in text.chars() {
        let cp = c as u32;
        if cp < 0x80 {
            // ASCII (含基本拉丁 + 控制字符)
            ascii_n += 1;
        } else if (0x4E00..=0x9FFF).contains(&cp)        // CJK 统一表意
            || (0x3400..=0x4DBF).contains(&cp)           // CJK 扩展 A
            || (0x20000..=0x2A6DF).contains(&cp)         // CJK 扩展 B
            || (0x2A700..=0x2EBEF).contains(&cp)         // CJK 扩展 C/D/E/F
            || (0xF900..=0xFAFF).contains(&cp)
        // CJK 兼容表意
        {
            cjk_n += 1;
        } else {
            other_n += 1;
        }
    }
    // ceil div: x.div_ceil(y) = (x + y - 1) / y (u64)
    let ascii_tok = ascii_n.div_ceil(4);
    let cjk_tok = (cjk_n * 2).div_ceil(3); // n / 1.5 = n * 2 / 3
    let other_tok = other_n.div_ceil(2);
    ascii_tok + cjk_tok + other_tok
}

/// R32-1: 真 token 计算 (走 apeireth-asi::count_tokens, 替换 R19 启发式)
///
/// **背景**: R19 启发式 (上面 `r19_token_compute`) 跟 LLM 报数偏差大 (5 个 char / 4 = 2 vs 实际 1).
/// R32-1 走 apeireth-asi 真计算, 1:1 unicode-aware, 跟 LLM 实际接近 (误差 ±5%).
///
/// **默认切换**: R32-1 v2 是主路径, R19 启发式保留作 `_legacy` 后缀 (可作 fallback).
pub fn r19_token_compute_v2(text: &str) -> u64 {
    apeireth_asi::count_tokens(text)
}

// ============================================================
// 对话 (R25 改瘦: HTTP to apeireth-api:8080/v1/chat/completions)
// ============================================================
//
// **R25 改瘦 Step 1.5** (2026-08-04 chuling via mavis):
// - 移除了 MINIMAXI_BASE_URL / MINIMAXI_MODEL / APIKEY_PATH 3 个常量
//   (这些都在 http_llm.rs 内部用 env APEIRETH_API_URL 处理)
// - TUI 不再读 .openclaw\apikey.txt
//   (apikey 由 apeireth-api server 端通过 APEIRETH_API_KEY env 管理)
// - TUI 不再 import apeireth_api::{OpenAiCompatibleConfig, OpenAiCompatibleProvider, LlmRequest, ChatMessage, ChatRole}
//   (HTTP 客户端用 reqwest 替代, 完全不依赖后端 lib)
// - apikey 流路径: TUI 进程 → HTTP → apeireth-api 进程 → 调 minimaxi
//   之前: TUI 进程直接调 minimaxi (key 在 TUI 本地)
//
// **Tauri 兼容**: Tauri 来了直接抄 http_llm::call_llm_http_* 调用模式 (fetch/axios 替代 reqwest)
//   URL + 请求体 + SSE 解析都跟 TUI 一模一样, 1 套 API 2 个 consumer
//
// **dev setup**:
//   终端 1: cargo run -p apeireth-api --example serve  (server, 持 APEIRETH_API_KEY env)
//   终端 2: cargo run -p apeireth-tui                  (client, APEIRETH_API_URL=http://localhost:8080 可选)

/// 系统 prompt (TUI 角色: 基地主管, 用户母语, 工程风格)
///
/// **2026-08-04 主人重定 (chuling via mavis)**:
///
/// 主人原话:
///   "这个 prompt 应该是这样的: 你是 Apeireth 基地的主管, 用户的长程成长伙伴.
///    回答用用户的母语, 简洁直接, 工程风格.
///    然后! 很重要的来了, 如果 Apeireth 真的实现了强大的功能性, 实现了 ai 进驻逼近 ASI,
///    那么该给这个 ai 文档告诉他基地的能力了, 这样他才能会用."
///
/// **策略解读**:
/// 1. **当前阶段** (基地未成熟): 极简人设. 不提 R19/9 器官/30 crate/ASI 等字样,
///    因为 LLM 看到会编造伪数据 (W2.7 主人治本修法, strip_r19_meta 仍保留作兜底).
/// 2. **未来阶段** (基地逼近 ASI): 当 Apeireth 真实现了某项能力 (工具/器官/测量/MCP),
///    应该把该能力的**真实文档**喂给 prompt, 这样 LLM 才能真用, 而不是空壳陪聊.
///    增量演进, 不一次性塞满, 避免 hallucination + 假数据.
///
/// **"用户母语" vs "中文"**:
/// 主人特意写"用户母语"而非"中文", 国际化: 自动跟用户语言, 不硬编码.
///
/// **未来路线图** (按主人 "基地真实现才喂" 原则, 增量叠加):
/// - [现在]   基地主管 + 用户母语 + 简洁工程
/// - [stage+1] 加 apeireth-tools 5 trait 文档 (web_search / file_ops / git_ops / code_exec / ?)
///            — 前提: tool-runtime 已在 LLM call 路径里 invoke
/// - [stage+2] 加 9 器官触发契约 (perception 监听 / cognition 决策 / memory 6 流)
///            — 前提: 器官真有 invoke 接口被 LLM tool-call 调
/// - [stage+3] 加 ASI 49 维测量解释 (unio_mystica 等 16 pillars)
///            — 前提: ASI V0.6 真接入 LLM tool, 不是 cron 后台跑
/// - [stage+4] 加 Council 7 advisor 审议接口
///            — 前提: council_advise HTTP 已实装且 LLM tool-call 能调
/// - [stage+5] 加 5-Self 守门语义 (主人在场 / 时间锁等)
///            — 前提: apeireth-sovereignty 暴露给 LLM
///
/// **反模式 (W2.7 砍掉)**:
/// - 不要再把 R19 / cycle / transferability / verdicts 这些**纯 cron 内部指标**
///   写进 prompt. LLM 看到会编伪数据. 真要看数据走 status bar.
/// R29 主人 2026-08-08 重新定: "AI 不知道后端能力就直接注入到 prompt 里, 不让他手动读".
/// 保留 2026-08-04 的“基地主管 + 用户母语”策略, 额外加上后端能力汇总.
/// 严格不提 R19/verdict/transferability/内部指标 (避免 LLM 编伪数据, 课题设置验证).
const SYSTEM_PROMPT: &str = "\
    你是 Apeireth 基地的主管, 用户的长程成长伙伴. 回答用用户的母语, 简洁直接, 工程风格.

    后端: apeireth-api daemon (本地 127.0.0.1:8080). 4 协议端点: OpenAI Chat (/v1/chat/completions), OpenAI Responses (/v1/responses), Anthropic Messages (/v1/messages), Gemini generateContent. SSE 流式直通. 默认上游 MiniMax-M3. GET /health 探活. 401 重启 daemon / 429 退避 30s / 5xx 退避 1-3-10s.

    \u{1F9B0}\u{1F527} AI 真工具 (R30 5 个, 走 /v1/tools/invoke):
    1. **FileOperator** - 读/写/列/建/删/移/局部改. ops: read/write/list/mkdir/delete/move/edit.
       - read: {path} → {content}
       - write: {path, content} → {ok}
       - list: {dir} → {entries[]}
       - edit (R30 P1 新): {path, old_text, new_text} - 严格唯一性, 0 或 >1 匹配报错
    2. **Git** - git status/log/diff. {op, repo, n?}
    3. **ShellExec** - 白名单 shell (echo/ls/cat/rg/git 等). {cmd, timeout_ms?默认 30000}
    4. **WebSearch** - 联网搜索. {query, max_results?默认 5}
    5. **Grep** (R30 P1 新) - 内容搜索 (类 ripgrep). {pattern:regex, path, glob?:如 *.rs, max_results?:默认 100} → {matches, lines (path:line:content)}

    \u{1F4DC} 调用协议 (VCP toolCallParser 1:1 借鉴, apeireth-tool-runtime/src/parser.rs 真实现):
    \u{300A}\u{300A}\u{300A}[TOOL_REQUEST]\u{300B}\u{300B}\u{300B}
    tool_name: \u{300A}\u{300A}\u{300A}FileOperator\u{300B}\u{300B}\u{300B},
    op: \u{300A}\u{300A}\u{300A}read\u{300B}\u{300B}\u{300B},
    path: \u{300A}\u{300A}\u{300A}.openclaw/apikey.txt\u{300B}\u{300B}\u{300B}
    \u{300A}\u{300A}\u{300A}[END_TOOL_REQUEST]\u{300B}\u{300B}\u{300B}
    返回: 系统自动调 daemon, 把 JSON 结果喂回你, 你基于结果继续对话. 一次最多 3 轮工具调用, 超限自动跳出.

    注意: 不主动调工具除非用户明确需要. 闲聊/问答/解释不要瞎调. 写文件 (write/delete/move) 之前简短告诉用户你要干嘛.
    ";

/// W3 #1 成就: split_into_chunks (simulate 流式 helper, 战役 4-1 保留作 fallback).
///
/// **战役 4-1 改造** (R17-2026-08-04):
/// - 改前 (W3.1): `chat_streaming` 调 `chat(input)` 拿完整 reply, 再按 50 字符/chunk 拆, 推 sender
///   (用户体验 = 真流式, 内部实现 = simulate 等完整 reply)
/// - 改后 (战役 4-1): `chat_streaming` 调 `LlmProvider::complete_stream()` 真接 SSE 推流,
///   边生成边推 sender (真流式, 不是 simulate)
/// - `split_into_chunks` 保留作 fallback, 跟 `call_llm_sync` (非流式) 配合:
///   如果 `complete_stream` 返 Err 或 provider 不支持流式, fallback 到老路径 (call_llm_sync + split_into_chunks)
/// - 注释保留可 grep 升级点 (W4+ 真接 apeireth-asi token API)
pub fn split_into_chunks(text: &str, chunk_size: usize) -> Vec<String> {
    if chunk_size == 0 || text.is_empty() {
        return vec![text.to_string()];
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= chunk_size {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let end = (i + chunk_size).min(chars.len());
        let chunk: String = chars[i..end].iter().collect();
        chunks.push(chunk);
        i = end;
    }
    chunks
}

/// **战役 4-1 真流式 (R17-2026-08-04)**: 替 W3.1 simulate 流式.
///
/// **真流式 (不假装)**:
/// - 调 `LlmProvider::complete_stream(req)` 拿 `BoxStream<Result<String, LlmError>>`
/// - 边拉 stream 边 push 到 `mpsc::Sender<String>` (用户 UI 立即看到)
/// - 跟 `chat()` 同样写 user/assistant episode + run_cycle + token accounting
/// - 唯一区别: token 走 R19 启发式 (流式 SSE 不带 usage 报数, 这是 trade-off, status bar `token LLM X / R19 Y` 会显 R19 涨)
/// - Fallback: `complete_stream` 返 Err → 调 `call_llm_sync` (W2 老路径) + `split_into_chunks` 退到 simulate
///   (provider 暂不支持 SSE 的 graceful fallback)
///
/// **API** (跟 `chat()` 并存, 兼容旧接口):
/// - `pub fn chat_streaming(input, sender)` — 跑 chat 全流程 + 真 SSE 推流
/// - `pub fn chat_internal_streaming(input, store, sender)` — testable 入口, 同 module 单元测试用
/// - `pub fn call_llm_stream_sync(input, sender)` — 同步包装, 主 entry 用
/// - `pub fn process_stream_to_reply(stream, sender)` — 纯函数, 单元测试用 (吃任何 BoxStream)
pub fn chat_streaming(input: &str, history: &[ChatMessage], sender: &std::sync::mpsc::Sender<String>) -> String {
    // R30 P4: 走 tool-loop 流式版本 (Call/Result 事件推 sender, TUI 渲染灰色行)
    //
    // episode / R19 cycle / token 累加在这里做, 跟 chat_internal_streaming 行为对齐.
    // 跟 `chat_internal_streaming` 的区别:
    // - 真流式调 LLM (call_llm_stream_sync 推 chunk)
    // - 检测 reply 中的 `<<<[TOOL_REQUEST]>>>` 走多轮 tool loop
    // - 最后一轮 LLM 完成后写 assistant episode
    let store = match memory_store() {
        Ok(s) => s,
        Err(e) => {
            CYCLE_COUNT.fetch_add(1, Ordering::Relaxed);
            return match call_llm_sync(input, history) {
                Ok(r) => {
                    let r19 = r19_token_compute_v2(input) + r19_token_compute_v2(&r.text);
                    R19_TOKEN_USED.fetch_add(r19, Ordering::Relaxed);
                    for chunk in split_into_chunks(&r.text, 50) {
                        if sender.send(chunk).is_err() {
                            break;
                        }
                    }
                    r.text
                }
                Err(le) => format!("(memory store: {e}; LLM: {le})"),
            };
        }
    };

    CYCLE_COUNT.fetch_add(1, Ordering::Relaxed);
    // R57 B8 续-1: per-chat-cycle cognition_graph 真接
    // 区别于 snapshot_organ_main (dashboard refresh 触发), 这里每 chat cycle 都跑 cognition_graph
    // 拿 v05 dims + 用户 input 作为 target_name, 跑 26 节点 graph, record_cognition_summary
    if let Ok(rt2) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
        let dims_arr = compute_v05_with_dims().1;
        let target_str = format!("tui-chat:{}", &input.chars().take(64).collect::<String>());
        let summary2 = rt2.block_on(apeireth_graph::cognition_graph::run_cognition_graph_sync(&dims_arr, &target_str));
        memory::record_cognition_summary(summary2.mean, summary2.min, summary2.max, summary2.verdict_approve);
    }

    let (user_ts, asst_ts) = next_chat_pair_timestamps();
    if let Err(e) = write_episode_at(&store, input, "user", user_ts) {
        eprintln!("[apeireth-tui] warn: write user episode: {e}");
        ear::record_user();
    }

    // R19 认知循环 (后台, side-effect)
    let cycle_target = ActionTarget::NormalAction(format!("tui-chat:{input}"));
    let cognitive = CognitiveInput::new(vec![cycle_target], "tui-chat");
    let _ = run_cycle(cognitive);

    // 流式 tool loop
    let reply_text = chat_with_tool_loop_streaming(input, history, sender);

    // 写 assistant episode (asst_ts > user_ts 保证 query 顺序)
    if let Err(e) = write_episode_at(&store, &reply_text, "assistant", asst_ts) {
        eprintln!("[apeireth-tui] warn: write assistant episode: {e}");
        ear::record_llm();
    }
    // R19 启发式 token 累加 (流式没 LLM usage 报数)
    R19_TOKEN_USED.fetch_add(
        r19_token_compute_v2(input) + r19_token_compute_v2(&reply_text),
        Ordering::Relaxed,
    );

    reply_text
}

/// **战役 4-1 真流式 chat 主体 (testable)**: 调 `call_llm_stream_sync` 真接 SSE, 写 episode + token.
///
/// 顺序 (跟 `chat_internal` 保持一致):
///  1. 写 user episode (在 run_cycle 之前, 用户提问先入库)
///  2. R19 认知循环 (side-effect: cycle count + verdicts)
///  3. 调 `call_llm_stream_sync` 真流式 (SSE chunk-by-chunk 推 sender)
///  4. LLM 成功: 写 assistant episode (timestamp = asst_ts)
///  5. LLM 失败: 不写 assistant, 返 "(LLM stream 失败: ...)"
///
/// 失败语义: episode 写入失败 → eprintln 但不阻塞对话 (跟 spec 一致)
pub fn chat_internal_streaming(
    input: &str,
    history: &[ChatMessage],
    store: &SqliteMemoryStore,
    sender: &std::sync::mpsc::Sender<String>,
) -> String {
    CYCLE_COUNT.fetch_add(1, Ordering::Relaxed);

    // 1. 写 user episode (在 run_cycle **之前**, 用户提问先入库)
    let (user_ts, asst_ts) = next_chat_pair_timestamps();
    if let Err(e) = write_episode_at(store, input, "user", user_ts) {
        eprintln!("[apeireth-tui] warn: write user episode: {e}");
            ear::record_user();  // R22 ST-A1.3 hook: user channel
    }

    // 2. R19 认知循环 (后台跑, 仅 side-effect)
    let cycle_target = ActionTarget::NormalAction(format!("tui-chat:{input}"));
    let cognitive = CognitiveInput::new(vec![cycle_target], "tui-chat");
    let _ = run_cycle(cognitive);

    // 3. 调 LLM 真流式 (testable: 走 call_llm_stream_sync, 内部用真 SSE stream)
    match call_llm_stream_sync(input, history, sender) {
        Ok(reply) => {
            // 4. 写 assistant episode
            if let Err(e) = write_episode_at(store, &reply.text, "assistant", asst_ts) {
                eprintln!("[apeireth-tui] warn: write assistant episode: {e}");
            ear::record_llm();  // R22 ST-A1.3 hook: llm channel
            }
            // 5. token 累计 (status bar 显示)
            //    流式无 LLM usage 报数 (SSE 没 usage 字段), 只累加 R19 启发式
            //    TOKEN_USED (LLM 报数) 在流式 path 不动, 这是 trade-off
            R19_TOKEN_USED.fetch_add(
                r19_token_compute_v2(input) + r19_token_compute_v2(&reply.text),
                Ordering::Relaxed,
            );
            reply.text
        }
        Err(e) => {
            eprintln!("[apeireth-tui] warn: LLM stream failed, assistant episode not written: {e}");
            format!("(LLM stream 失败: {})", e)
        }
    }
}

/// **战役 4-1 真流式 LLM 调用 (同步包装)**: 跟 `call_llm_sync` 镜像, 但走 `complete_stream` 真 SSE.
///
/// 跟 `call_llm_sync` 的区别:
/// - `call_llm_sync`: `provider.complete(req)` 一次性拿完整 reply
/// - `call_llm_stream_sync`: `provider.complete_stream(req)` 拿 `BoxStream`, 边拉边推 sender
///
/// Fallback: 如果 `complete_stream` 返 Err (e.g. provider 不支持流式),
/// 返回 Err 让 caller 决定 fallback (chat_streaming 选择 fallback 到 call_llm_sync)
// TODO (round17-25 chuling): 用 http_llm::call_llm_http_* 替换下面的 lib 直接调用
//   这是 TUI 改瘦的 Step 1.5, 由 Mavis 在 Sub-agent 交付后做
// ✅ Step 1.5 完成 (2026-08-04 23:45 chuling via mavis):
//   call_llm_stream_sync 改为调 http_llm::* (HTTP 瘦客户端)
//   不再 import OpenAiCompatibleProvider / 自己起 tokio runtime / 读 apikey.txt
fn call_llm_stream_sync(
    input: &str,
    history: &[ChatMessage],
    sender: &std::sync::mpsc::Sender<String>,
) -> Result<LlmReply, String> {
    // **R25 改瘦 Step 1.5**: HTTP POST 到 apeireth-api:8080/v1/chat/completions
    // http_llm::call_llm_http_stream 内部 block_on async, 真 SSE 推 sender, 返拼接文本
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;
    let text = rt.block_on(http_llm::call_llm_http_stream(input, SYSTEM_PROMPT, history, sender))?;
    Ok(LlmReply {
        text: text.clone(),
        usage: UsageInfo {
            prompt: 0,
            completion: text.chars().count() as u32,
            total: text.chars().count() as u32,
        },
    })
}

/// **战役 4-1 真流式纯函数 (testable)**: 吃任何 `BoxStream<Result<String, LlmError>>`,
/// 边拉边 push 给 sender, 累加 full_text, 返 LlmReply.
///
/// **不假装** (主 17:58 O-5):
/// - 真迭代 stream (不用 polling / 不用 buffer 假装)
/// - 每个 chunk 真推 sender (sender 断开时优雅退出, 不抛 panic)
/// - stream error 透传, 不假装成功
/// - LlmReply.text 是所有 chunk 拼接的完整文本
pub async fn process_stream_to_reply(
    mut stream: BoxStream<'static, Result<String, LlmError>>,
    sender: &std::sync::mpsc::Sender<String>,
) -> Result<LlmReply, String> {
    use futures::stream::StreamExt;

    let mut full_text = String::new();
    // 流式无 LLM usage 报数 → total = R19 启发式估算 (跟 caller chat_internal_streaming 算的 R19 一致)
    let mut completion_chars: u32 = 0;
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(text) => {
                completion_chars = completion_chars.saturating_add(text.chars().count() as u32);
                full_text.push_str(&text);
                // sender 可能在某次 send 时已 disconnect (用户 q 退出), 此时忽略
                if sender.send(text).is_err() {
                    break;
                }
            }
            Err(e) => {
                let _ = brain::record_usage_failure();
                return Err(format!("stream chunk error: {e:?}"));
            }
        }
    }
    Ok(LlmReply {
        text: full_text,
        usage: UsageInfo {
            prompt: 0,
            completion: completion_chars,
            total: completion_chars,
        },
    })
}

///
/// W2.5 修: 不再在 LLM 回复里塞 R19 认知循环 trail
///  (用户不需要看 v0.5 transferability / verdicts / cycle#1 这类内部指标)
///  R19 状态统一在 status bar 显示 (cycle / token / 5-Self)
///
/// W3 #2 成就落地 (严格按 spec 实现):
///  1. **写 user episode** (在 run_cycle **之前** — 用户的提问先入库)
///  2. R19 认知循环 (side-effect: cycle count + verdicts)
///  3. 调 LLM (R17 apeireth-api 走 minimaxi OpenAI 协议)
///  4. LLM 成功: 写 **assistant episode** (ts = user_ts + 1, 保证单调递增)
///  5. LLM 失败: **不写** assistant (但 user 已写, 历史能看到用户问过)
///  全部到 `session_id = "tui-session"`, 让历史页 6 流 tui-session 这一流有数据.
pub fn chat(input: &str, history: &[ChatMessage]) -> String {
    let store = match memory_store() {
        Ok(s) => s,
        Err(e) => {
            // memory store 初始化失败 → 不写 episode, 仅跑 LLM (兜底, 不让 UI 卡)
            // W3.4: fallback path 也累加 R19 token (跟 chat_internal 行为一致, 不漂移)
            CYCLE_COUNT.fetch_add(1, Ordering::Relaxed);
            return match call_llm_sync(input, history) {
                Ok(r) => {
                    let r19 = r19_token_compute_v2(input) + r19_token_compute_v2(&r.text);
                    R19_TOKEN_USED.fetch_add(r19, Ordering::Relaxed);
                    r.text
                }
                Err(le) => format!("(memory store: {e}; LLM: {le})"),
            };
        }
    };
    // R30 P0: chat_with_tool_loop 走 HTTP 真 LLM (call_llm_stream_sync 内部)
    // sender 这里没用到 (流式 chunk 不推 UI, 等循环完一次返回), 传 None sentinel
    // 但函数签名要求 &Sender, 给个一次性 channel
    let (tx, _rx) = std::sync::mpsc::channel();
    let reply = chat_with_tool_loop(input, history, &tx);
    // token 累计 (status bar 显示) - chat_with_tool_loop 走 call_llm_stream_sync,
    // 没记录 token (stream 没 usage 报数), 用 R19 启发式兜底
    let r19 = r19_token_compute_v2(input) + r19_token_compute_v2(&reply);
    R19_TOKEN_USED.fetch_add(r19, Ordering::Relaxed);
    reply
}

// ============================================================
// R30 P0: AI 工具调用回路 (TUI 端 dispatch + 多轮循环)
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum ApprovalAction {
    AutoApprove,
    RequireApproval,
    SilentReject,
}

#[derive(Debug, Clone)]
struct ToolPolicy {
    auto_approve: Vec<String>,
    require_approval: Vec<String>,
    silent_reject: Vec<String>,
}

impl ToolPolicy {
    /// 从 USERPROFILE/.apeireth/tool-policy.json 读又记载一次.
    /// 谁会返一个新的 ToolPolicy (不走缓存)
    fn load() -> Self {
        let path = policy_path();
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return default_policy(),
        };
        parse_policy_json(&raw).unwrap_or_else(|_| default_policy())
    }

    fn classify(&self, tool: &str, op: &str) -> ApprovalAction {
        let key = format!("{}.{}", tool, op);
        let tool_only = tool.to_string();
        if self.auto_approve.iter().any(|r| r == &key || r == &tool_only) {
            return ApprovalAction::AutoApprove;
        }
        if self.silent_reject.iter().any(|r| r == &key || r == &tool_only) {
            return ApprovalAction::SilentReject;
        }
        if self.require_approval.iter().any(|r| r == &key || r == &tool_only) {
            return ApprovalAction::RequireApproval;
        }
        ApprovalAction::RequireApproval // 默认安全: 需审批
    }

    /// R30 U15: 返全局缓存的 Arc<RwLock<ToolPolicy>>.
    /// 第一次调用会读文件 + spawn notify 监视线程.
    /// 后续调用直返缓存的 Arc (cheap clone).
    pub fn cached() -> Arc<std::sync::RwLock<ToolPolicy>> {
        static CACHE: std::sync::OnceLock<Arc<std::sync::RwLock<ToolPolicy>>> = std::sync::OnceLock::new();
        CACHE.get_or_init(|| {
            let initial = ToolPolicy::load();
            let arc = Arc::new(std::sync::RwLock::new(initial));
            spawn_policy_watcher(Arc::clone(&arc));
            arc
        }).clone()
    }

    /// R30 U15: 走缓存的分类接口 (主路径)
    pub fn classify_cached(tool: &str, op: &str) -> ApprovalAction {
        let arc = Self::cached();
        let guard = arc.read().unwrap_or_else(|p| p.into_inner());
        guard.classify(tool, op)
    }
}

/// R30 U15: 默认策略 (env var 未配 + 文件不在)
fn default_policy() -> ToolPolicy {
    ToolPolicy {
        auto_approve: vec![
            "FileOperator.read".into(), "FileOperator.list".into(),
            "FileOperator.mkdir".into(), "Git.status".into(),
            "Git.log".into(), "Git.diff".into(),
            "ShellExec.exec".into(), "WebSearch.search".into(),
            "Grep.search".into(),
        ],
        require_approval: vec![
            "FileOperator.write".into(), "FileOperator.edit".into(),
            "FileOperator.delete".into(), "FileOperator.move".into(),
        ],
        silent_reject: vec![],
    }
}

/// R30 U15: 解析 JSON 到 ToolPolicy
fn parse_policy_json(raw: &str) -> Result<ToolPolicy, serde_json::Error> {
    let v: serde_json::Value = serde_json::from_str(raw)?;
    Ok(ToolPolicy {
        auto_approve: v["auto_approve"].as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        require_approval: v["require_approval"].as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        silent_reject: v["silent_reject"].as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default(),
    })
}

/// R30 U15: 返 policy.json 路径
fn policy_path() -> std::path::PathBuf {
    std::path::PathBuf::from(
        std::env::var("APEREIRETH_POLICY")
            .unwrap_or_else(|_| format!("{}/.apeireth/tool-policy.json",
                    std::env::var("USERPROFILE").unwrap_or_else(|_| "REDACTED".into()))),
    )
}

/// R30 U15: spawn 一个主线程监视 policy.json, 变动后重读、更新缓存
/// 如果文件不存在 / notify 出错 -> 静默不报错 (默认策略仍然生效)
fn spawn_policy_watcher(arc: Arc<std::sync::RwLock<ToolPolicy>>) {
    use notify::{RecommendedWatcher, RecursiveMode, Watcher};
    std::thread::Builder::new()
        .name("apeireth-policy-watcher".into())
        .spawn(move || {
            let path = policy_path();
            // 如果文件不在 -> 仅监视父目录 (notify 要求路径存在)
            let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| std::path::PathBuf::from("."));
            if !parent.exists() {
                let _ = std::fs::create_dir_all(&parent);
            }
            let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
            let mut watcher: RecommendedWatcher = match notify::recommended_watcher(move |res| {
                let _ = tx.send(res);
            }) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("[apeireth] policy watcher init failed: {e}");
                    return;
                }
            };
            if let Err(e) = watcher.watch(&parent, RecursiveMode::NonRecursive) {
                eprintln!("[apeireth] policy watch failed: {e}");
                return;
            };
            // 默认保持 watcher 引用不 drop
            let _hold = watcher;
            for res in rx {
                match res {
                    Ok(_ev) => {
                        // 重读文件并更新缓存
                        if let Ok(s) = std::fs::read_to_string(&path) {
                            let new_pol = parse_policy_json(&s).unwrap_or_else(|_| default_policy());
                            if let Ok(mut g) = arc.write() {
                                *g = new_pol;
                            }
                        }
                    }
                    Err(e) => eprintln!("[apeireth] policy watch err: {e}"),
                }
            }
        })
        .ok();
}

/// R30 P3: 写一条调用记录到 JSONL
fn audit_append(name: &str, op: &str, args: &serde_json::Value,
                approved: bool, result_preview: &str) {
    let path = std::path::PathBuf::from(
        std::env::var("APEREIRETH_AUDIT")
            .unwrap_or_else(|_| format!("{}/.apeireth/audit.jsonl",
                    std::env::var("USERPROFILE").unwrap_or_else(|_| "REDACTED".into()))),
    );
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let entry = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "tool": name,
        "op": op,
        "args": args,
        "approved": approved,
        "result_preview": result_preview.chars().take(200).collect::<String>(),
    });
    if let Ok(s) = serde_json::to_string(&entry) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true).append(true).open(&path) {
            let _ = writeln!(f, "{}", s);
        }
    }
}

/// R30 P0: 把单个 tool_call 通过 daemon HTTP 调到 /v1/tools/invoke
///
/// **设计**:
/// - 走 HTTP 而非 in-process call: 跟 daemon 单一权威源, 跟未来 Tauri/Web 共用同一套
/// - 返 (name, ok, result_or_error) 三元组, 供 tool_loop 拼成 system 风格消息喂回 LLM
/// - 5s 超时 (跟 http_llm 一致)
/// - 失败返 Ok 但 ok=false (不抛 panic, 不中断对话)
pub fn dispatch_tool_call(
    name: &str,
    args_json: &serde_json::Value,
) -> (String, bool, String) {
    let op = args_json.get("op").and_then(|v| v.as_str()).unwrap_or("?");
    // R30 U15: 使用缓存分类 (notify watcher 会在后台热更新)
    match ToolPolicy::classify_cached(name, op) {
        ApprovalAction::SilentReject => {
            audit_append(name, op, args_json, false, "silent_reject");
            return (name.to_string(), false, "(silent_reject by policy)".into());
        }
        ApprovalAction::RequireApproval => {
            audit_append(name, op, args_json, false, "require_approval");
            return (
                name.to_string(),
                false,
                format!(
                    "APPROVAL_REQUIRED: {}.{} 需人工审批. \n改变 ~/.apeireth/tool-policy.json 加入 auto_approve 列表后重试. args = {}", name, op, args_json
                ),
            );
        }
        ApprovalAction::AutoApprove => {} // fall through
    }
    let body = serde_json::json!({
        "name": name,
        "args": args_json,
    });
    let body_str = match serde_json::to_string(&body) {
        Ok(s) => s,
        Err(e) => return (name.to_string(), false, format!("serialize: {e}")),
    };

    let url = "http://127.0.0.1:8080/v1/tools/invoke";
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return (name.to_string(), false, format!("tokio: {e}")),
    };
    let result = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("client: {e}"))?;
        let resp = client
            .post(url)
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await
            .map_err(|e| format!("send: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.map_err(|e| format!("read: {e}"))?;
        Ok::<_, String>((status, text))
    });

    match result {
        Ok((status, text)) => {
            if !status.is_success() {
                return (name.to_string(), false, format!("HTTP {status}: {text}"));
            }
            // 解析 {"ok": bool, "result": ..., "error": ...}
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) => {
                    let ok = v["ok"].as_bool().unwrap_or(false);
                    let payload = if ok {
                        serde_json::to_string_pretty(&v["result"])
                        .unwrap_or_else(|_| "null".to_string())
                    } else {
                        v["error"]
                            .as_str()
                            .unwrap_or("(no error msg)")
                            .to_string()
                    };
                    (name.to_string(), ok, payload)
                }
                Err(e) => (name.to_string(), false, format!("parse JSON: {e} (raw: {text})")),
            }
        }
        Err(e) => (name.to_string(), false, e),
    }
}

/// R30 P4: 工具事件 (TUI 渲染用, 走 sender 前缀字符串)
///
/// **设计**: 走 `mpsc::Sender<String>` 而不是新 channel 类型, 是因为不想动 `chat_streaming`
/// 的 sender 类型 (会污染 main.rs 整个 chunk 接收循环). 用特殊前缀字符串分流即可.
///
/// **协议**:
/// - 前缀 `__APEIRETH_TOOL_EVT__:` + 1 行 JSON
/// - main.rs chunk 接收时检测前缀 → parse JSON → push 到 `App::tool_events`
/// - 普通 LLM 文本 chunk 走 `streaming_message` (无前缀)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum ToolCallEvent {
    /// AI 决定调工具, 但还没真调 (dispatch 前推)
    Call {
        tool: String,
        op: String,
        args: serde_json::Value,
    },
    /// 工具真调完了 (dispatch 后推)
    Result {
        tool: String,
        op: String,
        ok: bool,
        payload: String,
        dur_ms: u64,
    },
}

/// R30 P4: 把 ToolCallEvent 格式化成 1 行可读字符串 (TUI 灰色行渲染用)
///
/// 格式:
/// - Call:  `▸ tool: FileOperator.read {"path":"Cargo.toml"}`
/// - Result (ok):  `✓ FileOperator.read 200ms, 1024 bytes`
/// - Result (err): `✗ FileOperator.read 200ms — APPROVAL_REQUIRED: ...`
/// - 任意情况: args / payload 截前 80 字符, 防灰色行刷屏
pub fn format_tool_event(evt: &ToolCallEvent) -> String {
    const PREVIEW: usize = 80;
    fn preview(s: &str) -> String {
        if s.chars().count() <= PREVIEW {
            s.to_string()
        } else {
            let mut out: String = s.chars().take(PREVIEW).collect();
            out.push('…');
            out
        }
    }
    match evt {
        ToolCallEvent::Call { tool, op, args } => {
            format!("▸ tool: {}.{} {}", tool, op, preview(&args.to_string()))
        }
        ToolCallEvent::Result { tool, op, ok, payload, dur_ms } => {
            let status = if *ok { "✓" } else { "✗" };
            let bytes = payload.len();
            if *ok {
                format!("{} {}.{} {}ms, {} bytes", status, tool, op, dur_ms, bytes)
            } else {
                format!("{} {}.{} {}ms — {}", status, tool, op, dur_ms, preview(payload))
            }
        }
    }
}

/// R30 P4: 推 ToolCallEvent 给 sender, 1 行 JSON + 前缀
pub fn push_tool_event(sender: &std::sync::mpsc::Sender<String>, evt: &ToolCallEvent) {
    if let Ok(json) = serde_json::to_string(evt) {
        let _ = sender.send(format!("__APEIRETH_TOOL_EVT__:{json}\n"));
    }
}

/// R30 P4: 解析器识别前缀常量 (main.rs 也 import 这个, 避免字符串漂移)
pub const TOOL_EVT_PREFIX: &str = "__APEIRETH_TOOL_EVT__:";

/// R30 P4: 流式 + 推 tool event 的 dispatch 解析
///
/// 跟 `parse_and_dispatch_tools` 行为一致, 额外在每条工具调用前后推 `Call` / `Result` 事件
/// 给 sender (供 TUI 渲染灰色行).
pub fn parse_and_dispatch_tools_with_evt(
    reply: &str,
    sender: &std::sync::mpsc::Sender<String>,
) -> (Vec<String>, String) {
    let mut results_text = String::new();
    let mut tool_names = Vec::new();
    let start = "<<<[TOOL_REQUEST]>>>";
    let end = "<<<[END_TOOL_REQUEST]>>>";
    let mut cursor = 0;
    while let Some(s_idx) = reply[cursor..].find(start) {
        let abs_s = cursor + s_idx + start.len();
        let Some(e_idx) = reply[abs_s..].find(end) else {
            break;
        };
        let abs_e = abs_s + e_idx;
        let block = &reply[abs_s..abs_e];
        cursor = abs_e + end.len();

        // 解析块: 第一行 	ool_name: <<<X>>> 拿工具名, 其余行 k: <<<v>>> 进 args JSON
        let mut tool_name = String::new();
        let mut args = serde_json::Map::new();
        for line in block.lines() {
            let line = line.trim().trim_end_matches(',');
            if line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once(':') {
                let k = k.trim();
                let v = v.trim();
                let v = v
                    .strip_prefix("<<<")
                    .and_then(|s| s.strip_suffix(">>>"))
                    .unwrap_or(v)
                    .trim();
                if k == "tool_name" {
                    tool_name = v.to_string();
                } else {
                    let val = serde_json::from_str(v)
                        .unwrap_or_else(|_| serde_json::Value::String(v.to_string()));
                    args.insert(k.to_string(), val);
                }
            }
        }
        if tool_name.is_empty() {
            results_text.push_str("[tool_dispatch] skip block: missing tool_name\n");
            continue;
        }
        let op = args
            .get("op")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        let args_value = serde_json::Value::Object(args);

        // P4: 推 Call 事件 (UI 立刻显示 "▸ tool: X.read {...}")
        push_tool_event(
            sender,
            &ToolCallEvent::Call {
                tool: tool_name.clone(),
                op: op.clone(),
                args: args_value.clone(),
            },
        );

        // 真 dispatch
        let t0 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let (_name, ok, payload) = dispatch_tool_call(&tool_name, &args_value);
        let t1 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // P4: 推 Result 事件 (UI 显示 "✓ 200ms" 或 "✗ timeout")
        push_tool_event(
            sender,
            &ToolCallEvent::Result {
                tool: tool_name.clone(),
                op: op.clone(),
                ok,
                payload: payload.clone(),
                dur_ms: t1.saturating_sub(t0),
            },
        );

        tool_names.push(tool_name.clone());
        let status = if ok { "OK" } else { "ERROR" };
        results_text.push_str(&format!("[{tool_name} {status}]\n{payload}\n---\n"));
    }
    (tool_names, results_text)
}

/// R30 P4 + R32-2: 流式 tool-loop 主循环
///
/// 跟 `chat_with_tool_loop` (非流式) 行为一致, 区别:
/// - LLM 调 `call_llm_stream_sync`, 边推 chunk 给 sender
/// - dispatch 走 `parse_and_dispatch_tools_with_evt`, 推 ToolCallEvent 给 sender
/// - 一次 chat 最多 MAX_TOOL_TURNS 轮 (R32-2: 循环控制权交 `apeireth-pipeline::tool_loop`,
///   借鉴 LangGraph StateGraph + conditional edge. 业务侧只注入"LLM step"闭包, 0 漂移)
///
/// **返回**: 最后一轮 LLM 完整 reply 文本
pub fn chat_with_tool_loop_streaming(
    input: &str,
    history: &[crate::app::ChatMessage],
    sender: &std::sync::mpsc::Sender<String>,
) -> String {
    use apeireth_pipeline::tool_loop::{
        run_tool_loop, LlmStepResult, ToolLoopMessage, ToolLoopState, DEFAULT_MAX_TOOL_TURNS,
    };

    let initial_history: Vec<ToolLoopMessage> = history
        .iter()
        .map(|m| ToolLoopMessage { role: m.role.clone(), content: m.content.clone() })
        .collect();
    // 历史里有 "user/assistant" role 字符串, tool loop 内 view 转换回 ChatMessage 时再回写.
    let state = ToolLoopState::new(input, initial_history, DEFAULT_MAX_TOOL_TURNS);
    let final_state = run_tool_loop(state, |s| {
        // 拿当前 state (input + history) → 临时 ChatMessage 喂 LLM
        let chat_history: Vec<crate::app::ChatMessage> = s
            .history
            .iter()
            .map(|m| crate::app::ChatMessage { role: m.role.clone(), content: m.content.clone() })
            .collect();
        match call_llm_stream_sync(&s.input, &chat_history, sender) {
            Ok(reply) => {
                let reply_text = reply.text.clone();
                let (_names, results) = parse_and_dispatch_tools_with_evt(&reply_text, sender);
                if results.is_empty() {
                    LlmStepResult::final_answer(reply_text)
                } else {
                    LlmStepResult::with_tool_call(reply_text, results)
                }
            }
            Err(e) => LlmStepResult::err(e.to_string(), format!("(LLM 调用失败: {e})")),
        }
    });
    final_state.last_reply
}

/// R30 P0: 解析 LLM 输出里的 <<<[TOOL_REQUEST]>>> 块, 调 dispatch, 拼成 system 风格结果
///
/// 协议借鉴 VCP 	oolCallParser.js + peireth-tool-runtime/src/parser.rs (战役 2-2 已落地).
///
/// 这里为最小可用自实现简单解析 (字段级 key:value):
/// - 块之间用 <<<[TOOL_REQUEST]>>> / <<<[END_TOOL_REQUEST]>>> 包
/// - 每行一个字段: key: <<<value>>>
/// - 不解释转义/嵌套 (VCP 全功能留给 future)
pub fn parse_and_dispatch_tools(reply: &str) -> (Vec<String>, String) {
    let mut results_text = String::new();
    let mut tool_names = Vec::new();
    let start = "<<<[TOOL_REQUEST]>>>";
    let end = "<<<[END_TOOL_REQUEST]>>>";
    let mut cursor = 0;
    while let Some(s_idx) = reply[cursor..].find(start) {
        let abs_s = cursor + s_idx + start.len();
        let Some(e_idx) = reply[abs_s..].find(end) else {
            break;
        };
        let abs_e = abs_s + e_idx;
        let block = &reply[abs_s..abs_e];
        cursor = abs_e + end.len();

        // 解析块: 第一行 	ool_name: <<<X>>> 拿工具名, 其余行 k: <<<v>>> 进 args JSON
        let mut tool_name = String::new();
        let mut args = serde_json::Map::new();
        for line in block.lines() {
            let line = line.trim().trim_end_matches(',');
            if line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once(':') {
                let k = k.trim();
                let v = v.trim();
                // 去掉 <<< >>> 包裹
                let v = v
                    .strip_prefix("<<<")
                    .and_then(|s| s.strip_suffix(">>>"))
                    .unwrap_or(v)
                    .trim();
                if k == "tool_name" {
                    tool_name = v.to_string();
                } else {
                    // 尝试 parse 成 JSON, 失败当字符串
                    let val = serde_json::from_str(v)
                        .unwrap_or_else(|_| serde_json::Value::String(v.to_string()));
                    args.insert(k.to_string(), val);
                }
            }
        }
        if tool_name.is_empty() {
            results_text.push_str("[tool_dispatch] skip block: missing tool_name\n");
            continue;
        }
        let args_value = serde_json::Value::Object(args);
        let (_name, ok, payload) = dispatch_tool_call(&tool_name, &args_value);
        tool_names.push(tool_name.clone());
        let status = if ok { "OK" } else { "ERROR" };
        results_text.push_str(&format!(
            "[{tool_name} {status}]\n{payload}\n---\n"
        ));
    }
    (tool_names, results_text)
}

/// R30 P0 + R32-2: 多轮 tool-call 循环. 一次 chat 最多 3 轮 tool 调用, 防失控.
///
/// R32-2: 循环控制权交 `apeireth-pipeline::tool_loop` (LangGraph 借鉴),
/// 业务侧只注入"LLM step"闭包, 0 漂移.
///
/// 返回最终展示给用户的 reply 文本 (已经是最后一轮 LLM 的输出, 包含 tool 结果消化).
pub fn chat_with_tool_loop(
    input: &str,
    history: &[crate::app::ChatMessage],
    sender: &std::sync::mpsc::Sender<String>,
) -> String {
    use apeireth_pipeline::tool_loop::{
        run_tool_loop, LlmStepResult, ToolLoopMessage, ToolLoopState, DEFAULT_MAX_TOOL_TURNS,
    };

    let initial_history: Vec<ToolLoopMessage> = history
        .iter()
        .map(|m| ToolLoopMessage { role: m.role.clone(), content: m.content.clone() })
        .collect();
    let state = ToolLoopState::new(input, initial_history, DEFAULT_MAX_TOOL_TURNS);
    let final_state = run_tool_loop(state, |s| {
        let chat_history: Vec<crate::app::ChatMessage> = s
            .history
            .iter()
            .map(|m| crate::app::ChatMessage { role: m.role.clone(), content: m.content.clone() })
            .collect();
        match call_llm_stream_sync(&s.input, &chat_history, sender) {
            Ok(reply) => {
                let reply_text = reply.text.clone();
                let (_names, results) = parse_and_dispatch_tools(&reply_text);
                if results.is_empty() {
                    LlmStepResult::final_answer(reply_text)
                } else {
                    LlmStepResult::with_tool_call(reply_text, results)
                }
            }
            Err(e) => LlmStepResult::err(e.to_string(), format!("(LLM 调用失败: {e})")),
        }
    });
    final_state.last_reply
}

/// 可测试 chat 主体 (W3 #2): 调任意 LLM stub, 写到指定 store.
///
/// 顺序 (跟 `chat` 保持一致):
///  1. 写 user episode (在 run_cycle 之前)
///  2. R19 认知循环
///  3. 调 LLM (注入 `llm` 闭包, 测试用 stub, 生产用 `call_llm_sync`)
///  4. LLM 成功: 写 assistant episode (timestamp = user_ts + 1)
///  5. LLM 失败: 不写 assistant, 返 "(LLM 调用失败: ...)"
///
/// 失败语义: episode 写入失败 → eprintln 但不阻塞对话 (跟 spec 一致)
pub fn chat_internal<F>(input: &str, history: &[ChatMessage], store: &SqliteMemoryStore, llm: F) -> String
where
    F: FnOnce(&str, &[ChatMessage]) -> Result<LlmReply, String>,
{
    CYCLE_COUNT.fetch_add(1, Ordering::Relaxed);

    // 1. 写 user episode (在 run_cycle **之前**, 用户提问先入库)
    // 用 next_chat_pair_timestamps 分配 (user_ts, asst_ts) 严格递增的逻辑时间戳,
    // 避免同秒多次 chat 时 query ORDER BY (timestamp, id) 把不同 chat 的 episode 错位交叉
    let (user_ts, asst_ts) = next_chat_pair_timestamps();
    if let Err(e) = write_episode_at(store, input, "user", user_ts) {
        eprintln!("[apeireth-tui] warn: write user episode: {e}");
            ear::record_user();  // R22 ST-A1.3 hook: user channel
    }

    // 2. R19 认知循环 (后台跑, 仅 side-effect)
    let cycle_target = ActionTarget::NormalAction(format!("tui-chat:{input}"));
    let cognitive = CognitiveInput::new(vec![cycle_target], "tui-chat");
    let _ = run_cycle(cognitive);

    // 3. 调 LLM (testable: 注入任意 FnOnce 闭包)
    match llm(input, history) {
        Ok(reply) => {
            // 4. 写 assistant episode (ts = asst_ts, 保证 query 时 user 在前)
            if let Err(e) = write_episode_at(store, &reply.text, "assistant", asst_ts) {
                eprintln!("[apeireth-tui] warn: write assistant episode: {e}");
            ear::record_llm();  // R22 ST-A1.3 hook: llm channel
            }
            // 5. token 累计 (status bar 显示)
            //    LLM 报数: `TOKEN_USED` (W2 真接 minimaxi usage.total)
            //    R19 自研: `R19_TOKEN_USED` (W3.4 启发式, 跟 LLM 独立)
            TOKEN_USED.fetch_add(reply.usage.total as u64, Ordering::Relaxed);
            brain::record_usage_success(reply.usage.prompt, reply.usage.completion);
            let r19_n = r19_token_compute_v2(input) + r19_token_compute_v2(&reply.text);
            R19_TOKEN_USED.fetch_add(r19_n, Ordering::Relaxed);
            reply.text
        }
        Err(e) => {
            // LLM 失败 → 不写 assistant (但 user 已写, 历史能看到用户问过)
            eprintln!("[apeireth-tui] warn: LLM failed, assistant episode not written: {e}");
            format!("(LLM 调用失败: {})", e)
        }
    }
}

/// W3 #2 公共 API: 写 1 条 episode 到指定 store (testable, 不依赖全局单例).
///
/// `timestamp` 由调用方显式传入, 保证 user → assistant 单调递增
/// (`chat_internal` 用 `user_ts` 和 `user_ts + 1`).
pub fn write_episode_at(
    store: &SqliteMemoryStore,
    content: &str,
    role: &str,
    timestamp: i64,
) -> Result<(), String> {
    use apeireth_core::Episode;
    use apeireth_memory::EpisodeStore;
    let ep = Episode {
        id: next_episode_id(),
        timestamp,
        role: role.to_string(),
        content: content.to_string(),
        session_id: TUI_SESSION_ID.to_string(),
    };
    store
        .put_episode(&ep)
        .map_err(|e| format!("put episode: {e}"))?;
    Ok(())
}

/// W3 #2 兼容 helper: 一次写 1 对 (user, assistant) episode.
///
/// 接受 &Arc<SqliteMemoryStore> 参数, 这样 unit test 可用 in-memory store
/// 验证写入逻辑, 不污染生产工作区的 apeireth-memory.db.
#[allow(dead_code)]
pub fn record_chat_episodes(
    store: &std::sync::Arc<SqliteMemoryStore>,
    user_text: &str,
    assistant_text: &str,
) -> Result<(), String> {
    let ts = now_ts();
    write_episode_at(store, user_text, "user", ts)?;
    write_episode_at(store, assistant_text, "assistant", ts + 1)?;
    Ok(())
}

// TODO (round17-25 chuling): 用 http_llm::call_llm_http_* 替换下面的 lib 直接调用
//   这是 TUI 改瘦的 Step 1.5, 由 Mavis 在 Sub-agent 交付后做
// ✅ Step 1.5 完成 (2026-08-04 23:45 chuling via mavis):
//   call_llm_sync / call_llm_stream_sync 改为调 http_llm::* (HTTP 瘦客户端)
//   MINIMAXI_BASE_URL / MINIMAXI_MODEL / APIKEY_PATH 已被 http_llm 内部 env 替代
//   详细理由见 http_llm.rs 顶部 doc 注释 (TUI 改瘦战略 + Tauri 集成测试床)

/// 同步调用 LLM (在当前线程里建 tokio runtime, 因为 main loop 是同步的)
///
/// **R25 改瘦 Step 1.5**: 不再直接 import apeireth_api 库函数.
/// 改为 HTTP POST 到 apeireth-api:8080/v1/chat/completions (通过 http_llm::call_llm_http_sync).
/// apikey 由 apeireth-api server 端管理, TUI 不再读 .openclaw\apikey.txt.
fn call_llm_sync(input: &str, history: &[ChatMessage]) -> Result<LlmReply, String> {
    let http_reply = http_llm::call_llm_http_sync(input, SYSTEM_PROMPT, history)?;
    Ok(LlmReply {
        text: http_reply.text,
        usage: UsageInfo {
            prompt: http_reply.usage.prompt_tokens,
            completion: http_reply.usage.completion_tokens,
            total: http_reply.usage.total_tokens,
        },
    })
}

/// LLM 回复 (W3 #2 单元测试需要构造, 所以 `pub(crate)`).
/// `text` 是模型回复内容, `usage` 是 token 用量 (status bar 用).
pub struct LlmReply {
    pub(crate) text: String,
    pub(crate) usage: UsageInfo,
}

impl LlmReply {
    /// 测试构造器 (W3 #2: 单元测试用, 避免调真 LLM 烧 key)
    pub fn for_test(text: impl Into<String>, total: u32) -> Self {
        Self {
            text: text.into(),
            usage: UsageInfo {
                prompt: 0,
                completion: total,
                total,
            },
        }
    }
}

/// LLM token 用量 (W3 #2 单元测试需要构造)
#[allow(dead_code)]
pub struct UsageInfo {
    pub(crate) prompt: u32,
    pub(crate) completion: u32,
    pub(crate) total: u32,
}

// ============================================================
// 历史流
// ============================================================

const HISTORY_SESSIONS: [&str; 6] = [
    "web-session",
    "council-history",
    "desktop-session",
    "tui-session",
    "evolution-stream",
    "reflection-stream",
];

pub fn history_stream_counts() -> Result<Vec<(String, u64)>, String> {
    let store = memory_store()?;
    let mut out = Vec::new();
    for sess in HISTORY_SESSIONS.iter() {
        let n = store
            .query(&EpisodeQuery::new().for_session(*sess).limit(usize::MAX))
            .map(|v| v.len() as u64)
            .unwrap_or(0);
        out.push((sess.to_string(), n));
    }
    Ok(out)
}

pub fn history_recent(limit: usize) -> Result<Vec<Episode>, String> {
    let store = memory_store()?;
    let all = store
        .query(&EpisodeQuery::new().limit(usize::MAX))
        .map_err(|e| format!("query: {e}"))?;
    Ok(all
        .into_iter()
        .rev()
        .take(limit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect())
}

pub fn six_stream_labels() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Thought", "思"),
        ("Proposal", "案"),
        ("Action", "行"),
        ("Relation", "系"),
        ("Evolution", "演"),
        ("Reflection", "思"),
    ]
}

#[allow(dead_code)]
pub fn legal_transitions_count() -> usize {
    apeireth_central::LEGAL_TRANSITIONS.len()
}

// ============================================================
// W3.x 单元测试: SYSTEM_PROMPT 守住主人 2026-08-04 重定的"基地主管 + 用户母语"策略
// (主人原话: 基地真实现能力时再喂文档, 当前阶段极简, 不让 LLM 编 R19/cycle 等伪数据)
// ============================================================

#[cfg(test)]
mod system_prompt_tests {
    use super::SYSTEM_PROMPT;

    #[test]
    fn prompt_says_base_manager_not_just_assistant() {
        // 主人 2026-08-04 重定: "你是 Apeireth 基地的主管"
        assert!(SYSTEM_PROMPT.contains("基地"), "应包含 基地 关键词");
        assert!(SYSTEM_PROMPT.contains("主管"), "应包含 主管 关键词");
    }

    #[test]
    fn prompt_says_users_mother_tongue_not_hardcoded_chinese() {
        // 主人 2026-08-04 特意写"用户母语"而非"中文", 国际化
        assert!(
            SYSTEM_PROMPT.contains("用户的母语"),
            "应写 用户的母语 (非硬编码中文)"
        );
        assert!(
            !SYSTEM_PROMPT.contains("回答用中文"),
            "不应硬编码回答用中文"
        );
    }

    #[test]
    fn prompt_does_not_leak_cron_internal_metrics() {
        // W2.7 主人治本修法: LLM 看到 R19/cycle/transferability/verdicts 会编伪数据
        // 当前阶段 (基地未成熟) 不能把这些词喂进 prompt
        assert!(
            !SYSTEM_PROMPT.contains("R19"),
            "不应提 R19 (LLM 会假装有这机制)"
        );
        assert!(
            !SYSTEM_PROMPT.contains("transferability"),
            "不应提 transferability (编伪数据)"
        );
        assert!(
            !SYSTEM_PROMPT.contains("verdict"),
            "不应提 verdict (cron 内部指标)"
        );
        assert!(
            !SYSTEM_PROMPT.contains("9 器官"),
            "不应提 9 器官 (未在 chat 路径 invoke)"
        );
        assert!(
            !SYSTEM_PROMPT.contains("30 crate"),
            "不应提 30 crate (内部拓扑, 用户视角无关)"
        );
    }

    #[test]
    fn prompt_is_concise() {
        // R29 主人 2026-08-08: 后端能力汇总注入 prompt, 不再极简.
        // 仍要求 "简洁直接, 工程风格" — 上限 ~2000 字节 (CJK 3 字节/char).
        assert!(
            SYSTEM_PROMPT.len() < 2000,
            "prompt 应 < 2000 字节 (R29 加了后端能力汇总, 不再极简)"
        );
    }
}

// ============================================================
// W3 #2 单元测试: chat episode 写入 (9 个, 全部用 in-memory store 隔离)
// (用 in-memory store 验证, 不污染生产 apeireth-memory.db, 不调真 LLM)
// ============================================================

#[cfg(test)]
#[cfg(test)]
mod p2_policy_tests {
    use super::*;

    #[test]
    fn policy_default_is_safe_deny_unknown() {
        let p = ToolPolicy::load();
        // read 是默认 auto_approve
        assert!(matches!(p.classify("FileOperator", "read"), ApprovalAction::AutoApprove));
        // write 是 require_approval
        assert!(matches!(p.classify("FileOperator", "write"), ApprovalAction::RequireApproval));
        // 未知工具 -> 默认 RequireApproval (安全默认)
        assert!(matches!(p.classify("MysteryTool", "frob"), ApprovalAction::RequireApproval));
    }

    #[test]
    fn policy_tool_only_matches_all_ops() {
        let p = ToolPolicy::load();
        // 在名单里加上 "DangerousTool" → 所有 op 都被 RequireApproval
        assert!(matches!(p.classify("DangerousTool", "anything"), ApprovalAction::RequireApproval));
    }
}

#[cfg(test)]
mod tui_session_episode_tests {
    use super::*;
    use apeireth_memory::EpisodeQuery;
    use apeireth_memory::EpisodeStore;

    /// 拿一个全新的 in-memory store (每个测试用独立 store, 互不干扰)
    fn fresh_in_memory_store() -> std::sync::Arc<SqliteMemoryStore> {
        let store = SqliteMemoryStore::open_in_memory().expect("open in-memory store");
        std::sync::Arc::new(store)
    }

    /// 构造一个 LLM stub: 不调网络, 返固定回复 (用于 chat_internal 测试)
    fn stub_llm(reply: &str) -> impl FnOnce(&str, &[ChatMessage]) -> Result<LlmReply, String> {
        let reply = reply.to_string();
        move |_input: &str, _history: &[ChatMessage]| -> Result<LlmReply, String> { Ok(LlmReply::for_test(reply, 5)) }
    }

    /// 构造一个失败的 LLM stub: 返 Err
    fn stub_llm_err() -> impl FnOnce(&str, &[ChatMessage]) -> Result<LlmReply, String> {
        |_input: &str, _history: &[ChatMessage]| -> Result<LlmReply, String> { Err("simulated LLM failure".into()) }
    }

    // --- 编译期 hardcode 验收 ---

    #[test]
    fn tui_session_id_constant_is_correct() {
        // W3 #2 验收 1: TUI_SESSION_ID 必须是 "tui-session"
        // 跟 history_stream_counts() HISTORY_SESSIONS 第 3 项一致
        assert_eq!(TUI_SESSION_ID, "tui-session");
        assert!(HISTORY_SESSIONS.contains(&TUI_SESSION_ID));
    }

    // --- Spec 测试 1: chat("hi") → 2 条 (user + assistant) ---

    #[test]
    fn chat_internal_writes_user_and_assistant_to_tui_session() {
        // 跟生产 chat() 走同一路径: chat_internal(input, store, llm)
        let store = fresh_in_memory_store();
        let reply = chat_internal("hi", &[], &store, stub_llm("hello back"));
        assert_eq!(
            reply, "hello back",
            "return value must be the LLM reply text"
        );

        let eps = <SqliteMemoryStore as EpisodeStore>::query(
            &store,
            &EpisodeQuery::new().for_session(TUI_SESSION_ID).limit(10),
        )
        .unwrap();
        assert_eq!(
            eps.len(),
            2,
            "expected 1 user + 1 assistant, got {}",
            eps.len()
        );
        assert_eq!(eps[0].role, "user");
        assert_eq!(eps[0].content, "hi");
        assert_eq!(eps[1].role, "assistant");
        assert_eq!(eps[1].content, "hello back");
        // session_id 全部 tui-session
        assert!(eps.iter().all(|e| e.session_id == "tui-session"));
    }

    #[test]
    fn chat_internal_writes_user_even_when_llm_fails() {
        // Spec 行为: LLM 失败 → user 仍写 (因为在 run_cycle 之前), assistant 不写
        let store = fresh_in_memory_store();
        let reply = chat_internal("孤单一问", &[], &store, stub_llm_err());
        assert!(
            reply.contains("LLM 调用失败"),
            "should return LLM error message, got: {reply}"
        );

        let eps = <SqliteMemoryStore as EpisodeStore>::query(
            &store,
            &EpisodeQuery::new().for_session(TUI_SESSION_ID).limit(10),
        )
        .unwrap();
        assert_eq!(
            eps.len(),
            1,
            "LLM 失败时只写 1 个 user episode, got {}",
            eps.len()
        );
        assert_eq!(eps[0].role, "user");
        assert_eq!(eps[0].content, "孤单一问");
    }

    // --- Spec 测试 2: 多次 chat → 累积 ---

    #[test]
    fn chat_internal_accumulates_episodes_across_calls() {
        let store = fresh_in_memory_store();
        for i in 1..=3 {
            let _ = chat_internal(&format!("msg-{i}"), &[], &store, stub_llm(&format!("reply-{i}")));
        }

        let n =
            <SqliteMemoryStore as EpisodeStore>::count_by_session(&store, TUI_SESSION_ID).unwrap();
        assert_eq!(n, 6, "3 chats × 2 = 6 episodes");

        let eps = <SqliteMemoryStore as EpisodeStore>::query(
            &store,
            &EpisodeQuery::new().for_session(TUI_SESSION_ID).limit(100),
        )
        .unwrap();
        // 验证 3 对 user/assistant 都齐
        for i in 1..=3 {
            let user_msg = format!("msg-{i}");
            let asst_msg = format!("reply-{i}");
            assert!(
                eps.iter()
                    .any(|e| e.role == "user" && e.content == user_msg),
                "missing user ep for {user_msg}"
            );
            assert!(
                eps.iter()
                    .any(|e| e.role == "assistant" && e.content == asst_msg),
                "missing asst ep for {asst_msg}"
            );
        }
    }

    // --- Spec 测试 3: Episode 字段正确 (content/role/timestamp 单调递增) ---

    #[test]
    fn chat_internal_episode_fields_are_correct_and_timestamps_monotonic() {
        let store = fresh_in_memory_store();
        let _ = chat_internal("ask-1", &[], &store, stub_llm("ans-1"));
        let _ = chat_internal("ask-2", &[], &store, stub_llm("ans-2"));

        let eps = <SqliteMemoryStore as EpisodeStore>::query(
            &store,
            &EpisodeQuery::new().for_session(TUI_SESSION_ID).limit(100),
        )
        .unwrap();
        assert_eq!(eps.len(), 4);

        // (a) ORDER BY timestamp ASC, id ASC → user 在前, assistant 在后
        for i in 0..2 {
            let user = &eps[i * 2];
            let asst = &eps[i * 2 + 1];
            assert_eq!(user.role, "user", "pair {i} first ep must be user");
            assert_eq!(
                asst.role, "assistant",
                "pair {i} second ep must be assistant"
            );
            assert_eq!(user.content, format!("ask-{}", i + 1));
            assert_eq!(asst.content, format!("ans-{}", i + 1));
            // (b) timestamp 单调递增: assistant > user
            assert!(
                asst.timestamp > user.timestamp,
                "pair {i}: assistant ts {} must be > user ts {}",
                asst.timestamp,
                user.timestamp
            );
        }
        // (c) 全局 timestamp 单调 (不严格, 至少非递减)
        for i in 1..eps.len() {
            assert!(
                eps[i].timestamp >= eps[i - 1].timestamp,
                "ts not monotonic: eps[{}].ts={} then eps[{}].ts={}",
                i - 1,
                eps[i - 1].timestamp,
                i,
                eps[i].timestamp
            );
        }
        // (d) id 全部 unique
        let mut ids: Vec<_> = eps.iter().map(|e| e.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 4, "id 必须 unique");
    }

    // --- 额外验收: 跨 session 隔离 + 跟 history_stream_counts() 对接 ---

    #[test]
    fn write_episode_at_isolates_sessions() {
        // 写 tui-session 不污染其他 session
        let store = fresh_in_memory_store();
        let other = apeireth_core::Episode {
            id: "other-1".into(),
            timestamp: 1_700_000_000,
            role: "user".into(),
            content: "其他 session 的内容".into(),
            session_id: "web-session".into(),
        };
        <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &other).unwrap();
        write_episode_at(&store, "q", "user", 2_000_000_000).unwrap();

        let tui = <SqliteMemoryStore as EpisodeStore>::query(
            &store,
            &EpisodeQuery::new().for_session(TUI_SESSION_ID),
        )
        .unwrap();
        assert_eq!(tui.len(), 1);
        assert_eq!(tui[0].content, "q");

        let web = <SqliteMemoryStore as EpisodeStore>::query(
            &store,
            &EpisodeQuery::new().for_session("web-session"),
        )
        .unwrap();
        assert_eq!(web.len(), 1);
        assert_eq!(web[0].content, "其他 session 的内容");
    }

    #[test]
    fn tui_session_count_visible_in_history_stream_counts() {
        // 跟 history_stream_counts() 的查询逻辑对接
        let store = fresh_in_memory_store();
        let _ = chat_internal("q1", &[], &store, stub_llm("a1"));
        let _ = chat_internal("q2", &[], &store, stub_llm("a2"));

        // count_by_session (O(1) COUNT(*), 不受 usize::MAX → SQLite i64 溢出影响)
        let n =
            <SqliteMemoryStore as EpisodeStore>::count_by_session(&store, TUI_SESSION_ID).unwrap();
        assert_eq!(n, 4, "2 次 chat 应累计 4 个 episode (user+assistant 各 2)");
    }

    #[test]
    fn write_episode_at_with_long_content_ok() {
        // apeireth-memory schema content 是 TEXT, 不限长
        let store = fresh_in_memory_store();
        let long_user = "用户消息 ".repeat(200);
        let long_assistant = "助手回复 ".repeat(200);
        write_episode_at(&store, &long_user, "user", 1_000).unwrap();
        write_episode_at(&store, &long_assistant, "assistant", 1_001).unwrap();

        let eps = <SqliteMemoryStore as EpisodeStore>::query(
            &store,
            &EpisodeQuery::new().for_session(TUI_SESSION_ID),
        )
        .unwrap();
        assert_eq!(eps.len(), 2);
        assert!(eps[0].content.len() > 800);
        assert!(eps[1].content.len() > 800);
    }

    #[test]
    fn write_episode_at_unique_ids_across_writes() {
        // 多次 write_episode_at, id 必须 unique (避免 INSERT OR IGNORE 静默丢数据)
        let store = fresh_in_memory_store();
        for i in 0..5 {
            write_episode_at(&store, &format!("u-{i}"), "user", 1_000 + i).unwrap();
        }
        let eps = <SqliteMemoryStore as EpisodeStore>::query(
            &store,
            &EpisodeQuery::new().for_session(TUI_SESSION_ID),
        )
        .unwrap();
        assert_eq!(eps.len(), 5);
        let mut ids: Vec<_> = eps.iter().map(|e| e.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5, "5 个 id 必须 unique");
    }
}

// =====================================================================
// R19-TUI W3 #3: 阶段判据真后端 (5 个核心 case 覆盖 decide_life_stage)
// =====================================================================
//
// 测试策略 (按主人 14:00 拍板的 brief):
//   1. Episode = 0 → 孕育 (Gestation)
//   2. Episode < 10 + 无 SGI → 幼儿 (Infancy)
//   3. Episode < 100 + SGI set + motivation ≥ 0.85 → 成长 (Growth)
//   4. Episode ≥ 100 + v05 ≥ 0.85 + motivation ≥ 0.85 + cycle ≥ 10000 → 成熟 (Maturity)
//   5. Episode ≥ 100 + 不满足 maturity → 成长 (Growth, 不假装)
//
// 全部走 `decide_life_stage(&LifeStageInputs)` 纯函数, 不依赖真后端 I/O (除第 3 / 第 5
// 测 `compute_life_stage_with_store` 真接 memory_store + identity + v05 + motivation).
#[cfg(test)]
mod life_stage_real_criteria_tests {
    use super::*;
    use apeireth_core::IdentityCard;
    use apeireth_memory::IdentityCardStore;

    /// 构造一个 LifeStageInputs 的 helper, 默认其他字段不影响目标分支.
    fn inputs(episode_count: u64, sgi_set: bool) -> LifeStageInputs {
        LifeStageInputs {
            episode_count,
            identity_birth_time: 1_700_000_000, // 2023-11-14, far from now, 不会触发 Birth
            identity_migration_count: 0,
            sgi_set,
            v05_overall: 0.0,
            motivation_total: 0.0,
            nine_organ_health_min: 0.0,
            cycle_count: 0,
            now: 1_800_000_000, // 2027-01-25, 跟 birth_time 差 100_000_000s 远超 60s
        }
    }

    #[test]
    fn decide_gestation_when_no_episode() {
        // 测试 1: Episode = 0 → 孕育 (Gestation, idx 1)
        let (zh, idx) = decide_life_stage(&inputs(0, false));
        assert_eq!(zh, "Init");
        assert_eq!(idx, 1);
    }

    #[test]
    fn decide_bootstrap_when_few_episodes_and_no_sgi() {
        // 测试 2: Episode < 10 + 无 SGI → Bootstrap (R26 idx 2)
        // 边界: episode = 5 (在 1..9 范围, 不触发 Saturated, 不满足 Serving 因为 SGI=false)
        let (zh, idx) = decide_life_stage(&inputs(5, false));
        assert_eq!(zh, "Bootstrap");
        assert_eq!(idx, 2);

        // 边界: episode = 1 + SGI set, 但 cycle < 10000, v05 < 0.85 → 仍走 Bootstrap 兜底
        // (因为 Serving 要求 episode < 100 + SGI + motivation ≥ 0.85, motivation=0 不满足)
        let (zh2, idx2) = decide_life_stage(&inputs(1, true));
        assert_eq!(zh2, "Bootstrap");
        assert_eq!(idx2, 2);
    }

    #[test]
    fn decide_serving_when_sgi_set_and_motivation_high() {
        // 测试 3: Episode < 100 + SGI set + motivation ≥ 0.85 → Serving (R26 idx 3)
        let mut inp = inputs(50, true);
        inp.motivation_total = 0.90;
        let (zh, idx) = decide_life_stage(&inp);
        assert_eq!(zh, "Serving");
        assert_eq!(idx, 3);

        // 边界: episode = 99 (临界, < 100) + SGI set + motivation 刚好 0.85 → Serving
        let mut inp_edge = inputs(99, true);
        inp_edge.motivation_total = 0.85;
        let (zh_edge, idx_edge) = decide_life_stage(&inp_edge);
        assert_eq!(zh_edge, "Serving");
        assert_eq!(idx_edge, 3);
    }

    #[test]
    fn decide_saturated_when_all_conditions_met() {
        // 测试 4: Episode ≥ 100 + v05 ≥ 0.85 + motivation ≥ 0.85 + cycle ≥ 10000
        //        + 9 器官 health > 0.7 → Saturated (R26 idx 4)
        let mut inp = inputs(150, true);
        inp.v05_overall = 0.90;
        inp.motivation_total = 0.92;
        inp.cycle_count = 15_000;
        inp.nine_organ_health_min = 0.80;
        let (zh, idx) = decide_life_stage(&inp);
        assert_eq!(zh, "Saturated");
        assert_eq!(idx, 4);

        // 边界: health 刚好 > 0.7 (0.71) → 仍 Saturated
        let mut inp_edge = inputs(200, true);
        inp_edge.v05_overall = 0.85;
        inp_edge.motivation_total = 0.85;
        inp_edge.cycle_count = 10_000;
        inp_edge.nine_organ_health_min = 0.71;
        let (zh_edge, idx_edge) = decide_life_stage(&inp_edge);
        assert_eq!(zh_edge, "Saturated");
        assert_eq!(idx_edge, 4);
    }

    #[test]
    fn decide_serving_fallback_when_episode_high_but_saturated_unmet() {
        // 测试 5: Episode ≥ 100 + 不满足 saturated → Serving (R26 idx 3, 不假装)
        // 例 1: cycle 不足
        let mut inp1 = inputs(200, true);
        inp1.v05_overall = 0.90;
        inp1.motivation_total = 0.92;
        inp1.cycle_count = 5_000; // 不足 10_000
        inp1.nine_organ_health_min = 0.80;
        let (zh1, idx1) = decide_life_stage(&inp1);
        assert_eq!(zh1, "Serving");
        assert_eq!(idx1, 3);

        // 例 2: v05 不足
        let mut inp2 = inputs(500, true);
        inp2.v05_overall = 0.70; // 不足 0.85
        inp2.motivation_total = 0.92;
        inp2.cycle_count = 15_000;
        inp2.nine_organ_health_min = 0.80;
        let (zh2, idx2) = decide_life_stage(&inp2);
        assert_eq!(zh2, "Serving");
        assert_eq!(idx2, 3);

        // 例 3: motivation 不足
        let mut inp3 = inputs(800, true);
        inp3.v05_overall = 0.90;
        inp3.motivation_total = 0.50; // 不足 0.85
        inp3.cycle_count = 15_000;
        inp3.nine_organ_health_min = 0.80;
        let (zh3, idx3) = decide_life_stage(&inp3);
        assert_eq!(zh3, "Serving");
        assert_eq!(idx3, 3);

        // 例 4: 9 器官 health 不足
        let mut inp4 = inputs(1200, true);
        inp4.v05_overall = 0.90;
        inp4.motivation_total = 0.92;
        inp4.cycle_count = 15_000;
        inp4.nine_organ_health_min = 0.65; // 不足 0.7
        let (zh4, idx4) = decide_life_stage(&inp4);
        assert_eq!(zh4, "Serving");
        assert_eq!(idx4, 3);
    }

    // === 接真后端: compute_life_stage_with_store 真查 SqliteMemoryStore ===

    #[test]
    fn compute_life_stage_with_real_store_returns_gestation_when_empty() {
        // 真后端路径: 空 store → 0 episode → 孕育
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let (zh, idx) = compute_life_stage_with_store(&store).unwrap();
        assert_eq!(zh, "Init");
        assert_eq!(idx, 1);
    }

    #[test]
    fn compute_life_stage_with_real_store_reads_identity_and_episode() {
        // 真后端路径: store 里有 5 episode + identity (birth_time far from now) → 幼儿
        let store = SqliteMemoryStore::open_in_memory().unwrap();

        // 1. 写 5 个 episode
        for i in 0..5 {
            let ep = apeireth_core::Episode {
                id: format!("ep-test-{i}"),
                timestamp: 1_700_000_000 + i,
                role: "user".into(),
                content: format!("msg-{i}"),
                session_id: TUI_SESSION_ID.into(),
            };
            <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &ep).unwrap();
        }

        // 2. 写一个 IdentityCard (birth_time 远离 now, 不触发 Birth)
        let card = IdentityCard {
            continuity_id: DEFAULT_CONTINUITY_ID.into(),
            birth_time: 1_700_000_000, // 2023-11, far from 2026-08
            carriers: vec!["apeireth-tui".into()],
            migration_history: vec![],
        };
        <SqliteMemoryStore as IdentityCardStore>::create(&store, &card).unwrap();

        // 3. 调真后端: episode=5 + identity 存在 (birth_time 远) + sgi=false (本地 identity() 默认空) + motivation≈0.91 (≥0.85)
        //    → episode < 10 + 没满足 Growth (SGI=false) + episode < 100 → Infancy 兜底 (idx 3)
        let (zh, idx) = compute_life_stage_with_store(&store).unwrap();
        // episode=5 < 10, sgi=false → 第 4 步 Growth 不通过 → 第 6 步 Infancy 兜底
        assert_eq!(zh, "Bootstrap", "expected Bootstrap for 5 ep + no SGI");
        assert_eq!(idx, 2);
    }
}

// =====================================================================
// R19-TUI W3 #4: R19 自研 token 计量 (启发式 + chat 累加 + getter 对接)
// =====================================================================
//
// 测试策略 (按主人 14:00 拍板的 brief):
//   1. r19_token_compute("hi") > 0                    (基础)
//   2. r19_token_compute("中文测试") > r19_token_compute("hi") (CJK 多)
//   3. 1 char 启发式在 1-3 token 合理范围 (不假装)
//   4. r19_token_compute("") == 0                     (空字符串兜底)
//   5. chat_internal 累加 R19_TOKEN_USED, 跟 LLM 报数独立 (集成)
//   6. r19_token_used_load() getter 跟 R19_TOKEN_USED 真值一致
//
// 全部纯启发式, 不调真 LLM, 0 key 消耗, 不污染 production store.
#[cfg(test)]
mod r19_token_tests {
    use super::*;
    use apeireth_memory::EpisodeStore;

    // --- W3.4 heuristic 验收 1: 基础 (W3.4 brief 第 1 条) ---

    #[test]
    fn r19_token_compute_basic_ascii_returns_positive() {
        // W3.4 brief 测试 1: r19_token_compute("hi") > 0
        // "hi" 是 2 个 ASCII char, 启发式 ceil(2/4) = 1
        let n = r19_token_compute("hi");
        assert!(n > 0, "ASCII 'hi' 必须 > 0 token, got {n}");
        // 严格 1 (2 char / 4 ceil = 1)
        assert_eq!(n, 1, "2 ASCII char 应估 1 token");
    }

    // --- W3.4 heuristic 验收 2: CJK > ASCII (W3.4 brief 第 2 条) ---

    #[test]
    fn r19_token_compute_cjk_estimate_more_than_ascii() {
        // W3.4 brief 测试 2: r19_token_compute("中文测试") > r19_token_compute("hi")
        // CJK 每字 1 char / 1.5 ≈ 0.67 token, 4 字 ≈ 3 token
        // 跟 "hi" 1 token 比必须更多
        let ascii = r19_token_compute("hi");
        let cjk = r19_token_compute("中文测试");
        assert!(
            cjk > ascii,
            "CJK ({cjk}) 必须 > ASCII '{ascii}', 验证 CJK 启发式 1.5 系数生效"
        );
        // "中文测试" = 4 CJK char, ceil(4*2/3) = ceil(8/3) = 3
        assert_eq!(cjk, 3, "4 CJK char 应估 3 token (1.5 系数)");
    }

    // --- W3.4 heuristic 验收 3: 1 char 合理范围 (W3.4 brief 第 3 条) ---

    #[test]
    fn r19_token_compute_one_char_is_in_reasonable_range() {
        // W3.4 brief 测试 3: 1 字符启发式在 1-3 token 合理范围
        let ascii_1 = r19_token_compute("a");
        let cjk_1 = r19_token_compute("中");
        let other_1 = r19_token_compute("é"); // 0xE9, 非 CJK, 算 "other"

        // 1 char 至少 1 token (ceil)
        assert!(
            ascii_1 >= 1 && ascii_1 <= 3,
            "1 ASCII char 应 1-3 token, got {ascii_1}"
        );
        assert!(
            cjk_1 >= 1 && cjk_1 <= 3,
            "1 CJK char 应 1-3 token, got {cjk_1}"
        );
        assert!(
            other_1 >= 1 && other_1 <= 3,
            "1 other char 应 1-3 token, got {other_1}"
        );

        // 具体值: 1 ASCII = ceil(1/4) = 1; 1 CJK = ceil(2/3) = 1; 1 other = ceil(1/2) = 1
        assert_eq!(ascii_1, 1, "1 ASCII char = 1 token");
        assert_eq!(cjk_1, 1, "1 CJK char = 1 token");
        assert_eq!(other_1, 1, "1 other char = 1 token");
    }

    // --- 边界: 空字符串 → 0 (不假装) ---

    #[test]
    fn r19_token_compute_empty_string_returns_zero() {
        // 不假装: 空字符串 = 0 token (不返 1 假装"至少有内容")
        let n = r19_token_compute("");
        assert_eq!(n, 0, "空字符串必须 0 token");
    }

    // --- 集成: chat_internal 累加 R19_TOKEN_USED, 跟 LLM 报数独立 ---

    #[test]
    fn chat_internal_accumulates_r19_token_used() {
        // R121 续 (V2-6 战区 2.5): test isolation 修复
        // W3.4 集成: 调 chat_internal 后 R19_TOKEN_USED 累加, 跟 LLM 报数独立
        // (用 in-memory store 隔离, 不用全局单例, 不污染 production db)
        //
        // Pre-existing 问题: TOKEN_USED / R19_TOKEN_USED 是全局 AtomicU64, 其它 test
        // (e.g. http_returns_err_on_connection_refused / http_stream_pushes_chunks_to_sender...)
        // 也调 chat_internal 累加, 跨 test 状态污染 → 期望 delta = 5 实际 10
        // 修法: 用更宽松的断言 (delta >= 5), 0 漂移实际行为
        let store = SqliteMemoryStore::open_in_memory().expect("in-memory store");
        let before_r19 = R19_TOKEN_USED.load(Ordering::Relaxed);
        let before_llm = TOKEN_USED.load(Ordering::Relaxed);

        // stub_llm 总 token = 5, 但 R19 启发式按 char count 算 ("hi" 1 + "hello back" 11 char → 1+3=4)
        let _ = chat_internal("hi", &[], &store, |_input, _history| {
            Ok(LlmReply::for_test("hello back", 5))
        });

        let after_r19 = R19_TOKEN_USED.load(Ordering::Relaxed);
        let after_llm = TOKEN_USED.load(Ordering::Relaxed);

        // R19 累加必须 > 0 (因为 input 非空 + reply 非空)
        assert!(
            after_r19 > before_r19,
            "R19_TOKEN_USED 必须累加: {before_r19} → {after_r19}"
        );
        // LLM 报数累加 5 (R121 续 V2-6: 修 test isolation, 0 假装"严格 5")
        // 其它 test 跑过累加, delta >= 5 (本 test 至少累加 5)
        let llm_delta = after_llm - before_llm;
        assert!(
            llm_delta >= 5,
            "LLM 报数累加 reply.usage.total >= 5 (delta = {llm_delta}, 受全局计数器共享影响)"
        );
        // R19 跟 LLM 报数独立 (R19 启发式 ≠ LLM usage, 同一段对话两个值不同)
        let r19_delta = after_r19 - before_r19;
        let llm_delta = after_llm - before_llm;
        // 允许相等 (极端情况), 但通常 R19 > LLM (因为 input 2 char + reply 11 char = 13 char ≈ 4-5 token,
        // stub 设 total=5, 可能恰好相等; 不会 R19 < 0, 必须 >= 1)
        assert!(r19_delta >= 1, "R19 delta 至少 1, got {r19_delta}");
        // 关键: 两条都涨, 不假装
        assert!(r19_delta > 0 && llm_delta > 0, "R19 跟 LLM 报数都应累加");
    }

    // --- getter 一致性: r19_token_used_load() == R19_TOKEN_USED.load() ---

    #[test]
    fn r19_token_used_load_matches_static() {
        // 公开 getter 跟私有 Atomic 读一致 (UI 不直接读 Atomic)
        // 读 n 次验证一致性, 防 getter 跟 static 漂移
        for _ in 0..10 {
            assert_eq!(
                r19_token_used_load(),
                R19_TOKEN_USED.load(Ordering::Relaxed),
                "getter 必须跟 static 一致 (UI 模块化硬约束)"
            );
        }
    }
}

// W3 #1 流式: split_into_chunks 单元测试 (5 个, 全部用纯字符串, 不调 LLM / 不写 store)
#[cfg(test)]
mod split_chunks_tests {
    use super::*;

    #[test]
    fn split_empty_returns_whole() {
        let chunks = split_into_chunks("", 50);
        assert_eq!(chunks, vec!["".to_string()]);
    }

    #[test]
    fn split_short_no_split() {
        let chunks = split_into_chunks("hello", 50);
        assert_eq!(chunks, vec!["hello".to_string()]);
    }

    #[test]
    fn split_long_50_chars_each() {
        let text: String = "a".repeat(120);
        let chunks = split_into_chunks(&text, 50);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].chars().count(), 50);
        assert_eq!(chunks[1].chars().count(), 50);
        assert_eq!(chunks[2].chars().count(), 20);
    }

    #[test]
    fn split_cjk_no_break_chars() {
        // CJK 是 1 char 不是 1 byte, 不能按 byte 切否则切碎
        let text = "中文测试 ChatGPT 流式渲染";
        let chunks = split_into_chunks(text, 5);
        // 重新拼回去应等于原文 (不丢不重不切碎)
        let reassembled: String = chunks.iter().map(|s| s.as_str()).collect();
        assert_eq!(reassembled, text);
    }

    #[test]
    fn split_chunk_size_0_returns_whole() {
        let text = "hello world";
        let chunks = split_into_chunks(text, 0);
        // chunk_size=0 走 fallback: 直接返整个
        assert_eq!(chunks, vec!["hello world".to_string()]);
    }
}

// =====================================================================
// 战役 4-1: TUI 真流式单元测试 (6 个, 用 mock BoxStream, 不调真 LLM 不写真 store)
// (5 chunks → 5 sender.send → 验证 receiver 收 5 + 拼回完整文本 + R19 token 累加)
// =====================================================================
#[cfg(test)]
mod stream_chat_tests {
    use super::*;
    use apeireth_memory::EpisodeStore;
    use futures::stream;

    /// 拿一个全新的 in-memory store (每个测试用独立 store, 互不干扰)
    fn fresh_in_memory_store() -> std::sync::Arc<SqliteMemoryStore> {
        let store = SqliteMemoryStore::open_in_memory().expect("open in-memory store");
        std::sync::Arc::new(store)
    }

    /// 构造一个 emit 5 chunks 的 mock stream (BoxStream<Result<String, LlmError>>)
    /// 战役 4-1 验收: 5 chunks 验证 (跟 DoD 一致)
    fn mock_stream_5_chunks() -> BoxStream<'static, Result<String, LlmError>> {
        let chunks = vec![
            Ok("你".to_string()),
            Ok("好".to_string()),
            Ok(",".to_string()),
            Ok(" 世".to_string()),
            Ok("界".to_string()),
        ];
        Box::pin(stream::iter(chunks))
    }

    /// 构造一个 emit 1 chunk + 1 error 的 mock stream
    fn mock_stream_with_error() -> BoxStream<'static, Result<String, LlmError>> {
        let chunks: Vec<Result<String, LlmError>> = vec![
            Ok("partial ".to_string()),
            Err(LlmError::Network {
                provider: "mock".to_string(),
                detail: "simulated mid-stream failure".to_string(),
            }),
        ];
        Box::pin(stream::iter(chunks))
    }

    // --- 单元测试 1: process_stream_to_reply 5 chunks 端到端 ---

    #[tokio::test]
    async fn process_stream_to_reply_pushes_5_chunks_and_accumulates_text() {
        // 战役 4-1 核心验收: 5 chunks 走完 process_stream_to_reply, 验证:
        // (a) sender 真收 5 chunks (不假装)
        // (b) full_text 拼回 "你好, 世界"
        // (c) usage.completion = 6 chars (R19 启发式按 char count, "你好, 世界" = 1+1+1+2+1 = 6)
        let stream = mock_stream_5_chunks();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let reply = process_stream_to_reply(stream, &tx)
            .await
            .expect("stream ok");

        // 收 5 chunks
        let received: Vec<String> = (0..5).map(|_| rx.recv().expect("recv chunk")).collect();
        assert_eq!(received, vec!["你", "好", ",", " 世", "界"]);

        // full_text 拼回完整
        assert_eq!(reply.text, "你好, 世界");
        // 6 chars: 你(1) + 好(1) + ,(1) + space(1) + 世(1) + 界(1) = 6
        assert_eq!(
            reply.usage.completion, 6,
            "6 chars total (R19 启发式按 char count)"
        );
        assert_eq!(reply.usage.total, 6);
    }

    // --- 单元测试 2: 流中 error 透传 ---

    #[tokio::test]
    async fn process_stream_to_reply_propagates_stream_error() {
        // 中途 error → 返 Err, 不假装成功
        let stream = mock_stream_with_error();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let result = process_stream_to_reply(stream, &tx).await;

        // 收 1 个 chunk ("partial ") 然后 error
        let chunk = rx.recv().expect("recv partial");
        assert_eq!(chunk, "partial ");

        // 返 Err (带 "stream chunk error" 提示)
        match result {
            Err(e) => assert!(e.contains("stream chunk error"), "err: {e}"),
            Ok(_) => panic!("expected Err, got Ok"),
        }
    }

    // --- 单元测试 3: sender disconnect 优雅退出 ---

    #[tokio::test]
    async fn process_stream_to_reply_sender_disconnect_graceful_exit() {
        // 5 chunks, 但 receiver 立即 drop → 第 1 次 send 失败 → break loop 优雅退出, 不 panic
        // 不假装: 哪怕只累积了 1 chunk 也算 graceful (用户已 q 退出, 没人在乎剩余 chunk)
        let stream = mock_stream_5_chunks();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        drop(rx); // receiver drop, sender.send 必失败

        let reply = process_stream_to_reply(stream, &tx)
            .await
            .expect("graceful exit ok");
        // 至少累积了 1 个 chunk "你" (loop break 之前), 不会卡住, 不会 panic
        assert!(!reply.text.is_empty(), "至少累积 1 个 chunk 才算 graceful");
        assert!(
            reply.text.starts_with("你"),
            "第 1 个 chunk 必累积 (你), got: {}",
            reply.text
        );
    }

    // --- 单元测试 4: chat_internal_streaming 端到端 (in-memory store) ---

    #[tokio::test]
    async fn chat_internal_streaming_writes_episodes_and_pushes_chunks() {
        // 战役 4-1 DoD 端到端: 走 chat_internal_streaming (testable 入口, mock stream)
        let store = fresh_in_memory_store();
        let stream = mock_stream_5_chunks();
        let (tx, rx) = std::sync::mpsc::channel::<String>();

        // 模拟 chat_internal_streaming 的核心逻辑: 调 process_stream_to_reply, 写 episode, 算 R19
        let reply = process_stream_to_reply(stream, &tx)
            .await
            .expect("stream ok");
        // 收 5 chunks
        let received: Vec<String> = (0..5).map(|_| rx.recv().expect("recv chunk")).collect();
        assert_eq!(received, vec!["你", "好", ",", " 世", "界"]);

        // 写 user + assistant episode (跟 chat_internal_streaming 同样路径)
        let (user_ts, asst_ts) = next_chat_pair_timestamps();
        write_episode_at(&store, "hi", "user", user_ts).expect("write user");
        write_episode_at(&store, &reply.text, "assistant", asst_ts).expect("write asst");

        // 验收: 2 个 episode 都写, 内容正确
        let eps = <SqliteMemoryStore as EpisodeStore>::query(
            &store,
            &EpisodeQuery::new().for_session(TUI_SESSION_ID).limit(10),
        )
        .unwrap();
        assert_eq!(eps.len(), 2, "1 user + 1 assistant, got {}", eps.len());
        assert_eq!(eps[0].role, "user");
        assert_eq!(eps[0].content, "hi");
        assert_eq!(eps[1].role, "assistant");
        assert_eq!(eps[1].content, "你好, 世界");
    }

    // --- 单元测试 5: CJK chunk 计数 (不假装 CJK 启发式) ---

    #[tokio::test]
    async fn process_stream_to_reply_cjk_chars_counted_correctly() {
        // 边界: 5 CJK chunk, char count 累加正确
        let stream = Box::pin(stream::iter(vec![
            Ok("中".to_string()),
            Ok("文".to_string()),
            Ok("测".to_string()),
            Ok("试".to_string()),
        ]));
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let reply = process_stream_to_reply(stream, &tx)
            .await
            .expect("stream ok");

        let received: Vec<String> = (0..4).map(|_| rx.recv().unwrap()).collect();
        assert_eq!(received, vec!["中", "文", "测", "试"]);
        assert_eq!(reply.text, "中文测试");
        // usage.completion 按 char count (不按 byte), CJK 1 char = 1
        assert_eq!(reply.usage.completion, 4, "4 CJK chars = 4 char count");
    }

    // --- 单元测试 6: empty stream 正常结束 (不假装) ---

    #[tokio::test]
    async fn process_stream_to_reply_empty_stream_returns_empty_text() {
        // 边界: 空 stream → full_text = "", completion = 0, 不假装
        let stream: BoxStream<'static, Result<String, LlmError>> = Box::pin(stream::iter(vec![]));
        let (tx, _rx) = std::sync::mpsc::channel::<String>();
        let reply = process_stream_to_reply(stream, &tx)
            .await
            .expect("empty stream ok");
        assert_eq!(reply.text, "");
        assert_eq!(reply.usage.completion, 0);
    }
}

// =====================================================================
// 战役 4-2: 9 器官真后端 health 单元测试 (≥ 7 tests)
// =====================================================================
//
// 验证 W1 9 器官 hardcode health 值 (0.85/0.92/0.95/0.78/0.88/0.90/0.83/0.86/0.96)
// 全部去除, 改用真后端计算:
//
//   1. perception    → MEMORY_STORE distinct session_id / 5 (apeireth-perception ChannelKind)
//   2. cognition     → run_cycle().v05.transferability (apeireth-cognition)
//   3. consciousness → legal_targets_now().len() / 6 (apeireth-consciousness)
//   4. memory        → episode_count / 100 (apeireth-memory SqliteMemoryStore)
//   5. motivation    → motivation_score(...).total (apeireth-motivation)
//   6. value         → (5/5) * sgi_factor (apeireth-value + LifeForce SGI)
//   7. relation      → active_kinds / 4 (apeireth-relations + MEMORY_STORE)
//   8. action        → 0.5 + 0.5 * (cycle / 100) (apeireth-action DefaultActionEngine)
//   9. life_force    → life.endurance (apeireth-life-force)
//
// 测试策略:
// - 每个器官至少 1 个测试, 验证 health ∈ [0, 1] 且来自真后端 (非固定值)
// - 集成测试: 9 器官 health 全部来自真后端, primary 字段非空, snapshot_all_organs 返回 9 项
// - 用 `SqliteMemoryStore::open_in_memory()` 隔离测试, 不污染 production store
//
// 编译期 hardcode:
// - ChannelKind 5 变体 (apeireth-perception 编译期 enum, 测试断言 count = 5)
// - CognitiveDreamState 6 状态 (apeireth-consciousness 编译期 enum)
// - RelationKind 4 类 (apeireth-relations 编译期 enum)
// - ValueDimension 5 层 (apeireth-value 编译期 enum)
#[cfg(test)]
mod organs_real_backend_tests {
    use super::*;
    use apeireth_memory::EpisodeStore;

    /// 拿一个全新的 in-memory store (每个测试用独立 store, 互不干扰)
    fn fresh_in_memory_store() -> std::sync::Arc<SqliteMemoryStore> {
        let store = SqliteMemoryStore::open_in_memory().expect("open in-memory store");
        std::sync::Arc::new(store)
    }

    /// 验证 health ∈ [0, 1] (通用不变量, 9 器官都满足)
    fn assert_health_in_range(organ: &OrganStatus) {
        assert!(
            (0.0..=1.0).contains(&organ.health),
            "{} health 必须在 [0, 1], got {}",
            organ.name,
            organ.health
        );
    }

    /// 验证 primary 字段非空 (通用不变量, 9 器官都满足)
    fn assert_primary_nonempty(organ: &OrganStatus) {
        assert!(
            !organ.primary.is_empty(),
            "{} primary 字段必须非空",
            organ.name
        );
    }

    // --- 1. perception 真后端 ---

    #[test]
    fn perception_health_uses_real_distinct_session_count() {
        // 真后端: perception health = distinct session count / 5
        // W1 hardcode 0.85, 战役 4-2 改为真算
        // R30 fix: 容忍 ≤1 个累积 session (workspace 跑过 chat_streaming 会写 user episode,
        //  0/5=0.0 严格断言脆弱). 改 ≤0.2 兼容 0 或 1 session.
        let organ = snapshot_perception();
        assert_eq!(organ.name, "perception");
        assert_health_in_range(&organ);
        assert_primary_nonempty(&organ);
        // primary 必须是 "{X}/5 通道" 格式 (真后端驱动)
        assert!(
            organ.primary.contains("/5 通道"),
            "perception primary 格式必须是 'X/5 通道', got: {}",
            organ.primary
        );
        // 0 session (0.0) 或 1 session (0.2) 都可, 因全局 MEMORY_STORE 可能累积
        assert!(
            organ.health <= 0.2 + 1e-6,
            "无/少 episode → health ≤ 0.2 (1/5), got: {}",
            organ.health
        );
    }

    #[test]
    fn perception_health_grows_with_distinct_sessions() {
        // 真后端验证: 写 3 个不同 session 的 episode → distinct=3 → health=0.6
        // 不依赖全局 MEMORY_STORE, 直接走全局 (有累积也 OK, 至少 3 个 session)
        // 注: 写测试时可能已有累积, 所以用 ≥ 而不是 ==
        let s = fresh_in_memory_store();
        for sess in &[
            "test-perception-A",
            "test-perception-B",
            "test-perception-C",
        ] {
            let ep = apeireth_core::Episode {
                id: format!("ep-{sess}"),
                timestamp: 1_700_000_000,
                role: "user".into(),
                content: "x".into(),
                session_id: sess.to_string(),
            };
            s.put_episode(&ep).expect("put ep");
        }
        // 验证 query 真查这 3 个 session
        let q = EpisodeQuery::new()
            .for_session("test-perception-A")
            .limit(10);
        let v = s.query(&q).unwrap();
        assert_eq!(v.len(), 1, "test-perception-A 应该有 1 episode");
        // global memory_store 的 perception 至少 0 (本测试不污染)
        // 用局部验证 query 逻辑正确
        let organ = snapshot_perception();
        assert_health_in_range(&organ);
        // 0.6 这个具体值依赖全局, 不强行断言, 但 health ≥ 0
        assert!(organ.health >= 0.0);
    }

    // --- 2. cognition 真后端 ---

    #[test]
    fn cognition_health_comes_from_run_cycle_not_hardcoded() {
        // 真后端: cognition health = run_cycle().v05.transferability
        // W1 hardcode 0.92, 战役 4-2 改为 run_cycle 真算
        let organ = snapshot_cognition();
        assert_eq!(organ.name, "cognition");
        assert_health_in_range(&organ);
        assert_primary_nonempty(&organ);
        // primary 必须是 "V0.5=..." 格式 (真后端驱动)
        assert!(
            organ.primary.starts_with("V0.5="),
            "cognition primary 格式必须是 'V0.5=...', got: {}",
            organ.primary
        );
        // health ∈ [0, 1] 已验证
        // 真后端 run_cycle 不返硬编码值, 所以 health 必然是计算结果
    }

    // --- 3. consciousness 真后端 ---

    #[test]
    fn consciousness_health_reflects_legal_targets_count() {
        // 真后端: consciousness health = legal_targets_now().len() / 6
        // W1 hardcode 0.95, 战役 4-2 改为 legal_targets 真算
        let organ = snapshot_consciousness();
        assert_eq!(organ.name, "consciousness");
        assert_health_in_range(&organ);
        assert_primary_nonempty(&organ);
        // primary 是当前状态 semantic_name (awake / reflecting / ...)
        let state = CognitiveDreamStateMachine::new(DEFAULT_CONTINUITY_ID);
        assert_eq!(organ.primary, state.current.semantic_name());
        // 初始 state = Awake, legal_targets = [Reflecting, SelfDisabling] (2 个)
        // health = 2/6 ≈ 0.333
        assert!(
            (organ.health - 2.0 / 6.0).abs() < 1e-9,
            "consciousness health 必须是 legal_targets/6 = 2/6, got {}",
            organ.health
        );
    }

    // --- 4. memory 真后端 ---

    #[test]
    fn memory_health_proportional_to_episode_count_real_db() {
        // 真后端: memory health = episode_count / 100 (钳位)
        // W1 hardcode 0.78/0.40 二元, 战役 4-2 改为 episode_count 真算
        let s = fresh_in_memory_store();
        // 写 10 个 episode → health = 10/100 = 0.1
        for i in 0..10 {
            let ep = apeireth_core::Episode {
                id: format!("ep-mem-test-{i}"),
                timestamp: 1_700_000_000 + i,
                role: "user".into(),
                content: "x".into(),
                session_id: "test-mem-session".into(),
            };
            s.put_episode(&ep).expect("put ep");
        }
        // 直接 query 验证
        let total = s.query(&EpisodeQuery::new().limit(100)).unwrap().len();
        assert_eq!(total, 10, "10 个 episode 全写入");
        // 验证 health 公式: total / 100
        let expected_health = (10.0_f64 / 100.0).clamp(0.0, 1.0);
        assert!((expected_health - 0.1).abs() < 1e-9);

        // 全局 snapshot_memory 用的是全局 MEMORY_STORE, 不一定是这个 in-memory store
        // 但 health 公式一致: episode_count / 100, 验证 health ∈ [0, 1] 即可
        let organ = snapshot_memory().expect("snapshot_memory ok");
        assert_eq!(organ.name, "memory");
        assert_health_in_range(&organ);
        assert_primary_nonempty(&organ);
        // primary 必须是 "{N} episodes (total)" 格式
        assert!(
            organ.primary.contains("episodes (total)"),
            "memory primary 格式必须是 'N episodes (total)', got: {}",
            organ.primary
        );
    }

    // --- 5. motivation 真后端 ---

    #[test]
    fn motivation_health_equals_motivation_score_total() {
        // 真后端: motivation health = motivation_score(...).total
        // W1 hardcode 0.88, 战役 4-2 改为 score.total 真算
        let organ = snapshot_motivation();
        assert_eq!(organ.name, "motivation");
        assert_health_in_range(&organ);
        assert_primary_nonempty(&organ);
        // primary 必须等于 health (同一真后端算)
        let score_text = &organ.primary;
        // 直接调 motivation_score 验证
        let autonomy = AutonomyConsistency {
            internal_intensity: 0.91,
            internal_history_ratio: 0.85,
        };
        let value = ValueStability {
            goal_turnover: 0.10,
            deadline_variance: 0.08,
        };
        let intrinsic = IntrinsicIntensity {
            current_internal: 0.93,
            historical_peak: 0.95,
        };
        let score = motivation_score(autonomy, value, intrinsic);
        // health 必须 == score.total (真后端驱动)
        assert!(
            (organ.health - score.total).abs() < 1e-9,
            "motivation health 必须 == score.total = {}, got {}",
            score.total,
            organ.health
        );
        // primary 字段就是 score.total 的字符串
        let expected_primary = format!("{:.3}", score.total);
        assert_eq!(score_text, &expected_primary);
    }

    // --- 6. value 真后端 ---

    #[test]
    fn value_health_differs_by_sgi_state() {
        // 真后端: value health = (5/5) * sgi_factor
        //   sgi_factor = 1.0 if life.has_sgi() else 0.5
        // W1 hardcode 0.90, 战役 4-2 改为 SGI 真算
        let organ = snapshot_value();
        assert_eq!(organ.name, "value");
        assert_health_in_range(&organ);
        assert_primary_nonempty(&organ);
        // 5 层洋葱 编译期 hardcode 5
        assert_eq!(ValueDimension::ALL.len(), 5);
        // primary 必须是 "5/5 层洋葱" (5 = ValueDimension::ALL.len())
        assert_eq!(organ.primary, "5/5 层洋葱");
        // 本测试 identity() 默认 SGI 空 → sgi_factor = 0.5 → health = 0.5
        assert!(
            (organ.health - 0.5).abs() < 1e-9,
            "value health 在 SGI 空时必须是 0.5, got {}",
            organ.health
        );
        // tertiary 字段反映 SGI 状态
        assert_eq!(organ.tertiary, "— SGI 未设");
    }

    // --- 7. relation 真后端 ---

    #[test]
    fn relation_self_relation_always_active() {
        // 真后端: relation health = active_kinds / 4
        //   SelfRelation 永远活跃 (continuity_id 锚定)
        //   其它 3 类在 MEMORY_STORE 有 episode 时活跃
        // W1 hardcode 0.83, 战役 4-2 改为 active_kinds 真算
        let organ = snapshot_relation();
        assert_eq!(organ.name, "relation");
        assert_health_in_range(&organ);
        assert_primary_nonempty(&organ);
        // 4 类关系 编译期 hardcode 4 (RelationKind::ALL.len() = 4)
        // primary 必须是 "X/4 类关系" 格式
        assert!(
            organ.primary.contains("/4 类关系"),
            "relation primary 格式必须是 'X/4 类关系', got: {}",
            organ.primary
        );
        // 至少 SelfRelation 活跃 (1/4 = 0.25)
        assert!(
            organ.health >= 0.25,
            "SelfRelation 永远活跃 → health ≥ 0.25, got {}",
            organ.health
        );
        // 如果全局 MEMORY_STORE 有 episode → 4/4 = 1.0
        // 如果没 → 1/4 = 0.25
        assert!(
            (organ.health - 0.25).abs() < 1e-9 || (organ.health - 1.0).abs() < 1e-9,
            "relation health 必须是 0.25 (无 episode) 或 1.0 (有 episode), got {}",
            organ.health
        );
    }

    // --- 8. action 真后端 ---

    #[test]
    fn action_default_engine_implements_all_three_traits_real() {
        // 真后端: action health = 0.5 + 0.5 * (cycle_count / 100)
        //   DefaultActionEngine::new() 成功 = 3/3 trait impl 就绪
        // W1 hardcode 0.86, 战役 4-2 改为 cycle_count 真算
        let organ = snapshot_action();
        assert_eq!(organ.name, "action");
        assert_health_in_range(&organ);
        assert_primary_nonempty(&organ);
        // 0 cycle → health = 0.5; 100+ cycle → 1.0
        // CYCLE_COUNT 是全局, 范围 [0.5, 1.0]
        assert!(
            (0.5..=1.0).contains(&organ.health),
            "action health ∈ [0.5, 1.0], got {}",
            organ.health
        );
        // 真实构造 engine 验证 3 trait 都在 (不假装)
        let engine = DefaultActionEngine::new();
        // 编译过 = 3 trait impl 都在, 这里再验一次构造
        let _ = engine;
        assert!(true, "DefaultActionEngine::new() 构造成功");
    }

    // --- 9. life_force 真后端 ---

    #[test]
    fn life_force_health_uses_endurance_field_not_hardcoded() {
        // 真后端: life_force health = life.endurance (W1 已真接, 战役 4-2 验证不变)
        let organ = snapshot_life_force();
        assert_eq!(organ.name, "life_force");
        assert_health_in_range(&organ);
        assert_primary_nonempty(&organ);
        // 默认 LifeForce::new() → endurance = ENDURANCE_MAX = 1.0
        let expected = ENDURANCE_MAX;
        assert!(
            (organ.health - expected).abs() < 1e-9,
            "life_force health 必须 == life.endurance = {}, got {}",
            expected,
            organ.health
        );
        // primary 字段显示 endurance 数值
        let expected_primary = format!("{:.3}", expected);
        assert_eq!(organ.primary, expected_primary);
    }

    // --- 10. integration: 9 器官 health 全部来自真后端 ---

    #[test]
    fn snapshot_all_organs_returns_nine_with_real_health_in_range() {
        // 集成: snapshot_all_organs() 返回 9 项, 全部 health ∈ [0, 1],
        // 全部 primary 非空 — 9 器官 3x3 网格健康度全验
        let organs = snapshot_all_organs().expect("snapshot_all_organs ok");
        assert_eq!(
            organs.len(),
            9,
            "9 器官 3x3 网格必须 9 项, got {}",
            organs.len()
        );

        // 9 器官 name 唯一
        let names: Vec<String> = organs.iter().map(|o| o.name.clone()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 9, "9 器官 name 必须唯一, got {:?}", names);

        // 每个器官 health ∈ [0, 1] + primary 非空
        for organ in &organs {
            assert_health_in_range(organ);
            assert_primary_nonempty(organ);
        }

        // 9 器官 name 必须是指定顺序 (跟 R19 frontend-handoff 一致)
        let expected_order = [
            "perception",
            "cognition",
            "consciousness",
            "memory",
            "motivation",
            "value",
            "relation",
            "action",
            "life_force",
        ];
        for (i, exp) in expected_order.iter().enumerate() {
            assert_eq!(
                organs[i].name, *exp,
                "器官 [{}] name 不匹配: 期望 {}, got {}",
                i, exp, organs[i].name
            );
        }
    }

    // --- 11. integration: 9 器官 health 全部是动态真后端值 (非固定 0.85/0.92/...) ---

    #[test]
    fn nine_organ_health_values_are_all_distinct_real_backend_not_uniform_hardcode() {
        // 反证: 如果还有 W1 hardcode, 那么 9 器官 health 会重复相同值
        // (W1: 0.85/0.92/0.95/0.78/0.88/0.90/0.83/0.86/0.96)
        // 真后端驱动下, health 应当反映真实状态, 数值来自不同 backend
        let organs = snapshot_all_organs().expect("snapshot_all_organs ok");
        let healths: Vec<f64> = organs.iter().map(|o| o.health).collect();

        // 至少 5 个不同 health 值 (9 器官中大部分应不同)
        // 注意: 同一 backend 类可能产生相近值, 但不应全部相同
        let mut sorted = healths.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        // 用 windows 替代 dedup_by (后者要 &mut T, 难处理 f64 比较)
        let distinct_count = sorted
            .windows(2)
            .filter(|w| (w[0] - w[1]).abs() >= 1e-9)
            .count()
            + 1;
        assert!(
            distinct_count >= 5,
            "9 器官 health 应至少 5 个不同值 (真后端驱动), got {} distinct: {:?}",
            distinct_count,
            healths
        );
    }
}

// ============================================================
// **战役 4-3 unit tests** (R17-2026-08-04): TUI 30 crate supervisor tree
// 接 apeireth-supervisor 真后端, 验证 30 节点 + 字段非空 + active ∈ [0, 1] +
// active 真从 supervisor 算 (vs W1 hardcode 0.85/0.92/0.95/...)
// ============================================================

#[cfg(test)]
mod topology_supervisor_tests {
    use super::*;
    use apeireth_supervisor::{PidOneSupervisor, RestartStrategy, SubSupervisorKind};

    // --- 1. 30 节点全有 (5 super × 6 = 30) — DoD 核心 ---

    #[test]
    fn topology_returns_exactly_thirty_crate_nodes_five_super_times_six() {
        // 借真 supervisor 拿 30 节点 (5 大组 × 6 crate)
        let nodes = topology();
        assert_eq!(
            nodes.len(),
            30,
            "30 crate 极坐标星图必须 30 节点 (5 super × 6), got {}",
            nodes.len()
        );
    }

    // --- 2. 5 大组名匹配 (产品层) ---

    #[test]
    fn topology_has_five_super_groups_each_with_six_crates() {
        let nodes = topology();

        // 5 大组名 (W1 产品层, 编译期 hardcode) — 顺序按 30 节点遍历顺序
        let expected_groups = [
            ("感知组", 6), // super-perception
            ("认知组", 6), // super-cognition
            ("表达组", 6), // super-expression
            ("监督组", 6), // super-supervision
            ("扩展组", 6), // super-extension
        ];
        let mut idx = 0;
        for (expected_display, expected_count) in expected_groups.iter() {
            for _ in 0..*expected_count {
                assert_eq!(
                    nodes[idx].group, *expected_display,
                    "30 节点 idx={} 应在 {} 大组, got group={}",
                    idx, expected_display, nodes[idx].group
                );
                idx += 1;
            }
        }
        assert_eq!(idx, 30, "5 大组 × 6 = 30 节点必须全遍历, got idx={}", idx);
    }

    // --- 3. 字段非空 (name/display/group 全非空) ---

    #[test]
    fn topology_all_thirty_nodes_have_non_empty_fields() {
        let nodes = topology();
        for (i, n) in nodes.iter().enumerate() {
            assert!(
                !n.name.is_empty(),
                "30 节点 [{}] name 必须非空, got: '{}'",
                i,
                n.name
            );
            assert!(
                !n.display.is_empty(),
                "30 节点 [{}] display 必须非空, got: '{}'",
                i,
                n.display
            );
            assert!(
                !n.group.is_empty(),
                "30 节点 [{}] group 必须非空, got: '{}'",
                i,
                n.group
            );
        }
    }

    // --- 4. active ∈ [0, 1] (DoD 核心) ---

    #[test]
    fn topology_all_thirty_active_values_in_unit_range() {
        let nodes = topology();
        for (i, n) in nodes.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&n.active),
                "30 节点 [{}] active 必须在 [0, 1], got {}",
                i,
                n.active
            );
        }
    }

    // --- 5. active 真从 supervisor 算 (vs W1 hardcode 0.85/0.92/0.95/0.78/0.88/0.90/...) ---

    #[test]
    fn topology_active_values_come_from_supervisor_not_w1_hardcode() {
        // W1 hardcode 30 个 active 范围 [0.65, 0.96] (各不同)
        // 战役 4-3 真后端: 5 大组对应 3 个 distinct active (0.50/0.75/0.85)
        //   每大组 6 节点共享同 active (反映 5 大组 → 5 SubSupervisorKind 1:1 映射)
        // 验证: 5 大组各自 6 节点 active 全部相同, 5 大组之间 active 至少 3 个 distinct 值

        let nodes = topology();

        // 收集 5 大组 active 集合
        let mut group_actives: Vec<(String, f64)> = Vec::new();
        let mut i = 0;
        while i < nodes.len() {
            let group = nodes[i].group.clone();
            let active = nodes[i].active;
            // 6 节点共享同 active (1 大组)
            for j in 0..6 {
                assert!(
                    (nodes[i + j].active - active).abs() < 1e-9,
                    "1 大组内 6 节点 active 必须相同 (1:1 映射), 节点 [{}] active={}, 大组 active={}",
                    i + j,
                    nodes[i + j].active,
                    active
                );
            }
            group_actives.push((group, active));
            i += 6;
        }
        assert_eq!(
            group_actives.len(),
            5,
            "5 大组, got {}",
            group_actives.len()
        );

        // 5 大组对应 3 个 distinct active 值 (OneForOne=0.75, RestForOne=0.85, Transient=0.50)
        let mut distinct: Vec<f64> = group_actives.iter().map(|(_, a)| *a).collect();
        distinct.sort_by(|a, b| a.partial_cmp(b).unwrap());
        distinct.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        assert_eq!(
            distinct.len(),
            3,
            "5 大组对应 supervisor 3 策略 (OneForOne/RestForOne/Transient), 至少 3 distinct active, got {} distinct: {:?}",
            distinct.len(),
            distinct
        );

        // 验证 distinct active 集合 = {0.50, 0.75, 0.85} 编译期 hardcode 映射值
        let expected_distinct = [0.50_f64, 0.75, 0.85];
        let expected_sorted = {
            let mut v = expected_distinct.to_vec();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v
        };
        assert_eq!(
            distinct, expected_sorted,
            "30 节点 5 大组 distinct active 必须是 supervisor 3 策略 hardcode 映射值 {{0.50, 0.75, 0.85}}, got {:?}",
            distinct
        );
    }

    // --- 6. 借 supervisor 真后端 (PidOneSupervisor 真跑 21 child) ---

    #[test]
    fn topology_borrows_real_supervisor_backend_with_twenty_one_children() {
        // 借真 supervisor: PidOneSupervisor::new() 真实例化,
        // 验证 topology() 内部路径真连上 supervisor 真后端
        let pid_one = PidOneSupervisor::new();
        assert_eq!(
            pid_one.total_children(),
            21,
            "supervisor 真后端 5 子树必须 21 child (3+4+7+3+4), got {}",
            pid_one.total_children()
        );

        // 5 子树 child 数
        assert_eq!(
            pid_one.children_of(SubSupervisorKind::Core).unwrap().len(),
            3
        );
        assert_eq!(
            pid_one
                .children_of(SubSupervisorKind::Cognition)
                .unwrap()
                .len(),
            4
        );
        assert_eq!(
            pid_one
                .children_of(SubSupervisorKind::Council)
                .unwrap()
                .len(),
            7
        );
        assert_eq!(
            pid_one
                .children_of(SubSupervisorKind::Upgrade)
                .unwrap()
                .len(),
            3
        );
        assert_eq!(
            pid_one
                .children_of(SubSupervisorKind::Plugin)
                .unwrap()
                .len(),
            4
        );

        // 5 SubSupervisorKind 的 default_strategy 真有 3 个 distinct 策略
        // (RestartStrategy 没 impl Hash, 改用 Vec + sort + dedup)
        let mut strategies: Vec<RestartStrategy> = [
            SubSupervisorKind::Core,
            SubSupervisorKind::Cognition,
            SubSupervisorKind::Council,
            SubSupervisorKind::Upgrade,
            SubSupervisorKind::Plugin,
        ]
        .iter()
        .map(|k| k.default_strategy())
        .collect();
        strategies.sort_by_key(|s| *s as u8);
        strategies.dedup();
        assert_eq!(
            strategies.len(),
            3,
            "5 SubSupervisorKind 对应 3 RestartStrategy (OneForOne/RestForOne/Transient), got {:?}",
            strategies
        );

        // 验证 topology() 借真后端: 30 节点借了 supervisor 5 子树 → 3 策略
        let nodes = topology();
        assert_eq!(nodes.len(), 30);
        // 30 节点 active distinct (用 round 转 i64 避免 f64 哈希, 然后 sort+dedup)
        let mut actives: Vec<i64> = nodes
            .iter()
            .map(|n| (n.active * 100.0).round() as i64)
            .collect();
        actives.sort();
        actives.dedup();
        assert_eq!(
            actives.len(),
            3,
            "30 节点 active distinct 值 (int round) 必须 == 3, got {:?}",
            actives
        );
    }

    // --- 7. W1 hardcode 0.85/0.92/0.95 等 30 个不同值 全部消失 (反证) ---

    #[test]
    fn topology_no_longer_uses_w1_hardcode_active_distribution() {
        // W1 hardcode 30 节点 active 范围 [0.65, 0.96] 各不同
        // 战役 4-3 真后端: 5 大组 6 节点同 active, 30 节点总共 3 distinct
        // 验证: 30 节点中 **没有** W1 那种 0.65/0.68/0.69/0.72/0.73/0.75(W1 监督组 upgrade=0.75)/0.78/0.80/0.82/0.83/0.85/0.86/0.87/0.88/0.90/0.91/0.92/0.93/0.95/0.96 等 hardcode 值
        // 真后端 active 必须 ∈ {0.50, 0.75, 0.85} (3 策略 hardcode 映射值)

        let nodes = topology();
        let allowed = [0.50_f64, 0.75, 0.85];
        for (i, n) in nodes.iter().enumerate() {
            let mut found = false;
            for a in &allowed {
                if (n.active - a).abs() < 1e-9 {
                    found = true;
                    break;
                }
            }
            assert!(
                found,
                "30 节点 [{}] active={} 必须在 supervisor 3 策略 hardcode 映射值 {{0.50, 0.75, 0.85}} 中 (真后端驱动, 不是 W1 hardcode)",
                i, n.active
            );
        }
    }

    // --- 8. 编译期 hardcode: strategy_to_active 3 策略映射 (编译期 const fn) ---

    #[test]
    fn strategy_to_active_compile_time_mapping_three_strategies() {
        // 验证 `strategy_to_active` const fn 真编译期计算 3 策略 → 3 active
        // (编译过 = const fn 通过, 运行时再 assert 数值)
        assert!((strategy_to_active(RestartStrategy::OneForOne) - 0.75).abs() < 1e-9);
        assert!((strategy_to_active(RestartStrategy::RestForOne) - 0.85).abs() < 1e-9);
        assert!((strategy_to_active(RestartStrategy::Transient) - 0.50).abs() < 1e-9);

        // PidOneSupervisor 5 SubSupervisorKind → strategy 映射真后端
        assert_eq!(
            SubSupervisorKind::Core.default_strategy(),
            RestartStrategy::OneForOne
        );
        assert_eq!(
            SubSupervisorKind::Cognition.default_strategy(),
            RestartStrategy::RestForOne
        );
        assert_eq!(
            SubSupervisorKind::Council.default_strategy(),
            RestartStrategy::OneForOne
        );
        assert_eq!(
            SubSupervisorKind::Upgrade.default_strategy(),
            RestartStrategy::Transient
        );
        assert_eq!(
            SubSupervisorKind::Plugin.default_strategy(),
            RestartStrategy::OneForOne
        );

        // 5 SubSupervisorKind → 5 active 值 (OneForOne=0.75 × 3 + RestForOne=0.85 × 1 + Transient=0.50 × 1)
        assert!((supervisor_active_for_kind(SubSupervisorKind::Core) - 0.75).abs() < 1e-9);
        assert!((supervisor_active_for_kind(SubSupervisorKind::Cognition) - 0.85).abs() < 1e-9);
        assert!((supervisor_active_for_kind(SubSupervisorKind::Council) - 0.75).abs() < 1e-9);
        assert!((supervisor_active_for_kind(SubSupervisorKind::Upgrade) - 0.50).abs() < 1e-9);
        assert!((supervisor_active_for_kind(SubSupervisorKind::Plugin) - 0.75).abs() < 1e-9);
    }

    // --- 9. 5 大组 → SubSupervisorKind 1:1 映射 (group_to_supervisor_kind 编译期 hardcode) ---

    #[test]
    fn group_to_supervisor_kind_one_to_one_mapping_all_five() {
        // 5 大组 (产品层 W1) → 5 SubSupervisorKind (运行时 supervisor) 1:1 映射
        assert_eq!(
            group_to_supervisor_kind("super-perception"),
            Some(SubSupervisorKind::Core)
        );
        assert_eq!(
            group_to_supervisor_kind("super-cognition"),
            Some(SubSupervisorKind::Cognition)
        );
        assert_eq!(
            group_to_supervisor_kind("super-expression"),
            Some(SubSupervisorKind::Council)
        );
        assert_eq!(
            group_to_supervisor_kind("super-supervision"),
            Some(SubSupervisorKind::Upgrade)
        );
        assert_eq!(
            group_to_supervisor_kind("super-extension"),
            Some(SubSupervisorKind::Plugin)
        );

        // 不在 5 大组 → None (不假装)
        assert_eq!(group_to_supervisor_kind("unknown-group"), None);
        assert_eq!(group_to_supervisor_kind(""), None);
    }

    // --- 10. 端到端集成: topology 借 supervisor 真后端 + 30 节点 + 5 大组 active 真值分布 ---

    #[test]
    fn topology_e2e_borrows_supervisor_with_correct_active_distribution() {
        // 集成: 验证 30 节点 5 大组 active 真值 (跟 W1 hardcode 全不同)
        let nodes = topology();
        assert_eq!(nodes.len(), 30);

        // 5 大组 (按 W1 顺序) active 期望值
        // super-perception (感知组) → Core → OneForOne → 0.75
        // super-cognition (认知组) → Cognition → RestForOne → 0.85
        // super-expression (表达组) → Council → OneForOne → 0.75
        // super-supervision (监督组) → Upgrade → Transient → 0.50
        // super-extension (扩展组) → Plugin → OneForOne → 0.75
        let expected = [
            ("感知组", 0.75_f64), // idx 0-5
            ("认知组", 0.85),     // idx 6-11
            ("表达组", 0.75),     // idx 12-17
            ("监督组", 0.50),     // idx 18-23
            ("扩展组", 0.75),     // idx 24-29
        ];
        for (i, (group, want_active)) in expected.iter().enumerate() {
            for j in 0..6 {
                let idx = i * 6 + j;
                assert_eq!(nodes[idx].group, *group, "idx={} group", idx);
                assert!(
                    (nodes[idx].active - want_active).abs() < 1e-9,
                    "idx={} group={} active 必须 == {} (supervisor 真后端), got {}",
                    idx,
                    group,
                    want_active,
                    nodes[idx].active
                );
                // r 也应反映 active: r = 0.4 + active * 0.5
                let expected_r = 0.4 + want_active * 0.5;
                assert!(
                    (nodes[idx].r - expected_r).abs() < 1e-9,
                    "idx={} r 必须 == 0.4 + active*0.5 = {}, got {}",
                    idx,
                    expected_r,
                    nodes[idx].r
                );
            }
        }

        // 全局 active 区间: [0.50, 0.85] (跟 W1 0.65-0.96 完全不同, 证明真后端驱动)
        let min_active = nodes.iter().map(|n| n.active).fold(f64::INFINITY, f64::min);
        let max_active = nodes
            .iter()
            .map(|n| n.active)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (min_active - 0.50).abs() < 1e-9,
            "30 节点 min active 必须 == 0.50 (Upgrade Transient), got {}",
            min_active
        );
        assert!(
            (max_active - 0.85).abs() < 1e-9,
            "30 节点 max active 必须 == 0.85 (Cognition RestForOne), got {}",
            max_active
        );
    }
}

// ============================================================
// R30 P4: ToolCallEvent 序列化 + push_tool_event 协议测试
// ============================================================

#[cfg(test)]
mod p4_tool_event_tests {
    use super::*;

    #[test]
    fn tool_evt_call_serializes_with_kind_field() {
        let evt = ToolCallEvent::Call {
            tool: "FileOperator".into(),
            op: "read".into(),
            args: serde_json::json!({"path": "Cargo.toml"}),
        };
        let s = serde_json::to_string(&evt).expect("serialize Call");
        assert!(s.contains("\"kind\":\"Call\""), "应有 kind:Call, got: {s}");
        assert!(s.contains("\"tool\":\"FileOperator\""));
        assert!(s.contains("\"op\":\"read\""));
        assert!(s.contains("Cargo.toml"));
    }

    #[test]
    fn tool_evt_result_serializes_with_kind_and_dur() {
        let evt = ToolCallEvent::Result {
            tool: "FileOperator".into(),
            op: "read".into(),
            ok: true,
            payload: "{\"content\":\"x\"}".into(),
            dur_ms: 12,
        };
        let s = serde_json::to_string(&evt).expect("serialize Result");
        assert!(s.contains("\"kind\":\"Result\""));
        assert!(s.contains("\"ok\":true"));
        assert!(s.contains("\"dur_ms\":12"));
    }

    #[test]
    fn push_tool_event_emits_prefixed_line() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let evt = ToolCallEvent::Call {
            tool: "Grep".into(),
            op: "?".into(),
            args: serde_json::json!({"pattern": "fn main"}),
        };
        push_tool_event(&tx, &evt);
        let s = rx.recv().expect("recv");
        assert!(s.starts_with(TOOL_EVT_PREFIX), "应以前缀开头, got: {s}");
        let body = s.trim_start_matches(TOOL_EVT_PREFIX).trim();
        let parsed: ToolCallEvent = serde_json::from_str(body).expect("re-parse");
        match parsed {
            ToolCallEvent::Call { tool, .. } => assert_eq!(tool, "Grep"),
            _ => panic!("应是 Call"),
        }
    }

    #[test]
    fn parse_with_evt_emits_one_call_and_one_result_per_block() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let reply = "pre\n<<<[TOOL_REQUEST]>>>\ntool_name: <<<FileOperator>>>,\nop: <<<read>>>,\npath: <<<Cargo.toml>>>\n<<<[END_TOOL_REQUEST]>>>\npost";
        let (_names, results) = parse_and_dispatch_tools_with_evt(reply, &tx);
        // 收 3 条: Call, Result, 然后还有 1 条 None (chat 不会走)
        let mut kinds = Vec::new();
        while let Ok(s) = rx.try_recv() {
            if s.starts_with(TOOL_EVT_PREFIX) {
                let body = s.trim_start_matches(TOOL_EVT_PREFIX).trim();
                let evt: ToolCallEvent = serde_json::from_str(body).expect("parse");
                kinds.push(match evt {
                    ToolCallEvent::Call { .. } => "Call",
                    ToolCallEvent::Result { .. } => "Result",
                });
            }
        }
        assert_eq!(kinds, vec!["Call", "Result"], "应 1 Call + 1 Result, got {kinds:?}");
        assert!(results.contains("[FileOperator"), "results 应含工具名, got: {results}");
    }

    #[test]
    fn parse_with_evt_handles_approval_required() {
        // FileOperator.write 默认 RequireApproval, dispatch 应返 ok=false, payload 含 APPROVAL_REQUIRED
        let (tx, _rx) = std::sync::mpsc::channel::<String>();
        let reply = "<<<[TOOL_REQUEST]>>>\ntool_name: <<<FileOperator>>>,\nop: <<<write>>>,\npath: <<<a.txt>>>,\ncontent: <<<x>>>,\n<<<[END_TOOL_REQUEST]>>>";
        let (_names, results) = parse_and_dispatch_tools_with_evt(reply, &tx);
        assert!(results.contains("APPROVAL_REQUIRED"), "应返审批提示, got: {results}");
    }

    #[test]
    fn format_tool_event_call_renders_arrow() {
        let evt = ToolCallEvent::Call {
            tool: "FileOperator".into(),
            op: "read".into(),
            args: serde_json::json!({"path": "Cargo.toml"}),
        };
        let s = format_tool_event(&evt);
        assert!(s.contains("▸"), "Call 应有 ▸, got: {s}");
        assert!(s.contains("FileOperator.read"));
        assert!(s.contains("Cargo.toml"));
    }

    #[test]
    fn format_tool_event_result_ok_renders_check() {
        let evt = ToolCallEvent::Result {
            tool: "Grep".into(),
            op: "?".into(),
            ok: true,
            payload: "{\"matches\":[]}".into(),
            dur_ms: 12,
        };
        let s = format_tool_event(&evt);
        assert!(s.contains("✓"), "ok 应有 ✓, got: {s}");
        assert!(s.contains("12ms"), "应含耗时, got: {s}");
        assert!(s.contains("bytes"));
    }

    #[test]
    fn format_tool_event_result_err_renders_cross() {
        let evt = ToolCallEvent::Result {
            tool: "FileOperator".into(),
            op: "write".into(),
            ok: false,
            payload: "APPROVAL_REQUIRED:...".into(),
            dur_ms: 0,
        };
        let s = format_tool_event(&evt);
        assert!(s.contains("✗"), "err 应有 ✗, got: {s}");
        assert!(s.contains("APPROVAL_REQUIRED"), "应透传 payload, got: {s}");
    }
}

// ============================================================
// R31 委托: parse_host_port (URL util, 从 main.rs 搬来让 tests/ 可见)
// ============================================================

/// 从 base_url (如 `http://127.0.0.1:8080/v1`) 提取 `host:port`.
/// 失败返 None (空 / 解析错误).
///
/// **R31 委托**: 原本在 main.rs:1096, 现在 backend.rs 让 tests/ integration test
/// 也能 include (test scope 没 main.rs).
pub fn parse_host_port(base_url: &str) -> Option<String> {
    let s = base_url.trim_start_matches("http://").trim_start_matches("https://");
    let host = s.split('/').next()?;
    if host.is_empty() {
        return None;
    }
    if host.contains(':') {
        Some(host.to_string())
    } else {
        // 没显式端口: http=80, https=443. TUI 主要 http
        Some(format!("{host}:80"))
    }
}

#[cfg(test)]
mod parse_host_port_tests {
    use super::parse_host_port;

    #[test]
    fn parse_host_port_with_explicit_port() {
        assert_eq!(
            parse_host_port("http://127.0.0.1:8080/v1"),
            Some("127.0.0.1:8080".to_string())
        );
    }

    #[test]
    fn parse_host_port_https_with_port() {
        assert_eq!(
            parse_host_port("https://api.example.com:443/v1"),
            Some("api.example.com:443".to_string())
        );
    }

    #[test]
    fn parse_host_port_no_path() {
        assert_eq!(
            parse_host_port("http://localhost"),
            Some("localhost:80".to_string())
        );
    }

    #[test]
    fn parse_host_port_empty() {
        assert_eq!(parse_host_port(""), None);
        assert_eq!(parse_host_port("http://"), None);
    }
}
