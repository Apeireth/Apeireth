//! apeireth-life-force: 生命力维 (A13 落点 — R14 Phase 4)
//!
//! **维度 1 生命力 (穿透维度, 纵向)** — 立体架构 v2 修正 #5+#6
//!
//! 依据:
//! - `docs/stage4/stage4-thinking-document.md` §2/§3: `LifeForce` trait + `LifeForce` struct
//! - `docs/stage1/inspiration-stage1-2026-07-30.md` §21.4: SGI = **Single-field Goal Identity** (单字段)
//! - `docs/stage3-blueprints/03-decision-flow.md` §3.10: 反思期 = 生命力维度节点 (不是横切)
//! - `docs/architecture-v3-aircraft-carrier.md` §2.1: 生命力维度 4 子组件 (反思/内稳态/反馈/涌现)
//!
//! **核心 API**:
//! - `LifeForce` struct — 持续力 (endurance) + 反思期计时 (reflection_period) + SGI 单字段
//! - `ReflectionPeriod` trait — 72h 默认时长, 可由具体实现覆盖
//! - `SelfGrowthIndicator` (SGI) — 单字段目标身份 (按 v8 修正, 不要拆成多个分散字段)
//! - 3 核心触发函数: `reflection_trigger` / `exhaustion_check` / `recovery_start`
//!
//! **不修改承诺 (LOCKED)**:
//! - ❌ 不修改 apeireth-core 任何已实装类型
//! - ❌ 不修改 R11 baseline 三值
//! - ❌ 不碰 apeireth-legacy/
//! - ❌ 不绕过 L0 HA / V1+V2+V3 AND 门
//! - ❌ **不依赖** 已 DEPRECATED 的 `apeireth-philosophy` (替代说明见 crate 文档),
//!     复用本 crate 自带的 `ReflectionPeriod` trait — 避免触碰 LOCKED 弃用声明.
//!
//! **诚实登记 (ponytail 简化)**:
//! - 反思期节点作为"生命力维度节点"实现, 接入电子环的细节留给 A18/A19 深化
//! - 反馈循环 (feedback_loop) 与涌现观察 (observe_emergence) 在此版本以最小数据形态提供

#![deny(unsafe_code)]

use apeireth_core::IdentityCard;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// R22 ST-A2.1 — 反思期 4 阶段状态机 + 周期触发调度器
pub mod reflection_cycle;
// R22 ST-A2.3 — 涌现能力识别 (Emergence Detection)
pub mod emergence;
// R173 ST-B2.1 — bridge 2: consciousness -> life-force
pub mod consciousness_bridge;
// R176: bridge 2 Kani proofs
mod bridge_kani_proofs;
// R177: organ invariants (10 tests + 2 Kani proofs)
mod organ_kani_proofs;

// ============================================
// 1. SGI (SelfGrowthIndicator) — 单字段目标身份
// ============================================

/// SGI (SelfGrowthIndicator) — **单字段**目标身份 (Single-field Goal Identity)
///
/// 按 `docs/stage1/inspiration-stage1-2026-07-30.md` §21.4 "SGI (Single-field Goal Identity)":
/// 目标身份**只占一个字段** `goal`, 不拆成多个分散字段 (avoid 三个属性散落四处).
///
/// 按 v8 修正: SGI = { goal, last_updated } — `goal` 单字段表达主 AI 当前的"我是谁 / 我要做什么"，
/// `last_updated` 仅作时间戳记录, 本身不参与"身份判定"语义.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfGrowthIndicator {
    /// 目标身份 (单字段) — 主 AI 当前的"我是谁 / 我要做什么"
    pub goal: String,
    /// 最近一次目标更新时间 (epoch seconds). 不参与身份判定, 仅作时间戳.
    pub last_updated: i64,
}

impl SelfGrowthIndicator {
    /// 构造 SGI (单字段).
    pub fn new(goal: impl Into<String>, now: i64) -> Self {
        Self {
            goal: goal.into(),
            last_updated: now,
        }
    }

    /// 更新 SGI (单字段) — 同时刷新时间戳.
    pub fn update(&mut self, new_goal: impl Into<String>, now: i64) {
        self.goal = new_goal.into();
        self.last_updated = now;
    }

