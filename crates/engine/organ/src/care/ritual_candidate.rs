//! 仪式候选源 (SG-22 仪式内核 → care 调度): 把 [`plan_rituals`] 的到点决策
//! 转成 [`CareCandidate`], 再由 care 管道**统一裁决** (勿扰 / 专注 / 频率 / 预算)。
//!
//! # 转换规则
//!
//! | [`DueKind`] | 候选 | 优先级 |
//! |---|---|---|
//! | [`DueKind::Due`] | 产出 | [`RitualCandidatePriority::High`] |
//! | [`DueKind::DueSoft`] | 产出 (低优先) | [`RitualCandidatePriority::Low`] |
//! | [`DueKind::NotDue`] / [`DueKind::Muted`] | **不产出** | — |
//!
//! - **不绕过纪律**: 本模块只产候选, 不投递、不看勿扰/频率/预算; 是否发起由
//!   [`super::engine::CareEngine::poll_bio_candidate`] 走既有纪律管道裁决。
//! - **不生成正文**: `material` = [`RitualDecision::suggested_message_key`]
//!   (稳定文案键, 如 `ritual.morning_check_in.due`), 正文与人格化措辞由上层渲染。
//! - **优先级不新增字段**: 既有 [`CareCandidate`] 结构不动; 优先级写进 `trigger`
//!   的可读描述 (`优先级 high` / `优先级 low`), 同批候选里 `Due` 排在 `DueSoft`
//!   前, 同优先级内保持 [`crate::ritual::RitualKind`] 的声明顺序 (确定性)。
//! - **默认关**: [`super::CareConfig::bio_candidates_enabled`] 默认 `false`;
//!   关闭时 [`ritual_candidates_for`] 恒返回空 (W3.2 装配面才允许打开)。
//! - **时间注入**: `now_ms` 由调用方给; 不读系统时钟、不做 I/O、不调 LLM。
//!
//! # 0 装诚实 (IMPLEMENTED, 未装配)
//!
//! 候选源与纪律管道入口已实现并有测试; **未接 runtime-assembly** (W3.2 才注入
//! replay 数据并打开开关), **默认关**。因此本模块目前不是线上行为, 不声称
//! PRODUCTION WIRED / DEFAULT ENABLED。

use std::cmp::Reverse;

use super::engine::CareCandidate;
use super::reminder::CareTier;
use super::CareConfig;
use crate::ritual::{plan_rituals, DueKind, RitualConfig, RitualDecision, RitualState};

/// 仪式候选优先级: `Due` 高于 `DueSoft` (只降级不升级, 与仪式内核同向)。
///
/// 不新增 [`CareCandidate`] 字段 (既有结构不动); 优先级进 `trigger` 的可读描述,
/// 并决定同批候选的输出顺序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RitualCandidatePriority {
    /// 软提醒 (`DueSoft`): 已过宽限, 低优先 (只提醒一次, 不追问)。
    Low,
    /// 到点 (`Due`): 仍在宽限内或首次建立, 高优先。
    High,
}

impl RitualCandidatePriority {
    /// 稳定标签 (进候选 `trigger`; 不参与判定)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
        }
    }

    /// 决策类型 → 候选优先级; `NotDue` / `Muted` 不产候选 (`None`)。
    pub const fn of(due: DueKind) -> Option<Self> {
        match due {
            DueKind::Due => Some(Self::High),
            DueKind::DueSoft => Some(Self::Low),
            DueKind::NotDue | DueKind::Muted => None,
        }
    }
}

