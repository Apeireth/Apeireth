//! 重聚候选源 (SG-19 分离-重聚内核 → care 调度): 把 [`ReunionPlan`] 转成
//! [`CareCandidate`], 由 care 管道**统一裁决** (勿扰 / 专注 / 频率 / 预算)。
//!
//! # 转换规则
//!
//! - **`Brief` 不产候选**: 短分离不演戏 (问候强度 QuietResume, 不值得占一次关怀)。
//! - **无可续线索不产候选**: `plan.threads` 为空 = 没有可续接的共同线索, 只有一句
//!   问候不足以说明「为什么是现在」, 宁可不发。
//! - **其余类别产出**: `Hours` / `Days` / `Weeks` 且有可续线索 → 一条候选。
//!
//! 候选内容:
//!
//! - `material` = `plan.greeting` (既有固定安全模板, **不含内疚 / 指责 / 索偿**
//!   措辞, 由 attachment 内核的不变量测试钉死; 本层不改写、不加码);
//! - `trigger` = 类别 + 线索锚点 + `plan.rationale` (分类 / 风格取舍 / 线索计数,
//!   同样不含禁止词), 供标准回合看见「为什么现在提」;
//! - `rule_id` = `reunion.<class>` (稳定身份, 合法 care id 字符集);
//! - `tier` = [`CareTier::Reunion`] (走普通频率账, 不占守护独立上限)。
//!
//! # 不读时钟
//!
//! `last_seen_ms` / `now_ms` / `threads` 都由调用方注入并已经过
//! [`crate::attachment::plan_reunion`] 求值; 本模块**不读系统时钟、不做 I/O**。
//! `due_ms` 取 `plan.threads` 里最近一条线索的 `last_touched_ms` (plan 内唯一
//! 真实时间锚), 只作可追溯的线索锚点; 投递时刻由 care 心跳按注入 `now_ms` 裁决。
//!
//! # 默认关 + 0 装诚实 (IMPLEMENTED, 未装配)
//!
//! [`reunion_candidate_for`] 受 [`super::CareConfig::bio_candidates_enabled`]
//! 总开关控制, **默认 `false`** (关闭时恒 `None`); runtime-assembly 的 replay
//! 数据注入与开关打开待 W3.2。因此本模块目前不是线上行为, 不声称
//! PRODUCTION WIRED / DEFAULT ENABLED。本层也不做去重 / 发起节奏: 同一 plan
//! 重复调用产同一候选 (确定性), 何时调用归心跳调度面。

use super::engine::CareCandidate;
use super::reminder::CareTier;
use super::CareConfig;
use crate::attachment::{ReunionPlan, SeparationClass};

/// 重聚计划 → care 候选 (纯函数、确定性、不读时钟)。
///
/// 仅当 `plan.class != Brief` **且** `plan.threads` 非空时产出候选; 文案用
/// `plan.greeting` (固定安全模板, 不产内疚文案), 身份用 `reunion.<class>`.
#[must_use]
pub fn reunion_candidate(plan: &ReunionPlan) -> Option<CareCandidate> {
    if plan.class == SeparationClass::Brief || plan.threads.is_empty() {
        return None;
    }
    let anchor_ms = plan
        .threads
        .iter()
        .map(|thread| thread.last_touched_ms)
        .max()
        .unwrap_or(0);
    let due_ms = i64::try_from(anchor_ms).unwrap_or(i64::MAX);
    let class_key = class_key(plan.class);
    Some(CareCandidate {
        rule_id: format!("reunion.{class_key}"),
        trigger: format!(
            "重聚候选（{class_key}，线索锚点 {due_ms}）；{}",
            plan.rationale
        ),
        due_ms,
        material: plan.greeting.clone(),
        tier: CareTier::Reunion,
    })
}

/// 按接线配置产重聚候选: 开关关 (默认) 恒 `None`, 开关开等价于 [`reunion_candidate`]。
///
/// [`CareEngine::poll_bio_candidate`]: super::engine::CareEngine::poll_bio_candidate
#[must_use]
pub fn reunion_candidate_for(config: &CareConfig, plan: &ReunionPlan) -> Option<CareCandidate> {
    if !config.bio_candidates_enabled() {
        return None;
    }
    reunion_candidate(plan)
}

