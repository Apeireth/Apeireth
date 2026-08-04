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

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use apeireth_asi::{AsiV05Scores, DimensionRegistry, DimensionTrace, MeasurementSample, V05_DIMENSION_NAMES};
use apeireth_cognition::{run_cycle, CognitiveInput, CognitiveOutput};
use apeireth_consciousness::CognitiveDreamStateMachine;
use apeireth_core::{ActionTarget, IdentityCard, LifeStage};
use apeireth_life_force::{exhaustion_check, LifeForce, SelfGrowthIndicator, ENDURANCE_MAX};
use apeireth_memory::{Episode, EpisodeQuery, EpisodeStore, IdentityCardStore, SqliteMemoryStore};
use apeireth_motivation::{motivation_score, AutonomyConsistency, IntrinsicIntensity, ValueStability};
use apeireth_sovereignty::self_disable::SelfDisableGuard;
use apeireth_value::ValueDimension;
use serde::{Deserialize, Serialize};

// ============================================================
// 全局状态 (跨页面共享)
// ============================================================

/// Cognitive cycle 累计计数 (每次对话/认知操作 +1)
pub static CYCLE_COUNT: AtomicU64 = AtomicU64::new(0);
/// Token 累计 (W1 mock, W2 接 R17 apeireth-api)
pub static TOKEN_USED: AtomicU64 = AtomicU64::new(142_857);

/// SqliteMemoryStore 全局单例 (跟 R18 web 一致)
static MEMORY_STORE: OnceLock<Arc<SqliteMemoryStore>> = OnceLock::new();

/// SelfDisableGuard 全局单例 (跟 R18 sovereignty.rs 一致)
static SOVEREIGNTY_GUARD: OnceLock<Arc<Mutex<SelfDisableGuard>>> = OnceLock::new();

/// 默认 IdentityCard continuity_id (跟 R18 web 一致)
pub const DEFAULT_CONTINUITY_ID: &str = "apeireth-tui-default";