    /// SGI 是否为空 (无目标身份).
    pub fn is_empty(&self) -> bool {
        self.goal.trim().is_empty()
    }
}

// ============================================
// 2. ReflectionPeriod — 反思期 trait (72h 默认)
// ============================================

/// 反思期 trait — 72h 默认时长 (3 天冷静期), 可由具体实现覆盖.
///
/// 依据:
/// - `docs/stage1/inspiration-stage1-2026-07-30.md` §20.1 M5: 反思期 ≥ 7 天
/// - `docs/stage1/inspiration-stage1-2026-07-30.md` §20.4 L5: 反思期 ≥ 7 天
/// - 任务约定 (本 PR): 72h (3 天) 最小单位 (P5 落地版, 后续 L5 真测时由评审扩到 7 天)
///
/// 注意: 72h ≠ 7 天, 此处使用任务约定的 72h 默认值. 升级到 L5 真测版本时
/// 需 [PONYTAIL-MIN] 改为 7 天 (官方接口保持 Default::default_duration_secs = 72*3600).
pub trait ReflectionPeriod {
    /// 反思期总时长 (秒).
    fn duration_secs(&self) -> i64;

    /// 反思期是否已结束 (started_at + duration_secs <= now).
    fn is_concluded(&self, started_at: i64, now: i64) -> bool {
        now >= started_at + self.duration_secs()
    }

    /// 反思期剩余秒数 (`0` 表示已结束).
    fn remaining_secs(&self, started_at: i64, now: i64) -> i64 {
        let end = started_at + self.duration_secs();
        (end - now).max(0)
    }

    /// 默认 72h 反思期 (3 天冷静期).
    fn default_duration_secs() -> i64 {
        72 * 3600
    }
}

/// 反思期状态 — 持久化数据 (started_at + identity continuity_id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionPeriodState {
    /// 反思期起始时间戳 (epoch seconds, 0 表示未启动)
    pub started_at: i64,
    /// 关联主体 continuity_id (跨载体同 ID — 接入 IdentityCard)
    pub continuity_id: String,
    /// 反思期时长 (由 trait 决定, 持久化以便审计)
    pub duration_secs: i64,
}

impl ReflectionPeriodState {
    /// 构造"未启动"反思期状态.
    pub fn dormant(continuity_id: impl Into<String>) -> Self {
        Self {
            started_at: 0,
            continuity_id: continuity_id.into(),
            duration_secs: <StandardReflectionPeriod as ReflectionPeriod>::default_duration_secs(),
        }
    }

    /// 启动反思期 (返回启动后的状态).
    pub fn start(&self, now: i64) -> Self {
        Self {
            started_at: now,
            continuity_id: self.continuity_id.clone(),
            duration_secs: self.duration_secs,
        }
    }

    /// 反思期是否激活 (已启动且未结束).
    pub fn is_active(&self, now: i64) -> bool {
        self.started_at > 0 && now < self.started_at + self.duration_secs
    }
}

/// 标准 72h 反思期实现 (默认实现).
#[derive(Debug, Clone, Copy, Default)]
pub struct StandardReflectionPeriod;

impl ReflectionPeriod for StandardReflectionPeriod {
    fn duration_secs(&self) -> i64 {
        Self::default_duration_secs()
    }
}

// ============================================
// 3. 持续力 (Endurance) — 生命力维度的"续航"指标
// ============================================

/// 持续力最小值 (0.0 = 完全耗竭).
pub const ENDURANCE_MIN: f64 = 0.0;
/// 持续力最大值 (1.0 = 满续航).
pub const ENDURANCE_MAX: f64 = 1.0;
/// 持续力阈值: 低于此值视为"耗竭" (exhaustion).
pub const ENDURANCE_EXHAUSTION_THRESHOLD: f64 = 0.2;
/// 持续力目标值 (recovery 完成后应达到).
pub const ENDURANCE_RECOVERY_TARGET: f64 = 0.8;

// ============================================
// 4. 反思期触发原因 (依据 §3.10 M1/M2/M3)
// ============================================