/// 把仪式内核的到点决策转成 care 候选 (纯函数、确定性、时间注入)。
///
/// 只保留 `Due` / `DueSoft`: `Muted` / `NotDue` 不产候选。输出顺序 = 优先级降序
/// (`Due` 在前), 同优先级内保持 [`crate::ritual::RitualKind`] 声明顺序, 与 `states`
/// 输入顺序无关。
///
/// 候选的 `rule_id` / `material` 都是 [`RitualDecision::suggested_message_key`]
/// (稳定文案键); `due_ms` = 注入的 `now_ms` (决策时刻); `tier` = [`CareTier::Ritual`]。
///
/// `now_ms` 为负 (时钟异常) 时按 epoch 0 求值 (防御性降级, 不 panic); 候选的
/// `due_ms` 原样保留调用方注入值。
#[must_use]
pub fn ritual_candidates(
    now_ms: i64,
    states: &[RitualState],
    config: &RitualConfig,
) -> Vec<CareCandidate> {
    let now_u64 = u64::try_from(now_ms).unwrap_or(0);
    let mut actionable: Vec<(RitualCandidatePriority, RitualDecision)> =
        plan_rituals(now_u64, states, config)
            .into_iter()
            .filter_map(|decision| {
                RitualCandidatePriority::of(decision.due).map(|priority| (priority, decision))
            })
            .collect();
    // 稳定排序: 高优先在前, 同优先级保持声明顺序 (确定性)。
    actionable.sort_by_key(|(priority, _decision)| Reverse(*priority));
    actionable
        .into_iter()
        .map(|(priority, decision)| candidate_from_decision(now_ms, priority, decision))
        .collect()
}

/// 按接线配置产仪式候选: 开关关 (默认) 恒返回空, 开关开等价于 [`ritual_candidates`]。
///
/// 关闭时**在产候选面就拦掉**, 不依赖调用方自觉; [`CareEngine::poll_bio_candidate`]
/// 另有第二道开关, 双保险。
///
/// [`CareEngine::poll_bio_candidate`]: super::engine::CareEngine::poll_bio_candidate
#[must_use]
pub fn ritual_candidates_for(
    config: &CareConfig,
    now_ms: i64,
    states: &[RitualState],
    ritual_config: &RitualConfig,
) -> Vec<CareCandidate> {
    if !config.bio_candidates_enabled() {
        return Vec::new();
    }
    ritual_candidates(now_ms, states, ritual_config)
}