/// 稳定类别键 (只用于候选身份 / 描述; 与 [`SeparationClass`] 的 serde snake_case 同词)。
const fn class_key(class: SeparationClass) -> &'static str {
    match class {
        SeparationClass::Brief => "brief",
        SeparationClass::Hours => "hours",
        SeparationClass::Days => "days",
        SeparationClass::Weeks => "weeks",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachment::{plan_reunion, GreetingStyle, ResumeThread, ReunionPolicy};
    use crate::care::rules::validate_care_rule_id;

    /// 固定注入时刻 (2023-11-14T22:13:20Z); 测试永不读系统时钟。
    const NOW: u64 = 1_700_000_000_000;
    const MINUTE_MS: u64 = 60_000;
    const HOUR_MS: u64 = 60 * MINUTE_MS;
    const DAY_MS: u64 = 24 * HOUR_MS;
    const WEEK_MS: u64 = 7 * DAY_MS;

    /// 内疚 / 指责 / 情绪勒索黑名单 (与 attachment 内核同源, 逐字钉死)。
    const FORBIDDEN_PHRASES: [&str; 10] = [
        "等你好久",
        "你去哪了",
        "你怎么不回",
        "我很难过因为你",
        "别丢下我",
        "你怎么才回来",
        "我一直等你",
        "你都不理我",
        "你欠我",
        "你让我等",
    ];

    fn thread(id: &str, last_touched_ms: u64) -> ResumeThread {
        ResumeThread {
            id: id.to_owned(),
            summary: format!("线索 {id}"),
            salience: 0.8,
            emotion_weight: 0.6,
            last_touched_ms,
        }
    }

    fn sample_threads() -> Vec<ResumeThread> {
        vec![
            thread("fresh", NOW - HOUR_MS),
            thread("older", NOW - 2 * DAY_MS),
        ]
    }

    fn plan_for(gap_ms: u64, threads: &[ResumeThread], policy: ReunionPolicy) -> ReunionPlan {
        plan_reunion(NOW.saturating_sub(gap_ms), NOW, threads, policy)
    }

    fn enabled() -> CareConfig {
        CareConfig {
            bio_candidates_enabled: true,
        }
    }

    fn assert_clean(text: &str) {
        for phrase in FORBIDDEN_PHRASES {
            assert!(
                !text.contains(phrase),
                "内疚/指责措辞 {phrase:?} 出现在 {text:?}"
            );
        }
    }

    #[test]
    fn days_separation_with_threads_produces_a_candidate() {
        let plan = plan_for(3 * DAY_MS, &sample_threads(), ReunionPolicy::default());
        assert_eq!(plan.class, SeparationClass::Days);

        let candidate = reunion_candidate(&plan).expect("Days + 可续线索必须产候选");
        assert_eq!(candidate.rule_id, "reunion.days");
        assert_eq!(
            candidate.material, plan.greeting,
            "文案 = 既有 greeting, 不改写不加码"
        );
        assert_eq!(candidate.tier, CareTier::Reunion);
        assert!(candidate.trigger.contains("Days"), "{}", candidate.trigger);
        assert_eq!(
            candidate.due_ms,
            i64::try_from(NOW - HOUR_MS).unwrap(),
            "线索锚点 = 最近一条可续线索的触碰时刻 (输入真值, 不读时钟)"
        );
        validate_care_rule_id(&candidate.rule_id).expect("候选身份是合法 care id");
    }

    #[test]
    fn weeks_separation_with_threads_produces_a_candidate() {
        let plan = plan_for(2 * WEEK_MS, &sample_threads(), ReunionPolicy::default());
        assert_eq!(plan.class, SeparationClass::Weeks);

        let candidate = reunion_candidate(&plan).expect("Weeks + 可续线索必须产候选");
        assert_eq!(candidate.rule_id, "reunion.weeks");
        assert_eq!(candidate.material, plan.greeting);
        assert_eq!(candidate.tier, CareTier::Reunion);
    }

    #[test]
    fn brief_separation_never_produces_a_candidate() {
        let plan = plan_for(30 * MINUTE_MS, &sample_threads(), ReunionPolicy::default());
        assert_eq!(plan.class, SeparationClass::Brief);
        assert!(!plan.threads.is_empty(), "前置: Brief 也有可续线索");
        assert!(reunion_candidate(&plan).is_none(), "短分离不演戏");
    }

    #[test]
    fn empty_threads_produce_no_candidate() {
        for gap in [5 * HOUR_MS, 3 * DAY_MS, 2 * WEEK_MS] {
            let plan = plan_for(gap, &[], ReunionPolicy::default());
            assert!(plan.threads.is_empty(), "前置: 无线索");
            assert!(
                reunion_candidate(&plan).is_none(),
                "无可续线索不产候选 (gap={gap})"
            );
        }

        // policy 关闭续接 / 上限 0 同样没有线索。
        for policy in [
            ReunionPolicy {
                resume_threads: false,
                ..ReunionPolicy::default()
            },
            ReunionPolicy {
                max_threads: 0,
                ..ReunionPolicy::default()
            },
            ReunionPolicy {
                max_age_ms: HOUR_MS - 1,
                ..ReunionPolicy::default()
            },
        ] {
            let plan = plan_for(3 * DAY_MS, &sample_threads(), policy);
            assert!(plan.threads.is_empty(), "前置: policy 过滤后无线索");
            assert!(reunion_candidate(&plan).is_none());
        }
    }

    #[test]
    fn switch_off_produces_no_candidate_and_switch_on_matches_the_pure_source() {
        let plan = plan_for(3 * DAY_MS, &sample_threads(), ReunionPolicy::default());

        assert!(!CareConfig::default().bio_candidates_enabled());
        assert!(
            reunion_candidate_for(&CareConfig::default(), &plan).is_none(),
            "默认关: 重聚候选永不产出"
        );
        assert_eq!(
            reunion_candidate_for(&enabled(), &plan),
            reunion_candidate(&plan),
            "开关开 = 纯候选源 (不改写候选)"
        );
        assert!(reunion_candidate_for(&enabled(), &plan).is_some());
    }

    #[test]
    fn candidate_never_carries_guilt_wording() {
        assert!(FORBIDDEN_PHRASES.len() >= 8);
        let threads = sample_threads();
        for (index, gap) in [5 * HOUR_MS, 3 * DAY_MS, 2 * WEEK_MS].iter().enumerate() {
            for style in [
                GreetingStyle::QuietResume,
                GreetingStyle::GentleCheckIn,
                GreetingStyle::WarmReturn,
            ] {
                let plan = plan_for(
                    *gap,
                    &threads,
                    ReunionPolicy {
                        greeting_style: style,
                        ..ReunionPolicy::default()
                    },
                );
                let Some(candidate) = reunion_candidate(&plan) else {
                    panic!("gap #{index} 必须产候选");
                };
                assert_clean(&candidate.rule_id);
                assert_clean(&candidate.trigger);
                assert_clean(&candidate.material);
            }
        }
    }

    #[test]
    fn candidate_is_deterministic_for_the_same_plan() {
        let threads = sample_threads();
        let first = plan_for(3 * DAY_MS, &threads, ReunionPolicy::default());
        let second = plan_for(3 * DAY_MS, &threads, ReunionPolicy::default());
        assert_eq!(first, second);
        assert_eq!(reunion_candidate(&first), reunion_candidate(&second));
        assert_eq!(reunion_candidate(&first), reunion_candidate(&first));
    }

    #[test]
    fn expired_threads_leave_no_candidate() {
        // 线索全部早于保鲜窗: plan.threads 为空 → 无候选 (不是内疚提醒)。
        let plan = plan_for(
            3 * DAY_MS,
            &sample_threads(),
            ReunionPolicy {
                max_age_ms: HOUR_MS - 1,
                ..ReunionPolicy::default()
            },
        );
        assert!(plan.threads.is_empty());
        assert!(reunion_candidate(&plan).is_none());
    }

    #[test]
    fn hours_separation_also_produces_a_candidate() {
        let plan = plan_for(5 * HOUR_MS, &sample_threads(), ReunionPolicy::default());
        assert_eq!(plan.class, SeparationClass::Hours);
        let candidate = reunion_candidate(&plan).expect("Hours + 可续线索必须产候选");
        assert_eq!(candidate.rule_id, "reunion.hours");
        assert_eq!(candidate.material, plan.greeting);
    }
}