/// 反思期触发原因 (依据 stage3-blueprints §3.10 M1/M2/M3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReflectionTrigger {
    /// M1 异常行为自动回流 — 主 AI 自我检测偏离哲学锚.
    AnomalyDetected(String),
    /// M2 升级后强制审计 — OTA 升级完成后 30min 内强制审计.
    PostUpgradeAudit,
    /// M3 日常反思周报 — 每周聚合主 AI 行为.
    WeeklyReport,
}

// ============================================
// 5. LifeForce — 穿透维度的核心 struct
// ============================================

/// 生命力维度 — 穿透整个架构的纵向维度.
///
/// 字段映射:
/// - `endurance`     — 持续力 (续航 0.0-1.0)
/// - `reflection`    — 反思期计时状态 (per R, 接入 IdentityCard continuity_id)
/// - `sgi`           — SGI 单字段 (Single-field Goal Identity, per v8 修正)
/// - `identity`      — 主体连续性 ID (复用 apeireth-core IdentityCard, 不重写)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeForce {
    /// 持续力 (续航) — [0.0, 1.0].
    pub endurance: f64,
    /// 反思期状态 (计时 + 主体绑定).
    pub reflection: ReflectionPeriodState,
    /// SGI 单字段目标身份 (按 v8 修正, 不拆为多个字段).
    pub sgi: SelfGrowthIndicator,
    /// 主体连续性 ID (复用 apeireth-core IdentityCard, 不重写).
    pub identity: IdentityCard,
}

impl LifeForce {
    /// 构造新生命力 (默认持续力 1.0, SGI 空, 反思期 dormant).
    pub fn new(identity: IdentityCard, now: i64) -> Self {
        let continuity_id = identity.continuity_id.clone();
        Self {
            endurance: ENDURANCE_MAX,
            reflection: ReflectionPeriodState::dormant(continuity_id),
            sgi: SelfGrowthIndicator::new("", now),
            identity,
        }
    }

    /// SGI 单字段是否已设置 (有目标身份).
    pub fn has_sgi(&self) -> bool {
        !self.sgi.is_empty()
    }

    /// 反思期是否激活中.
    pub fn is_in_reflection(&self, now: i64) -> bool {
        self.reflection.is_active(now)
    }
}

// ============================================
// 6. 错误类型
// ============================================

/// 生命周期能量操作错误.
#[derive(Debug, Error)]
pub enum LifeForceError {
    /// 持续力数值越界.
    #[error("endurance out of range [{min}, {max}]: got {value}")]
    EnduranceOutOfRange {
        /// 越界值.
        value: f64,
        /// 允许最小值.
        min: f64,
        /// 允许最大值.
        max: f64,
    },
    /// 反思期与主体不一致.
    #[error("reflection continuity_id mismatch: expected {expected}, got {actual}")]
    ContinuityMismatch {
        /// 期望的 continuity_id.
        expected: String,
        /// 实际的 continuity_id.
        actual: String,
    },
    /// SGI 单字段为空.
    #[error("SGI single-field goal is empty")]
    SgiEmpty,
}

// ============================================
// 7. 核心触发函数 (3 个核心 + 2 个辅助)
// ============================================

/// **核心 1: 反思期触发** — 启动反思期, 校验 continuity_id + SGI 单字段非空.
///
/// 完整逻辑 (R14-D6-B B5 列举):
/// - M1 异常行为自动回流 → AnomalyDetected
/// - M2 升级后强制审计 → PostUpgradeAudit
/// - M3 日常反思周报 → WeeklyReport
///
/// 触发后: 反思期状态变 active, 持续力小幅下降 (反映"反思消耗").
pub fn reflection_trigger(
    life: &mut LifeForce,
    trigger: ReflectionTrigger,
    now: i64,
) -> Result<ReflectionPeriodState, LifeForceError> {
    // 校验: continuity_id 必须一致
    if life.reflection.continuity_id != life.identity.continuity_id {
        return Err(LifeForceError::ContinuityMismatch {
            expected: life.identity.continuity_id.clone(),
            actual: life.reflection.continuity_id.clone(),
        });
    }
    // 校验: SGI 单字段非空 (思考 / 行动需有目标身份)
    if life.sgi.is_empty() {
        return Err(LifeForceError::SgiEmpty);
    }
    // 启动反思期
    life.reflection = life.reflection.start(now);
    // 反思消耗: 持续力下降 0.1 (但不低于 0)
    life.endurance = (life.endurance - 0.1).max(ENDURANCE_MIN);
    // 反射 SGI 时间戳
    life.sgi.last_updated = now;
    // 记录触发原因 (写反思日志的最小骨架 — A18/A19 深化)
    let _ = trigger; // 当前版本不持久化 trigger, 保留字段供扩展
    Ok(life.reflection.clone())
}

