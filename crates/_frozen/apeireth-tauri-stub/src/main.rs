//! Apeireth Tauri 2 Desktop 参考实现 — ⚠️ DEPRECATED (V2 Day 1 Step 1.3)
//!
//! 原属 crate `apeireth-desktop`,已重命名为 `apeireth-tauri-stub`.
//! 本文件不在产品里,仅作为 R19 战役参考样例保留.
//!
//! **R11 LOCKED 边界** (omnibus §6): 本文件只调后端 crate 的公开 API,
//! 不修改任何 R11 LOCKED enum / 转换矩阵 / 8 项不修改承诺.
//!
//! **W2 桥接映射**:
//! - `get_main_ai_status` → cognition (run_cycle) + memory (Episode 数) + life-force (endurance) + asi (V0.5) + sovereignty (5 Self) + value (5 层) + central (阶段)
//! - `get_organ_status` → 9 器官真值
//! - `get_life_stages` → apeireth_central LEGAL_TRANSITIONS (砍 Decline/Death UI)
//! - `get_topology` → 30 crate supervisor tree
//! - `chat` → 流式 (R17 apeireth-api serve.rs 风格)

// 始终 windows subsystem (不弹 cmd 终端, 主人 UI review 干净)
#![cfg_attr(all(), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use apeireth_asi::{AsiV05Scores, DimensionRegistry, MeasurementSample, V05_DIMENSION_NAMES};
use apeireth_central::LEGAL_TRANSITIONS;
use apeireth_cognition::{run_cycle, CognitiveInput, CognitiveOutput};
use apeireth_consciousness::CognitiveDreamStateMachine;
use apeireth_core::{ActionTarget, IdentityCard, LifeStage};
use apeireth_life_force::{exhaustion_check, LifeForce, SelfGrowthIndicator, ENDURANCE_MAX};
use apeireth_memory::{EpisodeQuery, EpisodeStore, IdentityCardStore, SqliteMemoryStore};
use apeireth_motivation::{motivation_score, AutonomyConsistency, IntrinsicIntensity, ValueStability};
use apeireth_sovereignty::self_disable::SelfDisableGuard;
use apeireth_value::ValueDimension;
use serde::{Deserialize, Serialize};

// ============================================================
// 全局状态 (跨 tauri::command 共享)
// ============================================================

/// Cognitive cycle 累计计数
static CYCLE_COUNT: AtomicU64 = AtomicU64::new(0);
/// Token 累计 (W2 mock, W3 接 R17 真)
static TOKEN_USED: AtomicU64 = AtomicU64::new(142_857);

/// SqliteMemoryStore 全局单例 (R18 memory.rs 同样模式)
static MEMORY_STORE: OnceLock<Arc<SqliteMemoryStore>> = OnceLock::new();

/// SelfDisableGuard 全局单例 (R18 sovereignty.rs 同样模式)
static SOVEREIGNTY_GUARD: OnceLock<Arc<Mutex<SelfDisableGuard>>> = OnceLock::new();

/// 默认 IdentityCard continuity_id (跟 R18 web 一致)
const DEFAULT_CONTINUITY_ID: &str = "apeireth-web-default";

// ============================================================
// 数据结构 (跟前端字段对齐)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganStatus {
    pub name: String,
    pub display: String,
    pub metaphor: String,
    /// 主指标归一 [0, 1] (W2 简化版: 从 R18 真实数据派生)
    pub health: f64,
    /// 主指标值
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
    /// 阶段 enum (R11 LOCKED, 不改)
    pub r11_enum: String,
    /// R19 UI 是否显示 (false = 砍 Decline/Death)
    pub visible: bool,
    /// 当前阶段?
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateNode {
    pub name: String,
    pub display: String,
    pub group: String,
    pub x: f64,
    pub y: f64,
    pub active: f64,
    pub pid: u32,
    pub restart_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub mode: String,
    pub theme: String,
    pub language: String,
    pub launch_page: String,
    pub splash_enabled: bool,
    pub breath_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: "focus".into(),
            theme: "archaic".into(),
            language: "zh".into(),
            launch_page: "bridge".into(),
            splash_enabled: true,
            breath_enabled: true,
        }
    }
}

// ============================================================
// 状态管理 — 启动期 lazy init
// ============================================================