/// 单条可投递决策 → 候选。
fn candidate_from_decision(
    now_ms: i64,
    priority: RitualCandidatePriority,
    decision: RitualDecision,
) -> CareCandidate {
    let RitualDecision {
        kind,
        due,
        reason,
        suggested_message_key,
    } = decision;
    CareCandidate {
        rule_id: suggested_message_key.clone(),
        trigger: format!(
            "仪式 {}（{}，优先级 {}）；{}",
            kind.label(),
            due.label(),
            priority.label(),
            reason
        ),
        due_ms: now_ms,
        material: suggested_message_key,
        tier: CareTier::Ritual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::care::rules::{validate_care_material, validate_care_rule_id};
    use crate::ritual::{Cadence, RitualKind, RitualSchedule, Window};

    /// 固定注入时刻 (2025-10-09T08:53:20Z); 测试永不读系统时钟。
    const NOW: i64 = 1_760_000_000_000;
    const NOW_U64: u64 = 1_760_000_000_000;
    const HOUR_MS: u64 = 3_600_000;
    const DAY_MS: u64 = 24 * HOUR_MS;
    /// 测试用宽限: 6 小时。
    const GRACE: u64 = 6 * HOUR_MS;

    fn schedule(kind: RitualKind, cadence: Cadence) -> RitualSchedule {
        RitualSchedule::new(kind, cadence, Window::Flexible, GRACE)
    }

    fn state(kind: RitualKind, last_done_ms: u64) -> RitualState {
        RitualState {
            kind,
            last_done_ms,
            consecutive_done: 0,
            skipped_count: 0,
            muted_until_ms: None,
        }
    }

    fn config_of(schedules: Vec<RitualSchedule>) -> RitualConfig {
        RitualConfig::new(schedules)
    }

    fn all_kind_schedules(cadence: Cadence) -> Vec<RitualSchedule> {
        RitualKind::ALL
            .iter()
            .map(|kind| schedule(*kind, cadence))
            .collect()
    }

    #[test]
    fn due_becomes_high_priority_candidate_with_stable_key() {
        let config = config_of(vec![schedule(RitualKind::MorningCheckIn, Cadence::Daily)]);
        let candidates = ritual_candidates(NOW, &[state(RitualKind::MorningCheckIn, 0)], &config);

        assert_eq!(candidates.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(candidate.rule_id, "ritual.morning_check_in.due");
        assert_eq!(
            candidate.material, candidate.rule_id,
            "素材 = 稳定文案键, 本层不生成正文"
        );
        assert!(
            candidate.trigger.contains("优先级 high"),
            "trigger: {}",
            candidate.trigger
        );
        assert_eq!(candidate.tier, CareTier::Ritual);
        assert_eq!(candidate.due_ms, NOW);
        validate_care_rule_id(&candidate.rule_id).expect("候选身份是合法 care id");
        validate_care_material(&candidate.material).expect("候选素材合法 (非空有界)");
    }

    #[test]
    fn due_soft_becomes_low_priority_candidate() {
        let config = config_of(vec![schedule(RitualKind::EveningWrap, Cadence::Daily)]);
        let late_state = state(RitualKind::EveningWrap, NOW_U64 - DAY_MS - GRACE - 1);
        let candidates = ritual_candidates(NOW, &[late_state], &config);

        assert_eq!(candidates.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(candidate.rule_id, "ritual.evening_wrap.due_soft");
        assert!(
            candidate.trigger.contains("优先级 low"),
            "trigger: {}",
            candidate.trigger
        );
        assert_eq!(candidate.tier, CareTier::Ritual);
        assert_eq!(candidate.due_ms, NOW);
    }

    #[test]
    fn muted_and_not_due_decisions_produce_no_candidates() {
        let config = config_of(vec![
            schedule(RitualKind::MorningCheckIn, Cadence::Daily),
            schedule(RitualKind::EveningWrap, Cadence::Daily),
        ]);
        let mut muted = state(RitualKind::MorningCheckIn, 0);
        muted.muted_until_ms = Some(NOW_U64 + DAY_MS);
        let not_due = state(RitualKind::EveningWrap, NOW_U64 - HOUR_MS);

        let candidates = ritual_candidates(NOW, &[muted, not_due], &config);
        assert!(
            candidates.is_empty(),
            "Muted / NotDue 不得产候选: {candidates:?}"
        );

        // 无排程同样不推断到期 (NotDue)。
        let no_schedule = ritual_candidates(
            NOW,
            &[state(RitualKind::SharedProject, 0)],
            &RitualConfig::default(),
        );
        assert!(no_schedule.is_empty());
    }

    #[test]
    fn due_candidates_are_ordered_before_due_soft_candidates() {
        let config = config_of(vec![
            schedule(RitualKind::MorningCheckIn, Cadence::Daily),
            schedule(RitualKind::EveningWrap, Cadence::Daily),
        ]);
        // Morning 已过宽限 (低优先), Evening 从未完成 (高优先): 输出必须重排。
        let states = [
            state(RitualKind::MorningCheckIn, NOW_U64 - DAY_MS - GRACE - 1),
            state(RitualKind::EveningWrap, 0),
        ];
        let candidates = ritual_candidates(NOW, &states, &config);

        let keys: Vec<&str> = candidates
            .iter()
            .map(|candidate| candidate.rule_id.as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "ritual.evening_wrap.due",
                "ritual.morning_check_in.due_soft"
            ],
            "高优先 (Due) 必须排在低优先 (DueSoft) 前"
        );
    }

    #[test]
    fn switch_off_produces_no_candidates_and_switch_on_matches_the_pure_source() {
        let config = config_of(all_kind_schedules(Cadence::Daily));
        let states: Vec<RitualState> = RitualKind::ALL.iter().map(|kind| state(*kind, 0)).collect();

        assert!(!CareConfig::default().bio_candidates_enabled());
        assert!(
            ritual_candidates_for(&CareConfig::default(), NOW, &states, &config).is_empty(),
            "默认关: 两类生物机制候选永不产出"
        );

        let enabled = CareConfig {
            bio_candidates_enabled: true,
        };
        assert_eq!(
            ritual_candidates_for(&enabled, NOW, &states, &config),
            ritual_candidates(NOW, &states, &config),
            "开关开 = 纯候选源 (不改写候选)"
        );
        assert!(!ritual_candidates_for(&enabled, NOW, &states, &config).is_empty());
    }

    #[test]
    fn candidates_are_deterministic_and_state_order_invariant() {
        let config = config_of(all_kind_schedules(Cadence::Daily));
        let states: Vec<RitualState> = RitualKind::ALL
            .iter()
            .map(|kind| state(*kind, NOW_U64 - DAY_MS))
            .collect();

        let first = ritual_candidates(NOW, &states, &config);
        let second = ritual_candidates(NOW, &states, &config);
        assert_eq!(first, second, "同输入同输出");

        let mut reversed = states;
        reversed.reverse();
        assert_eq!(
            ritual_candidates(NOW, &reversed, &config),
            first,
            "输入顺序不得改变候选输出"
        );
    }

    #[test]
    fn candidates_never_carry_guilt_or_streak_wording() {
        let forbidden = [
            "连续",
            "断签",
            "streak",
            "内疚",
            "亏欠",
            "欠账",
            "必须",
            "应该",
            "你怎么",
            "又没",
            "拖了",
            "迟到",
        ];
        let config = config_of(all_kind_schedules(Cadence::Daily));
        let mut states: Vec<RitualState> =
            RitualKind::ALL.iter().map(|kind| state(*kind, 0)).collect();
        states.push(state(
            RitualKind::MorningCheckIn,
            NOW_U64 - DAY_MS - GRACE - 1,
        ));
        let candidates = ritual_candidates(NOW, &states, &config);
        assert!(candidates.len() >= 2);
        for candidate in &candidates {
            for text in [
                candidate.rule_id.as_str(),
                candidate.trigger.as_str(),
                candidate.material.as_str(),
            ] {
                for needle in forbidden {
                    assert!(
                        !text.contains(needle),
                        "候选文本含禁用片段 {needle}: {text}"
                    );
                }
            }
        }
    }

    #[test]
    fn only_actionable_decisions_are_forwarded() {
        let config = config_of(vec![
            schedule(RitualKind::MorningCheckIn, Cadence::Daily),
            schedule(RitualKind::EveningWrap, Cadence::Daily),
            schedule(RitualKind::Anniversary, Cadence::Daily),
        ]);
        let mut muted = state(RitualKind::EveningWrap, 0);
        muted.muted_until_ms = Some(NOW_U64 + HOUR_MS);
        let states = [
            state(RitualKind::MorningCheckIn, 0),              // Due
            muted,                                             // Muted
            state(RitualKind::Anniversary, NOW_U64 - HOUR_MS), // NotDue
        ];
        let decisions = plan_rituals(NOW_U64, &states, &config);
        let actionable = decisions
            .iter()
            .filter(|decision| decision.due.is_actionable())
            .count();
        assert_eq!(actionable, 1, "前置: 只有一条可投递决策");
        assert_eq!(
            ritual_candidates(NOW, &states, &config).len(),
            actionable,
            "候选数必须等于可投递决策数"
        );
    }

    #[test]
    fn all_kinds_due_keep_declaration_order_within_one_priority() {
        let config = config_of(all_kind_schedules(Cadence::Daily));
        let states: Vec<RitualState> = RitualKind::ALL.iter().map(|kind| state(*kind, 0)).collect();
        let candidates = ritual_candidates(NOW, &states, &config);

        assert_eq!(candidates.len(), RitualKind::ALL.len());
        for (candidate, kind) in candidates.iter().zip(RitualKind::ALL) {
            assert_eq!(candidate.rule_id, format!("ritual.{}.due", kind.as_str()));
            assert_eq!(candidate.tier, CareTier::Ritual);
        }
    }

    #[test]
    fn negative_now_degrades_without_panic() {
        let config = config_of(vec![schedule(RitualKind::MorningCheckIn, Cadence::Daily)]);
        let candidates = ritual_candidates(-1, &[state(RitualKind::MorningCheckIn, 0)], &config);
        assert_eq!(candidates.len(), 1, "负时钟按 epoch 0 求值, 不 panic");
        assert_eq!(candidates[0].due_ms, -1, "due_ms 原样保留注入值");
    }
}