/// **核心 2: 耗竭检查** — 持续力是否低于阈值.
pub fn exhaustion_check(life: &LifeForce) -> bool {
    life.endurance < ENDURANCE_EXHAUSTION_THRESHOLD
}

/// **核心 3: 恢复启动** — 启动恢复, 持续力回升到目标值.
pub fn recovery_start(life: &mut LifeForce) -> f64 {
    life.endurance = ENDURANCE_RECOVERY_TARGET;
    life.endurance
}

/// **辅助 4: 校验持续力数值范围** — 用于外部输入校验.
pub fn validate_endurance(value: f64) -> Result<f64, LifeForceError> {
    if !(ENDURANCE_MIN..=ENDURANCE_MAX).contains(&value) {
        return Err(LifeForceError::EnduranceOutOfRange {
            value,
            min: ENDURANCE_MIN,
            max: ENDURANCE_MAX,
        });
    }
    Ok(value)
}

/// **辅助 5: 反思期进度** — 返回 [0.0, 1.0], 0=刚启动, 1.0=已结束.
pub fn reflection_progress(life: &LifeForce, now: i64) -> f64 {
    if life.reflection.started_at == 0 {
        return 0.0;
    }
    let elapsed = (now - life.reflection.started_at).max(0) as f64;
    let total = life.reflection.duration_secs as f64;
    if total <= 0.0 {
        return 1.0;
    }
    (elapsed / total).clamp(0.0, 1.0)
}