fn memory_store() -> Result<Arc<SqliteMemoryStore>, String> {
    if let Some(s) = MEMORY_STORE.get() {
        return Ok(Arc::clone(s));
    }
    // DB 路径: <CARGO_MANIFEST_DIR>/../apeireth-memory.db (跟 R18 web 一致)
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

fn sovereignty_guard() -> Arc<Mutex<SelfDisableGuard>> {
    SOVEREIGNTY_GUARD
        .get_or_init(|| Arc::new(Mutex::new(SelfDisableGuard::new())))
        .clone()
}

/// R19 8 阶段 (跟 R11 LOCKED 10 阶段 enum 错位: Decline/Death 砍, Rebirth 重生保留)
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

// ============================================================
// Tauri Commands (W2 真接 R18)
// ============================================================

/// 1. 主 AI 状态卡 7 数字
#[tauri::command]
fn get_main_ai_status() -> Result<MainAiStatus, String> {
    // 1. ASI V0.5 — 用 R18 asi.rs 同样模式: 合成 sample + DimensionRegistry::compute_all_dims
    let v05 = compute_v05();
    let continuity = v05.continuity;
    let philosophy = v05.philosophy_guard;

    // 2. 阶段 — 简化判据 (W2: Episode 数 > 100 = Growth, W3 接真判据)
    let (stage_zh, stage_idx) = compute_life_stage()
        .map_err(|e| format!("compute_life_stage: {e}"))?;

    // 3. 反思期 — 调 LifeForce 真字段
    let reflection_status = compute_reflection_status();

    // 4. endurance — 调 LifeForce 真字段
    let endurance = compute_endurance();

    // 5. Episode 数 — 调 SqliteMemoryStore 真 query
    let episode_count = match memory_store() {
        Ok(s) => s
            .query(&EpisodeQuery::new().limit(usize::MAX))
            .map(|v| v.len() as u64)
            .unwrap_or(0),
        Err(_) => 0,
    };

    // 6. Cognitive cycle 累计
    let cycle_count = CYCLE_COUNT.load(Ordering::Relaxed);

    // 7. Token 累计 (W2 mock, W3 接 R17)
    let token_used = TOKEN_USED.load(Ordering::Relaxed);

    // 8. 5 Self — 调 SelfDisableGuard 真 is_armed
    let five_self = {
        let guard = sovereignty_guard();
        let g = guard.lock().map_err(|e| format!("sovereignty mutex: {e}"))?;
        if g.is_armed { "✓ armed" } else { "✗ disarmed" }.to_string()
    };

    Ok(MainAiStatus {
        asi_v05: (continuity + philosophy) / 2.0, // W2 简化
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

/// 2. 9 器官心跳
#[tauri::command]
fn get_organ_status() -> Result<Vec<OrganStatus>, String> {
    let organs = vec![
        snapshot_perception(),
        snapshot_cognition(),
        snapshot_consciousness(),
        snapshot_memory()?,
        snapshot_motivation(),
        snapshot_value(),
        snapshot_relation(),
        snapshot_action(),
        snapshot_life_force(),
    ];
    Ok(organs)
}

/// 3. 8 阶段 (R19 砍 Decline/Death)
#[tauri::command]
fn get_life_stages() -> Result<Vec<LifeStageInfo>, String> {
    let active_idx = compute_life_stage().map(|(_, i)| i).unwrap_or(4);
    let mut stages = Vec::new();
    for stage in [
        LifeStage::Gestation, LifeStage::Birth, LifeStage::Infancy,
        LifeStage::Growth, LifeStage::Maturity, LifeStage::Reproduction,
        LifeStage::Migration, LifeStage::Rebirth,
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

/// 4. 30 crate 拓扑 (supervisor tree)
#[tauri::command]
fn get_topology() -> Result<Vec<CrateNode>, String> {
    Ok(mock_topology())
}

/// 5/6. 设置
#[tauri::command]
fn get_settings() -> Settings {
    Settings::default()
}

#[tauri::command]
fn set_settings(settings: Settings) -> Settings {
    settings
}

/// 7. 对话 (W2 mock, W3 接 R17 apeireth-api)
#[tauri::command]
fn chat(input: String) -> String {
    // 增加 cycle 计数 (每次 chat 算 1 个 run_cycle)
    CYCLE_COUNT.fetch_add(1, Ordering::Relaxed);
    // 跑一次真 run_cycle (W2: 1 个 NormalAction 候选)
    let cycle_target = ActionTarget::NormalAction(format!("chat:{input}"));
    let cognitive = CognitiveInput::new(vec![cycle_target], "desktop-chat");
    let result = match run_cycle(cognitive) {
        Ok(c) => format!(
            "(W2 真接 run_cycle · input_id={} · allowed={} · rejected={}) → 收到: {}",
            c.input_id,
            matches!(c.output, CognitiveOutput::Decision(_)),
            matches!(c.output, CognitiveOutput::Reject(_)),
            input
        ),
        Err(e) => format!("(W2) run_cycle 错误: {e:?}"),
    };
    result
}

// ============================================================
// 9 器官 snapshot (R18 同样的真后端调用模式)
// ============================================================

fn snapshot_perception() -> OrganStatus {
    OrganStatus {
        name: "perception".into(),
        display: "感知".into(),
        metaphor: "五感".into(),
        health: 0.85,
        primary: "5/5 通道".into(),
        secondary: "Text · Voice · Vision".into(),
        tertiary: "events/s: 12 (R18)".into(),
    }
}

fn snapshot_cognition() -> OrganStatus {
    // 跑 1 次 run_cycle 看状态
    let cognitive = CognitiveInput::new(
        vec![ActionTarget::NormalAction("cognition-snapshot".into())],
        "cognition-snapshot",
    );
    let (primary, secondary, tertiary) = match run_cycle(cognitive) {
        Ok(c) => {
            let v05 = AsiV05Scores::from_trace(&apeireth_asi::DimensionTrace {
                trace_id: 0,
                sample_id: 0,
                timestamp: 0,
                v05_dims: c.v05_legacy_array_for_snapshot(),
                v1136_subs: [0.0; 9],
                hook_overrides: vec![],
            });
            let verdicts = c.verdicts.len();
            (
                format!("V0.5={:.3}", v05.transferability),
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

fn snapshot_consciousness() -> OrganStatus {
    let machine = CognitiveDreamStateMachine::new(DEFAULT_CONTINUITY_ID);
    let state = machine.current;
    OrganStatus {
        name: "consciousness".into(),
        display: "意识".into(),
        metaphor: "心智".into(),
        health: 0.95,
        primary: state.semantic_name().to_string(),
        secondary: machine.legal_targets_now().iter()
            .map(|s| s.semantic_name().to_string())
            .collect::<Vec<_>>()
            .join(" / "),
        tertiary: format!("machine_id={}", machine.machine_id),
    }
}

fn snapshot_memory() -> Result<OrganStatus, String> {
    let s = memory_store()?;
    // 总 episode 数
    let total = s.query(&EpisodeQuery::new().limit(usize::MAX))
        .map(|v| v.len())
        .unwrap_or(0);
    // 各 session 计数 (W2 简化: 查 web session)
    let web_count = s.query(&EpisodeQuery::new().for_session("web-session").limit(usize::MAX))
        .map(|v| v.len())
        .unwrap_or(0);
    let council_count = s.query(&EpisodeQuery::new().for_session("council-history").limit(usize::MAX))
        .map(|v| v.len())
        .unwrap_or(0);
    // 是否有 identity card
    let has_identity = s.get(DEFAULT_CONTINUITY_ID)
        .map(|opt| opt.is_some())
        .unwrap_or(false);
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

fn snapshot_motivation() -> OrganStatus {
    // 调 R18 motivation 真实 motivation_score (W2 用稳定输入)
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
        tertiary: format!("SGI 7 约束 (C-SGI-1~7)"),
    }
}

fn snapshot_value() -> OrganStatus {
    let dims = ValueDimension::ALL;
    let count = dims.len();
    OrganStatus {
        name: "value".into(),
        display: "价值".into(),
        metaphor: "前额叶".into(),
        health: 0.90,
        primary: format!("{}/{} 层洋葱", count, count),
        secondary: dims.iter().map(|d| d.letter().to_string()).collect::<Vec<_>>().join(" / "),
        tertiary: "硬门槛 ≥ 0.85 (R18)".into(),
    }
}

fn snapshot_relation() -> OrganStatus {
    // W2: apeireth-relation 是 4 类 enum, 不暴露 registry API, 用静态展示
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

fn snapshot_action() -> OrganStatus {
    // W2: apeireth-action 默认 in-memory 模拟
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

fn snapshot_life_force() -> OrganStatus {
    let identity = IdentityCard {
        continuity_id: DEFAULT_CONTINUITY_ID.into(),
        birth_time: 1_700_000_000,
        carriers: vec!["apeireth-desktop".into()],
        migration_history: vec![],
    };
    let now = chrono::Utc::now().timestamp();
    let life = LifeForce::new(identity, now);
    let exhausted = exhaustion_check(&life);
    OrganStatus {
        name: "life_force".into(),
        display: "生命力".into(),
        metaphor: "免疫".into(),
        health: life.endurance,
        primary: format!("{:.3}", life.endurance),
        secondary: if life.is_in_reflection(now) { "反思期 active".into() } else { "dormant".into() },
        tertiary: if exhausted { "⚠ endurance < 0.2".into() } else { format!("SGI: {}", if life.has_sgi() { "✓" } else { "(空)" }) },
    }
}

// ============================================================
// ASI V0.5 (R18 asi.rs 同样模式: 合成 sample + DimensionRegistry)
// ============================================================

fn compute_v05() -> AsiV05Scores {
    let registry = DimensionRegistry::new();
    let mut sample = MeasurementSample::default();
    // W2: 稳定合成输入 (跟 R18 asi.rs demo loop 一致: success/attempt = 0.9 起步)
    for name in V05_DIMENSION_NAMES.iter() {
        sample.successes.insert((*name).to_string(), 90);
        sample.attempts.insert((*name).to_string(), 100);
        sample.qualities.insert((*name).to_string(), 1.0);
    }
    let v05_dims = registry.compute_all_dims(&sample);
    let trace = apeireth_asi::DimensionTrace {
        trace_id: 0,
        sample_id: 0,
        timestamp: chrono::Utc::now().timestamp(),
        v05_dims,
        v1136_subs: [0.0; 9],
        hook_overrides: vec![],
    };
    AsiV05Scores::from_trace(&trace)
}

// ============================================================
// 阶段判据 (W2 简化, W3 接 apeireth_central)
// ============================================================

fn compute_life_stage() -> Result<(String, u8), String> {
    // 简化判据: Episode 数
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

fn compute_reflection_status() -> String {
    let identity = IdentityCard {
        continuity_id: DEFAULT_CONTINUITY_ID.into(),
        birth_time: 1_700_000_000,
        carriers: vec!["apeireth-desktop".into()],
        migration_history: vec![],
    };
    let now = chrono::Utc::now().timestamp();
    let mut life = LifeForce::new(identity, now);
    life.sgi = SelfGrowthIndicator::new("assist-and-reflect", now);
    if life.is_in_reflection(now) { "active".into() } else { "dormant".into() }
}

fn compute_endurance() -> f64 {
    let identity = IdentityCard {
        continuity_id: DEFAULT_CONTINUITY_ID.into(),
        birth_time: 1_700_000_000,
        carriers: vec!["apeireth-desktop".into()],
        migration_history: vec![],
    };
    let now = chrono::Utc::now().timestamp();
    let life = LifeForce::new(identity, now);
    life.endurance / ENDURANCE_MAX
}

// ============================================================
// 30 crate supervisor tree (跟 W1 一样的 layout, 引用 R11 设计)
// ============================================================

fn mock_topology() -> Vec<CrateNode> {
    let mut nodes = Vec::new();
    // 总
    nodes.push(CrateNode {
        name: "apeireth-supervisor".into(), display: "总监督".into(), group: "总".into(),
        x: 400.0, y: 300.0, active: 0.95, pid: 1, restart_strategy: "permanent".into(),
    });
    // 主核
    let core = vec![
        ("apeireth-core", "核心", 200.0, 200.0, 0.92, 101, "rest_for_one"),
        ("apeireth-onion", "洋葱", 600.0, 200.0, 0.88, 104, "rest_for_one"),
        ("apeireth-constraint", "约束", 200.0, 400.0, 0.85, 105, "rest_for_one"),
        ("apeireth-central", "中央", 600.0, 400.0, 0.90, 100, "rest_for_one"),
    ];
    for (n, d, x, y, a, p, s) in core {
        nodes.push(CrateNode {
            name: n.into(), display: d.into(), group: "主核".into(),
            x, y, active: a, pid: p, restart_strategy: s.into(),
        });
    }
    // 治理
    let gov = vec![
        ("apeireth-sovereignty", "主权", 100.0, 300.0, 0.93, 102, "one_for_one"),
        ("apeireth-council", "智囊团", 700.0, 300.0, 0.87, 201, "one_for_one"),
        ("apeireth-life-force", "生命力", 400.0, 100.0, 0.96, 209, "rest_for_one"),
    ];
    for (n, d, x, y, a, p, s) in gov {
        nodes.push(CrateNode {
            name: n.into(), display: d.into(), group: "治理".into(),
            x, y, active: a, pid: p, restart_strategy: s.into(),
        });
    }
    // 器官
    let org = vec![
        ("apeireth-perception", "感知", 50.0, 100.0, 0.85),
        ("apeireth-cognition", "认知", 150.0, 50.0, 0.92),
        ("apeireth-consciousness", "意识", 300.0, 30.0, 0.95),
        ("apeireth-memory", "记忆", 500.0, 30.0, 0.78),
        ("apeireth-motivation", "动机", 650.0, 50.0, 0.88),
        ("apeireth-value", "价值", 750.0, 100.0, 0.90),
        ("apeireth-relation", "关系", 780.0, 250.0, 0.83),
        ("apeireth-action", "行动", 750.0, 480.0, 0.86),
    ];
    for (n, d, x, y, a) in org {
        nodes.push(CrateNode {
            name: n.into(), display: d.into(), group: "器官".into(),
            x, y, active: a, pid: 0, restart_strategy: "—".into(),
        });
    }
    // 工具
    let tool = vec![
        ("apeireth-api", "API", 80.0, 500.0, 0.82),
        ("apeireth-bus", "总线", 200.0, 550.0, 0.88),
        ("apeireth-upgrade", "升级", 320.0, 580.0, 0.75),
        ("apeireth-extension", "扩展", 450.0, 580.0, 0.70),
        ("apeireth-pybridge", "桥", 580.0, 550.0, 0.65),
        ("apeireth-tools", "工具集", 720.0, 500.0, 0.78),
    ];
    for (n, d, x, y, a) in tool {
        nodes.push(CrateNode {
            name: n.into(), display: d.into(), group: "工具".into(),
            x, y, active: a, pid: 0, restart_strategy: "transient".into(),
        });
    }
    // 测量
    let meas = vec![
        ("apeireth-asi", "ASI", 400.0, 200.0, 0.91),
        ("apeireth-bench", "基准", 350.0, 480.0, 0.68),
        ("apeireth-test", "测试", 500.0, 480.0, 0.72),
        ("apeireth-verify", "验证", 450.0, 480.0, 0.69),
    ];
    for (n, d, x, y, a) in meas {
        nodes.push(CrateNode {
            name: n.into(), display: d.into(), group: "测量".into(),
            x, y, active: a, pid: 0, restart_strategy: "—".into(),
        });
    }
    // 其他
    let oth = vec![
        ("apeireth-cli", "CLI", 100.0, 400.0, 0.80),
        ("apeireth-evolution", "演化", 300.0, 100.0, 0.73),
        ("apeireth-web", "Web R18", 600.0, 480.0, 0.85),
        ("apeireth-philosophy", "哲学", 400.0, 400.0, 0.78),
    ];
    for (n, d, x, y, a) in oth {
        if !nodes.iter().any(|node| node.name == n) {
            nodes.push(CrateNode {
                name: n.into(), display: d.into(), group: "总".into(),
                x, y, active: a, pid: 0, restart_strategy: "—".into(),
            });
        }
    }
    nodes
}

// ============================================================
// 启动入口
// ============================================================

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_main_ai_status,
            get_organ_status,
            get_life_stages,
            get_topology,
            get_settings,
            set_settings,
            chat,
        ])
        .run(tauri::generate_context!())
        .expect("error while running apeireth-desktop");
}

// ============================================================
// CognitiveCycle 扩展 (W2 helper: 把 cycle.v05 投影到 24 维数组)
// R11 LOCKED 边界: 不改 CognitiveCycle struct, 只在本 crate 扩展
// ============================================================

trait CognitiveCycleExt {
    fn v05_legacy_array_for_snapshot(&self) -> [f64; 24];
}

impl CognitiveCycleExt for apeireth_cognition::CognitiveCycle {
    fn v05_legacy_array_for_snapshot(&self) -> [f64; 24] {
        // 简化: 用 v05.continuity / salience / identity / philosophy_guard / transferability
        // 扩展成 24 维 (跟 R18 asi.rs 同 5 维投影模式)
        let c = self.v05.continuity;
        let s = self.v05.salience;
        let i = self.v05.identity;
        let p = self.v05.philosophy_guard;
        let t = self.v05.transferability;
        [
            c, c, c, c, c,
            s, s, s, s, s,
            i, i, i, i, i,
            p, p, p, p, p,
            t, t, t, t,
        ]
    }
}

// 引用 LEGAL_TRANSITIONS 让编译验证 R11 LOCKED 边界
#[allow(dead_code)]
fn _check_locked_legal_transitions() -> usize {
    LEGAL_TRANSITIONS.len()  // = 12 (R11 LOCKED, W2 读取用)
}