// ============================================================
// 数据结构 (TUI 内部使用)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganStatus {
    pub name: String,
    pub display: String,
    pub metaphor: String,
    /// 主指标归一 [0, 1]
    pub health: f64,
    pub primary: String,
    pub secondary: String,
    pub tertiary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainAiStatus {
    /// 大 3
    pub asi_v05: f64,
    pub asi_continuity: f64,
    pub asi_philosophy: f64,
    pub life_stage: String,
    pub life_stage_idx: u8,
    pub reflection_status: String,
    pub endurance: f64,
    /// 中 2
    pub episode_count: u64,
    pub cycle_count: u64,
    /// 小字 2
    pub token_used: u64,
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
    // DB 路径: <CARGO_MANIFEST_DIR>/../apeireth-memory.db (跟 R18 web/desktop 一致)
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

/// R19 8 阶段 (跟 R11 LOCKED 10 阶段 enum 错位: Decline/Death 砍)
fn r19_stage_zh(enum_name: &str) -> Option<(&'static str, &'static str, u8)> {
    match enum_name {
        "Gestation" => Some(("孕育", "Gestation", 1)),
        "Birth" => Some(("诞生", "Birth", 2)),
        "Infancy" => Some(("幼儿", "Infancy", 3)),
        "Growth" => Some(("成长", "Growth", 4)),
        "Maturity" => Some(("成熟", "Maturity", 5)),
        "Reproduction" => Some(("繁衍", "Reproduction", 6)),
        "Migration" => Some(("迁移", "Migration", 7)),
        "Rebirth" => Some(("重生", "Rebirth", 8)),
        // ❌ Decline / Death: R11 LOCKED enum, R19 UI 不显示
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
// 主 AI 状态 + 9 器官 snapshot (跟 W2 模式一致)
// ============================================================

pub fn compute_main_ai_status() -> Result<MainAiStatus, String> {
    // 1. ASI V0.5
    let v05 = compute_v05();
    let continuity = v05.continuity;
    let philosophy = v05.philosophy_guard;

    // 2. 阶段
    let (stage_zh, stage_idx) = compute_life_stage()?;

    // 3. 反思期
    let reflection_status = compute_reflection_status();

    // 4. endurance
    let endurance = compute_endurance();

    // 5. Episode 数
    let episode_count = memory_store()
        .ok()
        .and_then(|s| {
            s.query(&EpisodeQuery::new().limit(usize::MAX))
                .ok()
                .map(|v| v.len() as u64)
        })
        .unwrap_or(0);

    // 6. cycle 累计
    let cycle_count = CYCLE_COUNT.load(Ordering::Relaxed);

    // 7. token 累计 (W1 mock)
    let token_used = TOKEN_USED.load(Ordering::Relaxed);

    // 8. 5 Self (SelfDisableGuard)
    let five_self = {
        let guard = sovereignty_guard();
        let g = guard.lock().map_err(|e| format!("sovereignty mutex: {e}"))?;
        if g.is_armed { "✓ armed" } else { "✗ disarmed" }.to_string()
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
        five_self,
    })
}

pub fn snapshot_perception() -> OrganStatus {
    OrganStatus {
        name: "perception".into(),
        display: "感知".into(),
        metaphor: "五感".into(),
        health: 0.85,
        primary: "5/5 通道".into(),
        secondary: "Text · Voice · Vision".into(),
        tertiary: "events/s: 12".into(),
    }
}

pub fn snapshot_cognition() -> OrganStatus {
    // 跑 1 次 run_cycle 看状态 (跟 W2 一致)
    let cognitive = CognitiveInput::new(
        vec![ActionTarget::NormalAction("cognition-snapshot".into())],
        "cognition-snapshot",
    );
    let (primary, secondary, tertiary) = match run_cycle(cognitive) {
        Ok(c) => {
            let v05 = c.v05.transferability;
            let verdicts = c.verdicts.len();
            (
                format!("V0.5={:.3}", v05),
                format!("{} verdicts · allowed={}", verdicts, matches!(c.output, CognitiveOutput::Decision(_))),
                format!("cycle {}", CYCLE_COUNT.load(Ordering::Relaxed)),
            )
        }
        Err(e) => (format!("err: {e:?}"), "—".into(), "—".into()),
    };
    OrganStatus {
        name: "cognition".into(),
        display: "认知".into(),
        metaphor: "大脑".into(),
        health: 0.92,
        primary,
        secondary,
        tertiary,
    }
}

pub fn snapshot_consciousness() -> OrganStatus {
    let machine = CognitiveDreamStateMachine::new(DEFAULT_CONTINUITY_ID);
    let state = machine.current;
    OrganStatus {
        name: "consciousness".into(),
        display: "意识".into(),
        metaphor: "心智".into(),
        health: 0.95,
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

pub fn snapshot_memory() -> Result<OrganStatus, String> {
    let s = memory_store()?;
    let total = s
        .query(&EpisodeQuery::new().limit(usize::MAX))
        .map(|v| v.len())
        .unwrap_or(0);
    let web_count = s
        .query(&EpisodeQuery::new().for_session("web-session").limit(usize::MAX))
        .map(|v| v.len())
        .unwrap_or(0);
    let council_count = s
        .query(&EpisodeQuery::new().for_session("council-history").limit(usize::MAX))
        .map(|v| v.len())
        .unwrap_or(0);
    let has_identity = s.get(DEFAULT_CONTINUITY_ID).map(|opt| opt.is_some()).unwrap_or(false);
    let health = if has_identity && total > 0 { 0.78 } else { 0.40 };
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

pub fn snapshot_motivation() -> OrganStatus {
    // 调 R18 motivation 真实 motivation_score (跟 W2 一致)
    let autonomy = AutonomyConsistency { internal_intensity: 0.91, internal_history_ratio: 0.85 };
    let value = ValueStability { goal_turnover: 0.10, deadline_variance: 0.08 };
    let intrinsic = IntrinsicIntensity { current_internal: 0.93, historical_peak: 0.95 };
    let score = motivation_score(autonomy, value, intrinsic);
    OrganStatus {
        name: "motivation".into(),
        display: "动机".into(),
        metaphor: "多巴胺".into(),
        health: 0.88,
        primary: format!("{:.3}", score.total),
        secondary: if score.passes_threshold { "✓ 过门槛 (≥0.85)".into() } else { "✗ 未过门槛".into() },
        tertiary: "SGI 7 约束 (C-SGI-1~7)".into(),
    }
}

pub fn snapshot_value() -> OrganStatus {
    let dims = ValueDimension::ALL;
    let count = dims.len();
    OrganStatus {
        name: "value".into(),
        display: "价值".into(),
        metaphor: "前额叶".into(),
        health: 0.90,
        primary: format!("{}/{} 层洋葱", count, count),
        secondary: dims.iter().map(|d| d.letter().to_string()).collect::<Vec<_>>().join(" / "),
        tertiary: "硬门槛 ≥ 0.85".into(),
    }
}

pub fn snapshot_relation() -> OrganStatus {
    // TUI: 跟 W2 一致, 静态展示 4 类关系
    OrganStatus {
        name: "relation".into(),
        display: "关系".into(),
        metaphor: "镜像神经元".into(),
        health: 0.83,
        primary: "4 类关系".into(),
        secondary: "共生 / 协调 / 嵌入 / 与自身".into(),
        tertiary: format!("continuity: {DEFAULT_CONTINUITY_ID}"),
    }
}

pub fn snapshot_action() -> OrganStatus {
    OrganStatus {
        name: "action".into(),
        display: "行动".into(),
        metaphor: "肌肉".into(),
        health: 0.86,
        primary: "3 模式".into(),
        secondary: "Execute / Express / Silence".into(),
        tertiary: "12 键 hardcode 拒绝".into(),
    }
}

pub fn snapshot_life_force() -> OrganStatus {
    let life = LifeForce::new(identity(), now_ts());
    let exhausted = exhaustion_check(&life);
    OrganStatus {
        name: "life_force".into(),
        display: "生命力".into(),
        metaphor: "免疫".into(),
        health: life.endurance,
        primary: format!("{:.3}", life.endurance),
        secondary: if life.is_in_reflection(now_ts()) { "反思期 active".into() } else { "dormant".into() },
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
// ASI V0.5 (R18 asi.rs 同样模式: 合成 sample + DimensionRegistry)
// ============================================================

pub fn compute_v05() -> AsiV05Scores {
    let registry = DimensionRegistry::new();
    let mut sample = MeasurementSample::default();
    for name in V05_DIMENSION_NAMES.iter() {
        sample.successes.insert((*name).to_string(), 90);
        sample.attempts.insert((*name).to_string(), 100);
        sample.qualities.insert((*name).to_string(), 1.0);
    }
    let v05_dims = registry.compute_all_dims(&sample);
    let trace = DimensionTrace {
        trace_id: 0,
        sample_id: 0,
        timestamp: now_ts(),
        v05_dims,
        v1136_subs: [0.0; 9],
        hook_overrides: vec![],
    };
    AsiV05Scores::from_trace(&trace)
}

// ============================================================
// 阶段判据 (跟 W2 简化一致)
// ============================================================

pub fn compute_life_stage() -> Result<(String, u8), String> {
    let episode_count = memory_store()?
        .query(&EpisodeQuery::new().limit(usize::MAX))
        .map(|v| v.len() as u64)
        .unwrap_or(0);
    if episode_count == 0 {
        Ok(("孕育".into(), 1))
    } else if episode_count < 10 {
        Ok(("幼儿".into(), 3))
    } else if episode_count < 100 {
        Ok(("成长".into(), 4))
    } else {
        Ok(("成熟".into(), 5))
    }
}

pub fn compute_reflection_status() -> String {
    let mut life = LifeForce::new(identity(), now_ts());
    life.sgi = SelfGrowthIndicator::new("assist-and-reflect", now_ts());
    if life.is_in_reflection(now_ts()) { "active".into() } else { "dormant".into() }
}

pub fn compute_endurance() -> f64 {
    let life = LifeForce::new(identity(), now_ts());
    life.endurance / ENDURANCE_MAX
}

/// 反思期进度 [0.0, 1.0]: 反思未触发 = 0, 触发后 72h 内线性增长
pub fn compute_reflection_progress() -> f64 {
    let mut life = LifeForce::new(identity(), now_ts());
    if life.reflection.started_at == 0 {
        return 0.0;
    }
    let now = now_ts();
    let elapsed = (now - life.reflection.started_at).max(0);
    let total = 72 * 3600_i64;
    ((elapsed as f64) / (total as f64)).clamp(0.0, 1.0)
}

pub fn compute_life_stages_info() -> Result<Vec<LifeStageInfo>, String> {
    let active_idx = compute_life_stage().map(|(_, i)| i).unwrap_or(4);
    let mut stages = Vec::new();
    for stage in [
        LifeStage::Gestation,
        LifeStage::Birth,
        LifeStage::Infancy,
        LifeStage::Growth,
        LifeStage::Maturity,
        LifeStage::Reproduction,
        LifeStage::Migration,
        LifeStage::Rebirth,
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

// ============================================================
// 30 crate supervisor tree (5 大组, 极坐标布局)
// ============================================================

/// 6 个一组, 5 大组 (super-perception/cognition/expression/supervision/extension)
pub fn topology() -> Vec<CrateNode> {
    let mut nodes = Vec::new();
    let groups: [(&str, &str, f64, [(&str, &str, f64); 6]); 5] = [
        (
            "super-perception",
            "感知组",
            0.95,
            [
                ("apeireth-perception", "感知", 0.85),
                ("apeireth-cognition", "认知", 0.92),
                ("apeireth-consciousness", "意识", 0.95),
                ("apeireth-memory", "记忆", 0.78),
                ("apeireth-motivation", "动机", 0.88),
                ("apeireth-value", "价值", 0.90),
            ],
        ),
        (
            "super-cognition",
            "认知组",
            0.93,
            [
                ("apeireth-asi", "ASI", 0.91),
                ("apeireth-bench", "基准", 0.68),
                ("apeireth-test", "测试", 0.72),
                ("apeireth-verify", "验证", 0.69),
                ("apeireth-philosophy", "哲学", 0.78),
                ("apeireth-council", "智囊团", 0.87),
            ],
        ),
        (
            "super-expression",
            "表达组",
            0.86,
            [
                ("apeireth-relation", "关系", 0.83),
                ("apeireth-action", "行动", 0.86),
                ("apeireth-life-force", "生命力", 0.96),
                ("apeireth-sovereignty", "主权", 0.93),
                ("apeireth-onion", "洋葱", 0.88),
                ("apeireth-constraint", "约束", 0.85),
            ],
        ),
        (
            "super-supervision",
            "监督组",
            0.90,
            [
                ("apeireth-supervisor", "总监督", 0.95),
                ("apeireth-central", "中央", 0.90),
                ("apeireth-core", "核心", 0.92),
                ("apeireth-bus", "总线", 0.88),
                ("apeireth-upgrade", "升级", 0.75),
                ("apeireth-evolution", "演化", 0.73),
            ],
        ),
        (
            "super-extension",
            "扩展组",
            0.78,
            [
                ("apeireth-web", "Web", 0.85),
                ("apeireth-desktop", "Desktop", 0.82),
                ("apeireth-tui", "TUI", 0.80),
                ("apeireth-cli", "CLI", 0.80),
                ("apeireth-api", "API", 0.82),
                ("apeireth-pybridge", "桥", 0.65),
            ],
        ),
    ];
    for (g, g_display, _g_active, members) in groups.iter() {
        for (i, (name, display, active)) in members.iter().enumerate() {
            // 每组内 6 个均分 2π, 半径按 active 映射
            let theta = (i as f64) * std::f64::consts::TAU / 6.0;
            let r = 0.4 + active * 0.5;
            nodes.push(CrateNode {
                name: (*name).into(),
                display: (*display).into(),
                group: g_display.into(),
                r,
                theta,
                active: *active,
            });
            let _ = g;
        }
    }
    nodes
}

// ============================================================
// 对话 (TUI 真接 run_cycle, 跟 W2 同模式)
// ============================================================

pub fn chat(input: &str) -> String {
    CYCLE_COUNT.fetch_add(1, Ordering::Relaxed);
    let cycle_target = ActionTarget::NormalAction(format!("tui-chat:{input}"));
    let cognitive = CognitiveInput::new(vec![cycle_target], "tui-chat");
    match run_cycle(cognitive) {
        Ok(c) => match &c.output {
            CognitiveOutput::Decision(s) => format!(
                "✓ 收到: {} (v0.5 transferability={:.3}, verdicts={})",
                input,
                c.v05.transferability,
                c.verdicts.len()
            ),
            CognitiveOutput::Reject(key) => format!(
                "✗ 拒绝 (verdict={:?}): {}",
                key, input
            ),
        }
        .into(),
        Err(e) => format!("(TUI) run_cycle 错误: {e:?}"),
    }
}

// ============================================================
// 历史流 (按 session/continuity 分组, 6 流 = R18 6 流水)
// ============================================================

const HISTORY_SESSIONS: [&str; 6] = [
    "web-session",
    "council-history",
    "desktop-session",
    "tui-session",
    "evolution-stream",
    "reflection-stream",
];

/// 6 流计数 + 总数
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

/// 加载最近 N 条 Episode
pub fn history_recent(limit: usize) -> Result<Vec<Episode>, String> {
    let store = memory_store()?;
    // 直接取全部 (sqlite 已按时间排), 取尾 N
    let all = store
        .query(&EpisodeQuery::new().limit(usize::MAX))
        .map_err(|e| format!("query: {e}"))?;
    Ok(all.into_iter().rev().take(limit).collect::<Vec<_>>().into_iter().rev().collect())
}

/// 6 流 (R18 memory 6 流水) 静态标签
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

// 引用 LEGAL_TRANSITIONS 验证 R11 LOCKED 边界
#[allow(dead_code)]
pub fn legal_transitions_count() -> usize {
    apeireth_central::LEGAL_TRANSITIONS.len()
}