// ============================================
// 8. 单元测试 (5+ tests)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_identity() -> IdentityCard {
        IdentityCard {
            continuity_id: "did:apeireth:test-001".to_string(),
            birth_time: 1_700_000_000,
            carriers: vec!["carrier-A".to_string()],
            migration_history: vec![],
        }
    }

    #[test]
    fn sgi_new_sets_single_field_and_timestamp() {
        let sgi = SelfGrowthIndicator::new("assist-and-reflect", 1_700_000_000);
        assert_eq!(sgi.goal, "assist-and-reflect");
        assert_eq!(sgi.last_updated, 1_700_000_000);
        assert!(!sgi.is_empty());
    }

    #[test]
    fn sgi_is_empty_when_goal_blank() {
        let sgi = SelfGrowthIndicator::new("   ", 0);
        assert!(sgi.is_empty());
    }

    #[test]
    fn sgi_update_refreshes_goal_and_timestamp() {
        let mut sgi = SelfGrowthIndicator::new("old", 100);
        sgi.update("new", 200);
        assert_eq!(sgi.goal, "new");
        assert_eq!(sgi.last_updated, 200);
    }

    #[test]
    fn reflection_period_default_72h() {
        let p = StandardReflectionPeriod;
        assert_eq!(p.duration_secs(), 72 * 3600);
    }

    #[test]
    fn reflection_period_is_concluded_after_72h() {
        let p = StandardReflectionPeriod;
        let started = 1_700_000_000;
        assert!(!p.is_concluded(started, started + 71 * 3600));
        assert!(p.is_concluded(started, started + 72 * 3600));
        assert!(p.is_concluded(started, started + 100 * 3600));
    }

    #[test]
    fn reflection_period_remaining_secs_clamps_to_zero() {
        let p = StandardReflectionPeriod;
        let started = 1_700_000_000;
        assert_eq!(p.remaining_secs(started, started), 72 * 3600);
        assert_eq!(p.remaining_secs(started, started + 72 * 3600), 0);
        assert_eq!(p.remaining_secs(started, started + 100 * 3600), 0);
    }

    #[test]
    fn life_force_new_starts_at_full_endurance() {
        let life = LifeForce::new(make_identity(), 1_700_000_000);
        assert_eq!(life.endurance, ENDURANCE_MAX);
        assert!(!life.is_in_reflection(1_700_000_000));
        assert!(!life.has_sgi());
    }

    #[test]
    fn reflection_trigger_starts_period_and_consumes_endurance() {
        let mut life = LifeForce::new(make_identity(), 1_700_000_000);
        life.sgi = SelfGrowthIndicator::new("assist", 1_700_000_000);
        let now = 1_700_000_100;
        let state = reflection_trigger(&mut life, ReflectionTrigger::WeeklyReport, now)
            .expect("trigger must succeed");
        assert!(state.is_active(now));
        assert!(life.endurance < ENDURANCE_MAX);
    }

    #[test]
    fn reflection_trigger_rejects_empty_sgi() {
        let mut life = LifeForce::new(make_identity(), 1_700_000_000);
        let res = reflection_trigger(
            &mut life,
            ReflectionTrigger::PostUpgradeAudit,
            1_700_000_000,
        );
        assert!(matches!(res, Err(LifeForceError::SgiEmpty)));
    }

    #[test]
    fn reflection_trigger_rejects_mismatched_continuity() {
        let mut life = LifeForce::new(make_identity(), 1_700_000_000);
        life.sgi = SelfGrowthIndicator::new("assist", 1_700_000_000);
        // 手动篡改 reflection.continuity_id 以模拟"不一致"
        life.reflection.continuity_id = "did:apeireth:other".to_string();
        let res = reflection_trigger(
            &mut life,
            ReflectionTrigger::AnomalyDetected("phl01_violation".into()),
            1_700_000_000,
        );
        assert!(matches!(
            res,
            Err(LifeForceError::ContinuityMismatch { .. })
        ));
    }

    #[test]
    fn exhaustion_check_detects_low_endurance() {
        let mut life = LifeForce::new(make_identity(), 1_700_000_000);
        life.endurance = 0.1;
        assert!(exhaustion_check(&life));
        life.endurance = 0.5;
        assert!(!exhaustion_check(&life));
    }

    #[test]
    fn recovery_start_brings_endurance_to_target() {
        let mut life = LifeForce::new(make_identity(), 1_700_000_000);
        life.endurance = 0.05;
        let after = recovery_start(&mut life);
        assert_eq!(after, ENDURANCE_RECOVERY_TARGET);
        assert!(!exhaustion_check(&life));
    }

    #[test]
    fn validate_endurance_rejects_out_of_range() {
        assert!(validate_endurance(-0.1).is_err());
        assert!(validate_endurance(1.1).is_err());
        assert!(validate_endurance(0.5).is_ok());
    }

    #[test]
    fn reflection_progress_starts_at_zero_and_clamps_to_one() {
        let mut life = LifeForce::new(make_identity(), 1_700_000_000);
        life.sgi = SelfGrowthIndicator::new("assist", 1_700_000_000);
        // 未启动
        assert_eq!(reflection_progress(&life, 1_700_000_000), 0.0);
        // 启动后 36h
        let now = 1_700_000_000 + 36 * 3600;
        reflection_trigger(&mut life, ReflectionTrigger::WeeklyReport, 1_700_000_000)
            .expect("trigger");
        let p = reflection_progress(&life, now);
        assert!((p - 0.5).abs() < 0.001);
        // 超过 72h — clamp 到 1.0
        let way_later = 1_700_000_000 + 100 * 3600;
        assert!((reflection_progress(&life, way_later) - 1.0).abs() < 0.001);
    }
}


pub use crate::{
    emergence::{
        EmergenceDetector, EmergenceError, EmergenceReport, EmergenceSignal,
        EmergenceSignalType,
    },
    reflection_cycle::{
        ReflectionCycleError, ReflectionCycleEvent, ReflectionCycleScheduler,
        ReflectionPhase,
    },
};

