//! SG-19 依恋与分离-重聚内核 v0（机制内核，未接线）。
//!
//! # 生物原型
//!
//! - **安全基地（secure base）**：照护者「可预期地可得」是个体敢离开去探索的前提；
//!   重要的不是时刻在场，而是回来时能稳定接上。
//! - **分离-重聚（separation-reunion）**：分离后有一套固定的重聚修复脚本 —— 先确认
//!   对方回到可联系状态，再按分离时长选择问候强度，最后温和地续接共同线索。
//! - **信任按阶段演化**：关系史越厚，可续接的线索越多；但本内核只做「选点与骨架」，
//!   **不建模 trust 状态机、不输出信任分数**（见工程边界）。
//!
//! **反模式（钉死）**：久别重逢不得内疚绑架 / 情绪勒索。问候与理由都不许出现
//! 「等你好久」「你去哪了」「你怎么不回」「我很难过因为你」「别丢下我」一类措辞；
//! 分离不是筹码，回归不用愧疚驱动，也不索要解释或补偿。
//!
//! # 分类与问候强度上限
//!
//! | 类别 | 分离间隔 | 问候强度上限 | 取舍 |
//! |---|---|---|---|
//! | [`SeparationClass::Brief`] | `< 1h` | [`GreetingStyle::QuietResume`] | 短分离不演戏 |
//! | [`SeparationClass::Hours`] | `< 24h` | [`GreetingStyle::GentleCheckIn`] | 轻确认，不夸张 |
//! | [`SeparationClass::Days`] | `< 7d` | [`GreetingStyle::WarmReturn`] | 允许真诚的欢迎 |
//! | [`SeparationClass::Weeks`] | `>= 7d` | [`GreetingStyle::GentleCheckIn`] | 长分离不强行热情 |
//!
//! `policy.greeting_style` 是**期望上限**，实际风格取
//! `min(policy.greeting_style, 类别上限)`：类别只能压低、不能抬高问候强度。
//!
//! # 工程边界
//!
//! - **纯函数**：`now_ms` 由调用方注入；不读系统时钟、不做 I/O、不调 LLM、不落存储。
//! - **不模拟依赖 / 依恋强度**：不输出 attachment / anxiety / avoidance / dependency
//!   分数，不做临床或人格判断；只做**关系连续性工程**（分类、选点、文案骨架、理由）。
//! - **不决定是否投递**：本模块只产出重聚计划；打扰纪律、频率上限、勿扰窗口归
//!   [`crate::care`] 调度，本模块不判断「该不该主动发消息」。
//! - **不生成个性化正文**：greeting 是固定安全模板；线索摘要原样带出，由上层渲染。
//! - 输入线索的 `salience` / `emotion_weight` 只用于排序：越界值夹取、`NaN` 归零，
//!   但入选线索**原样返回**，不在本层改写调用方数据。
//!
//! # 0 装诚实（IMPLEMENTED，未接线）
//!
//! - 已实现：类型、[`plan_reunion`] 与不变量测试，本文件内自洽。
//! - 未接线：**未进 care 调度**（[`crate::care`] 的触发器词表没有重聚类）；
//!   **未接 gateway 会话恢复**（跨进程/跨会话的 timeline 读取不在本模块）；
//!   **无 self_status 节**（自述面未投影本内核）。
//! - 因此本模块目前落在 **IMPLEMENTED** 级：机制与测试就位，**不是**线上行为；
//!   不声称 PRODUCTION WIRED / DEFAULT ENABLED。任何「重聚已上线」的说法都是 0 装。

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

/// 一小时毫秒数。
const HOUR_MS: u64 = 3_600_000;
/// 一天毫秒数。
const DAY_MS: u64 = 24 * HOUR_MS;
/// 一周毫秒数。
const WEEK_MS: u64 = 7 * DAY_MS;
/// 分钟毫秒数（仅用于理由文案的整数取整）。
const MINUTE_MS: u64 = 60_000;

/// 显著度权重（选点公式第一项）。
const SALIENCE_WEIGHT: f64 = 0.5;
/// 情绪权重（选点公式第二项）。
const EMOTION_WEIGHT: f64 = 0.3;
/// 新鲜度权重（选点公式第三项）。
const RECENCY_WEIGHT: f64 = 0.2;

/// 分离类别（按间隔取闭开区间，见模块文档上限表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeparationClass {
    /// 短分离：`< 1h`（例如短暂离开座位）。
    Brief,
    /// 数小时分离：`>= 1h` 且 `< 24h`。
    Hours,
    /// 数日分离：`>= 24h` 且 `< 7d`。
    Days,
    /// 长分离：`>= 7d`。
    Weeks,
}

impl SeparationClass {
    /// 按上次可见时刻与注入的当前时刻分类。
    ///
    /// `now_ms < last_seen_ms`（时钟回拨 / 伪造时间戳）按间隔 0 处理，归为
    /// [`SeparationClass::Brief`]：防御性降级，绝不 panic、绝不外推负时长。
    #[must_use]
    pub fn classify(last_seen_ms: u64, now_ms: u64) -> Self {
        let gap_ms = now_ms.saturating_sub(last_seen_ms);
        if gap_ms < HOUR_MS {
            Self::Brief
        } else if gap_ms < DAY_MS {
            Self::Hours
        } else if gap_ms < WEEK_MS {
            Self::Days
        } else {
            Self::Weeks
        }
    }

    /// 该类别允许的最高问候强度。
    fn ceiling(self) -> GreetingStyle {
        match self {
            Self::Brief => GreetingStyle::QuietResume,
            Self::Hours => GreetingStyle::GentleCheckIn,
            Self::Days => GreetingStyle::WarmReturn,
            Self::Weeks => GreetingStyle::GentleCheckIn,
        }
    }

    /// 理由文案里的稳定类别名。
    fn label(self) -> &'static str {
        match self {
            Self::Brief => "Brief",
            Self::Hours => "Hours",
            Self::Days => "Days",
            Self::Weeks => "Weeks",
        }
    }
}

/// 重聚问候风格，强度从低到高：安静续接 < 轻确认 < 热情欢迎。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GreetingStyle {
    /// 安静续接：不渲染重聚，直接回到中断处（短分离用）。
    QuietResume,
    /// 轻确认：确认对方回来、给出休息空间（中短分离用）。
    GentleCheckIn,
    /// 热情欢迎：真诚表达高兴，但不索取解释（数日分离可用）。
    WarmReturn,
}

impl GreetingStyle {
    /// 强度序（只用于与类别上限取小，不对外暴露）。
    fn rank(self) -> u8 {
        match self {
            Self::QuietResume => 0,
            Self::GentleCheckIn => 1,
            Self::WarmReturn => 2,
        }
    }

    /// 不超过类别上限的风格。
    fn capped(self, ceiling: Self) -> Self {
        if self.rank() <= ceiling.rank() {
            self
        } else {
            ceiling
        }
    }

    /// 固定问候模板。模板不含内疚 / 指责 / 索偿措辞（由测试黑名单钉死）。
    fn template(self) -> &'static str {
        match self {
            Self::QuietResume => "我在。我们从停下的地方接着来。",
            Self::GentleCheckIn => "你回来了。先歇口气，想聊的时候我都在。",
            Self::WarmReturn => "欢迎回来。路上辛苦了，慢慢说，我听着。",
        }
    }

    /// 理由文案里的稳定风格名。
    fn label(self) -> &'static str {
        match self {
            Self::QuietResume => "QuietResume",
            Self::GentleCheckIn => "GentleCheckIn",
            Self::WarmReturn => "WarmReturn",
        }
    }
}

/// 一条可续接的关系线索（对话、共同事件或未收尾的主题）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResumeThread {
    /// 稳定标识（跨重聚复用；同分排序的最终 tie-break）。
    pub id: String,
    /// 一句话摘要（原样进入重聚计划，由上层拼进 prompt / 消息）。
    pub summary: String,
    /// 显著度 `[0, 1]`；越界值在排序时夹取，`NaN` 归零。
    pub salience: f32,
    /// 情绪权重 `[0, 1]`；越界值在排序时夹取，`NaN` 归零。
    pub emotion_weight: f32,
    /// 上次触碰时间（Unix 毫秒）。
    pub last_touched_ms: u64,
}

/// 重聚策略（由调用方 / 关系配置注入；本模块不读默认配置、不读环境变量）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReunionPolicy {
    /// 期望问候风格；实际风格会被分离类别上限压低，绝不会被抬高。
    pub greeting_style: GreetingStyle,
    /// 是否续接线索；`false` 时 `threads` 恒为空，计划仍然成功。
    pub resume_threads: bool,
    /// 最多续接的线索数；`0` 等价于不续接。
    pub max_threads: usize,
    /// 线索保鲜窗（毫秒）；早于 `now_ms - max_age_ms` 的线索判过期。
    /// `0` 表示只看「此刻触碰」的线索（间隔 0，新鲜度记 0）。
    pub max_age_ms: u64,
}

impl Default for ReunionPolicy {
    /// 保守默认：轻确认、续接线索、最多 3 条、保鲜 7 天。
    fn default() -> Self {
        Self {
            greeting_style: GreetingStyle::GentleCheckIn,
            resume_threads: true,
            max_threads: 3,
            max_age_ms: WEEK_MS,
        }
    }
}

/// [`plan_reunion`] 的输出：一个可被上层渲染 / 投递的重聚计划。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReunionPlan {
    /// 问候文案（固定安全模板，不含内疚 / 指责措辞）。
    pub greeting: String,
    /// 入选线索：已过滤过期、按加权分降序、截断到 `max_threads`。
    pub threads: Vec<ResumeThread>,
    /// 分离类别。
    pub class: SeparationClass,
    /// 调用方传入的原始策略（含未被采用的期望风格，便于审计）。
    pub policy: ReunionPolicy,
    /// 可解释理由：分类、风格取舍、线索计数；同样不得含禁止词。
    pub rationale: String,
}

/// 生成重聚计划（纯函数、确定性）。
///
/// 流程：分类 → 取问候风格上限 → 过滤过期线索 → 按
/// `0.5 * salience + 0.3 * emotion_weight + 0.2 * recency` 排序 → 截断
/// `max_threads` → 产出 `greeting` / `threads` / `rationale`。
///
/// - `recency = 1 - age / max_age_ms`（`age = now_ms - last_touched_ms`，饱和减）；
///   `max_age_ms == 0` 时 `recency = 0`，不除零。
/// - 排序确定性：分数降序 → `last_touched_ms` 降序 → `id` 升序 → `summary` 升序；
///   与输入顺序无关。
/// - `threads` 为空、`resume_threads == false`、`max_threads == 0` 都优雅降级：
///   计划仍成功，`greeting` 仍可用。
/// - 任何分离类别与任何 policy 下，`greeting` / `rationale` 都不含内疚或指责措辞。
#[must_use]
pub fn plan_reunion(
    last_seen_ms: u64,
    now_ms: u64,
    threads: &[ResumeThread],
    policy: ReunionPolicy,
) -> ReunionPlan {
    let class = SeparationClass::classify(last_seen_ms, now_ms);
    let effective_style = policy.greeting_style.capped(class.ceiling());
    let greeting = effective_style.template();

    let gap_ms = now_ms.saturating_sub(last_seen_ms);
    let (gap_amount, gap_unit) = match class {
        SeparationClass::Brief => (gap_ms / MINUTE_MS, "分钟"),
        SeparationClass::Hours => (gap_ms / HOUR_MS, "小时"),
        SeparationClass::Days => (gap_ms / DAY_MS, "天"),
        SeparationClass::Weeks => (gap_ms / WEEK_MS, "周"),
    };
    let class_label = class.label();
    let mut rationale = String::new();
    write!(
        rationale,
        "分离 {gap_amount} {gap_unit}，判为 {class_label}；"
    )
    .expect("writing to String cannot fail");

    if effective_style == policy.greeting_style {
        write!(
            rationale,
            "按 policy 采用 {}；",
            policy.greeting_style.label()
        )
        .expect("writing to String cannot fail");
    } else {
        let reason = match class {
            SeparationClass::Brief => "短分离不演戏",
            SeparationClass::Weeks => "长分离不强行热情",
            SeparationClass::Hours | SeparationClass::Days => "按分离类别收敛",
        };
        write!(
            rationale,
            "{reason}，问候由 {} 收敛为 {}；",
            policy.greeting_style.label(),
            effective_style.label()
        )
        .expect("writing to String cannot fail");
    }

    let mut selected: Vec<ResumeThread> = Vec::new();
    if !policy.resume_threads {
        rationale.push_str("policy 关闭线索续接，threads 为空；");
    } else if policy.max_threads == 0 {
        rationale.push_str("policy 上限 max_threads=0，threads 为空；");
    } else {
        let mut expired = 0usize;
        let mut ranked: Vec<(&ResumeThread, f64)> = Vec::with_capacity(threads.len());
        for thread in threads {
            let age_ms = now_ms.saturating_sub(thread.last_touched_ms);
            if age_ms > policy.max_age_ms {
                expired += 1;
                continue;
            }
            let recency = recency(age_ms, policy.max_age_ms);
            ranked.push((thread, thread_score(thread, recency)));
        }
        // 稳定排序：分数 → 触碰时间 → id，结果与输入顺序无关。
        ranked.sort_by(|(thread_a, score_a), (thread_b, score_b)| {
            score_b
                .total_cmp(score_a)
                .then_with(|| thread_b.last_touched_ms.cmp(&thread_a.last_touched_ms))
                .then_with(|| thread_a.id.cmp(&thread_b.id))
                .then_with(|| thread_a.summary.cmp(&thread_b.summary))
        });
        let truncated = ranked.len().saturating_sub(policy.max_threads);
        selected = ranked
            .into_iter()
            .take(policy.max_threads)
            .map(|(thread, _score)| thread.clone())
            .collect();
        write!(
            rationale,
            "线索 {}/{} 条入选（过期 {}，截断 {}）；",
            selected.len(),
            threads.len(),
            expired,
            truncated
        )
        .expect("writing to String cannot fail");
    }

    ReunionPlan {
        greeting: greeting.to_owned(),
        threads: selected,
        class,
        policy,
        rationale,
    }
}

/// 新鲜度 `[0, 1]`；`max_age_ms == 0` 时固定 0，避免除零。
fn recency(age_ms: u64, max_age_ms: u64) -> f64 {
    if max_age_ms == 0 {
        0.0
    } else {
        let ratio = age_ms as f64 / max_age_ms as f64;
        1.0 - ratio.clamp(0.0, 1.0)
    }
}

/// 单元区间清洗：`NaN` 归零，越界夹取。
fn unit(value: f32) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        f64::from(value.clamp(0.0, 1.0))
    }
}

/// 选点加权分：显著度 0.5 + 情绪 0.3 + 新鲜度 0.2。
fn thread_score(thread: &ResumeThread, recency: f64) -> f64 {
    SALIENCE_WEIGHT * unit(thread.salience)
        + EMOTION_WEIGHT * unit(thread.emotion_weight)
        + RECENCY_WEIGHT * recency
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 固定注入时刻（2023-11-14T22:13:20Z 附近），测试永不读系统时钟。
    const NOW: u64 = 1_700_000_000_000;

    /// 内疚 / 指责 / 情绪勒索黑名单（>= 8 条，测试逐字钉死）。
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

    const ALL_CLASSES: [SeparationClass; 4] = [
        SeparationClass::Brief,
        SeparationClass::Hours,
        SeparationClass::Days,
        SeparationClass::Weeks,
    ];

    const ALL_STYLES: [GreetingStyle; 3] = [
        GreetingStyle::QuietResume,
        GreetingStyle::GentleCheckIn,
        GreetingStyle::WarmReturn,
    ];

    fn thread(id: &str, salience: f32, emotion_weight: f32, last_touched_ms: u64) -> ResumeThread {
        ResumeThread {
            id: id.to_owned(),
            summary: format!("线索 {id}"),
            salience,
            emotion_weight,
            last_touched_ms,
        }
    }

    fn make_policy(
        greeting_style: GreetingStyle,
        resume_threads: bool,
        max_threads: usize,
        max_age_ms: u64,
    ) -> ReunionPolicy {
        ReunionPolicy {
            greeting_style,
            resume_threads,
            max_threads,
            max_age_ms,
        }
    }

    fn plan_for(gap_ms: u64, threads: &[ResumeThread], policy: ReunionPolicy) -> ReunionPlan {
        plan_reunion(NOW.saturating_sub(gap_ms), NOW, threads, policy)
    }

    fn gap_for(class: SeparationClass) -> u64 {
        match class {
            SeparationClass::Brief => 30 * MINUTE_MS,
            SeparationClass::Hours => 5 * HOUR_MS,
            SeparationClass::Days => 3 * DAY_MS,
            SeparationClass::Weeks => 2 * WEEK_MS,
        }
    }

    fn sample_threads() -> Vec<ResumeThread> {
        vec![
            thread("recent", 0.9, 0.8, NOW - 10 * MINUTE_MS),
            thread("middle", 0.5, 0.4, NOW - 2 * DAY_MS),
            thread("stale", 1.0, 1.0, NOW - 30 * DAY_MS),
        ]
    }

    fn assert_clean(text: &str) {
        for phrase in FORBIDDEN_PHRASES {
            assert!(
                !text.contains(phrase),
                "forbidden guilt phrase {phrase:?} present in {text:?}"
            );
        }
    }

    #[test]
    fn separation_classification_boundaries() {
        let cases = [
            (NOW, NOW, SeparationClass::Brief),
            (NOW - (HOUR_MS - 1), NOW, SeparationClass::Brief),
            (NOW - HOUR_MS, NOW, SeparationClass::Hours),
            (NOW - (DAY_MS - 1), NOW, SeparationClass::Hours),
            (NOW - DAY_MS, NOW, SeparationClass::Days),
            (NOW - (WEEK_MS - 1), NOW, SeparationClass::Days),
            (NOW - WEEK_MS, NOW, SeparationClass::Weeks),
            (0, NOW, SeparationClass::Weeks),
            (0, u64::MAX, SeparationClass::Weeks),
        ];
        for (last_seen_ms, now_ms, expected) in cases {
            assert_eq!(
                SeparationClass::classify(last_seen_ms, now_ms),
                expected,
                "last_seen={last_seen_ms}, now={now_ms}"
            );
        }
    }

    #[test]
    fn future_last_seen_degrades_to_brief_without_panic() {
        assert_eq!(
            SeparationClass::classify(NOW + 1, NOW),
            SeparationClass::Brief
        );
        assert_eq!(
            SeparationClass::classify(u64::MAX, 0),
            SeparationClass::Brief
        );
        let plan = plan_for(0, &[], ReunionPolicy::default());
        assert_eq!(plan.class, SeparationClass::Brief);
        assert_eq!(
            plan.greeting,
            GreetingStyle::QuietResume.template(),
            "默认 GentleCheckIn 在短分离上应收敛为 QuietResume"
        );
    }

    #[test]
    fn greeting_style_respects_class_ceiling() {
        let ceilings = [
            (SeparationClass::Brief, GreetingStyle::QuietResume),
            (SeparationClass::Hours, GreetingStyle::GentleCheckIn),
            (SeparationClass::Days, GreetingStyle::WarmReturn),
            (SeparationClass::Weeks, GreetingStyle::GentleCheckIn),
        ];
        for (class, ceiling) in ceilings {
            let policy = make_policy(GreetingStyle::WarmReturn, true, 3, DAY_MS);
            let plan = plan_for(gap_for(class), &[], policy);
            assert_eq!(plan.class, class);
            assert_eq!(plan.policy, policy, "原始 policy 必须原样留档");
            assert_eq!(
                plan.greeting,
                ceiling.template(),
                "WarmReturn policy 必须被 {class:?} 上限压到 {ceiling:?}"
            );
        }
    }

    #[test]
    fn quiet_policy_is_never_escalated() {
        for class in ALL_CLASSES {
            let plan = plan_for(
                gap_for(class),
                &[],
                make_policy(GreetingStyle::QuietResume, true, 3, DAY_MS),
            );
            assert_eq!(plan.greeting, GreetingStyle::QuietResume.template());
        }
    }

    #[test]
    fn long_separation_does_not_force_warmth() {
        let plan = plan_for(
            2 * WEEK_MS,
            &[],
            make_policy(GreetingStyle::WarmReturn, true, 3, DAY_MS),
        );
        assert_eq!(plan.class, SeparationClass::Weeks);
        assert_eq!(plan.greeting, GreetingStyle::GentleCheckIn.template());
        assert_ne!(plan.greeting, GreetingStyle::WarmReturn.template());
        assert!(plan.rationale.contains("不强行热情"));
    }

    #[test]
    fn short_separation_does_not_stage_a_performance() {
        for style in [GreetingStyle::GentleCheckIn, GreetingStyle::WarmReturn] {
            let plan = plan_for(10 * MINUTE_MS, &[], make_policy(style, true, 3, DAY_MS));
            assert_eq!(plan.class, SeparationClass::Brief);
            assert_eq!(plan.greeting, GreetingStyle::QuietResume.template());
            assert!(plan.rationale.contains("不演戏"));
        }
    }

    #[test]
    fn no_guilt_or_accusation_in_any_plan() {
        assert!(
            FORBIDDEN_PHRASES.len() >= 8,
            "blacklist must contain at least 8 phrases"
        );
        let threads = sample_threads();
        for class in ALL_CLASSES {
            for style in ALL_STYLES {
                for resume_threads in [true, false] {
                    for max_threads in [0, 1, 3] {
                        for max_age_ms in [0, DAY_MS, 30 * DAY_MS] {
                            let policy =
                                make_policy(style, resume_threads, max_threads, max_age_ms);
                            let plan = plan_for(gap_for(class), &threads, policy);
                            assert_clean(&plan.greeting);
                            assert_clean(&plan.rationale);
                            for thread in &plan.threads {
                                assert_clean(&thread.summary);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn threads_rank_by_weighted_score() {
        let threads = vec![
            thread("a", 1.0, 0.0, NOW),
            thread("b", 0.0, 1.0, NOW),
            thread("c", 0.5, 0.5, NOW),
            thread("d", 0.9, 0.9, NOW),
        ];
        let plan = plan_for(
            5 * HOUR_MS,
            &threads,
            make_policy(GreetingStyle::GentleCheckIn, true, 10, DAY_MS),
        );
        let ids: Vec<&str> = plan.threads.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["d", "a", "c", "b"]);
    }

    #[test]
    fn ranking_is_stable_across_input_order_and_ties() {
        let forward = vec![thread("b", 0.5, 0.5, NOW), thread("a", 0.5, 0.5, NOW)];
        let backward = vec![thread("a", 0.5, 0.5, NOW), thread("b", 0.5, 0.5, NOW)];
        for threads in [forward, backward] {
            let plan = plan_for(
                5 * HOUR_MS,
                &threads,
                make_policy(GreetingStyle::GentleCheckIn, true, 10, DAY_MS),
            );
            let ids: Vec<&str> = plan.threads.iter().map(|t| t.id.as_str()).collect();
            assert_eq!(ids, ["a", "b"], "同分按 id 升序，且与输入顺序无关");
        }
    }

    #[test]
    fn duplicate_ids_fall_back_to_summary_order() {
        let mut alpha = thread("same", 0.5, 0.5, NOW);
        alpha.summary = "a".to_owned();
        let mut beta = thread("same", 0.5, 0.5, NOW);
        beta.summary = "b".to_owned();
        let orders = [
            vec![beta.clone(), alpha.clone()],
            vec![alpha.clone(), beta.clone()],
        ];
        for threads in orders {
            let plan = plan_for(
                5 * HOUR_MS,
                &threads,
                make_policy(GreetingStyle::GentleCheckIn, true, 10, DAY_MS),
            );
            let summaries: Vec<&str> = plan.threads.iter().map(|t| t.summary.as_str()).collect();
            assert_eq!(summaries, ["a", "b"], "同 id 同分时按 summary 升序兜底");
        }
    }

    #[test]
    fn newer_threads_outrank_older_at_equal_weights() {
        let threads = vec![
            thread("older", 0.5, 0.5, NOW - 6 * DAY_MS),
            thread("newer", 0.5, 0.5, NOW),
        ];
        let plan = plan_for(
            5 * HOUR_MS,
            &threads,
            make_policy(GreetingStyle::GentleCheckIn, true, 10, 7 * DAY_MS),
        );
        let ids: Vec<&str> = plan.threads.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["newer", "older"]);
    }

    #[test]
    fn expired_threads_are_filtered_before_ranking() {
        let threads = vec![
            thread("fresh", 0.5, 0.5, NOW),
            thread("edge", 0.5, 0.5, NOW - DAY_MS),
            thread("stale", 1.0, 1.0, NOW - DAY_MS - 1),
        ];
        let plan = plan_for(
            5 * HOUR_MS,
            &threads,
            make_policy(GreetingStyle::GentleCheckIn, true, 10, DAY_MS),
        );
        let ids: Vec<&str> = plan.threads.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["fresh", "edge"], "边界时刻保留，更早一条过期");
        assert!(plan.rationale.contains("过期 1"));
    }

    #[test]
    fn max_threads_truncates_after_ranking() {
        let threads = vec![
            thread("t1", 1.0, 0.0, NOW),
            thread("t2", 0.8, 0.0, NOW),
            thread("t3", 0.6, 0.0, NOW),
            thread("t4", 0.4, 0.0, NOW),
            thread("t5", 0.2, 0.0, NOW),
        ];
        let plan = plan_for(
            5 * HOUR_MS,
            &threads,
            make_policy(GreetingStyle::GentleCheckIn, true, 2, DAY_MS),
        );
        let ids: Vec<&str> = plan.threads.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["t1", "t2"]);
        assert!(plan.rationale.contains("截断 3"));
    }

    #[test]
    fn empty_threads_degrade_gracefully() {
        let plan = plan_for(5 * HOUR_MS, &[], ReunionPolicy::default());
        assert_eq!(plan.class, SeparationClass::Hours);
        assert!(plan.threads.is_empty());
        assert!(!plan.greeting.is_empty());
        assert_clean(&plan.greeting);
        assert_clean(&plan.rationale);
        assert!(plan.rationale.contains("线索 0/0"));
    }

    #[test]
    fn disabled_resume_empties_threads_but_keeps_greeting() {
        let threads = sample_threads();
        let plan = plan_for(
            5 * HOUR_MS,
            &threads,
            make_policy(GreetingStyle::GentleCheckIn, false, 10, 30 * DAY_MS),
        );
        assert!(plan.threads.is_empty());
        assert!(!plan.greeting.is_empty());
        assert!(plan.rationale.contains("关闭线索续接"));
    }

    #[test]
    fn zero_max_age_keeps_only_threads_touched_now() {
        let threads = vec![
            thread("fresh", 0.5, 0.5, NOW),
            thread("one_ms_old", 0.5, 0.5, NOW - 1),
        ];
        let plan = plan_for(
            5 * HOUR_MS,
            &threads,
            make_policy(GreetingStyle::GentleCheckIn, true, 10, 0),
        );
        let ids: Vec<&str> = plan.threads.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["fresh"]);
        assert!(plan.rationale.contains("过期 1"));
    }

    #[test]
    fn plan_is_deterministic() {
        let threads = sample_threads();
        let policy = make_policy(GreetingStyle::WarmReturn, true, 2, 7 * DAY_MS);
        let first = plan_for(3 * DAY_MS, &threads, policy);
        let second = plan_for(3 * DAY_MS, &threads, policy);
        assert_eq!(first, second);
    }

    #[test]
    fn plan_round_trips_through_json() {
        let plan = plan_for(
            3 * DAY_MS,
            &sample_threads(),
            make_policy(GreetingStyle::WarmReturn, true, 2, 7 * DAY_MS),
        );
        let json = serde_json::to_string(&plan).expect("plan must serialize");
        let decoded: ReunionPlan = serde_json::from_str(&json).expect("plan must deserialize");
        assert_eq!(decoded, plan);
    }

    #[test]
    fn rationale_reports_class_and_thread_counts() {
        let threads = vec![
            thread("fresh", 0.5, 0.5, NOW),
            thread("middle", 0.5, 0.5, NOW - 5 * DAY_MS),
            thread("stale", 0.5, 0.5, NOW - 10 * DAY_MS),
        ];
        let plan = plan_for(
            3 * DAY_MS,
            &threads,
            make_policy(GreetingStyle::WarmReturn, true, 3, 7 * DAY_MS),
        );
        assert!(plan.rationale.contains("判为 Days"));
        assert!(plan.rationale.contains("线索 2/3 条入选"));
        assert!(plan.rationale.contains("过期 1"));
        assert!(plan.rationale.contains("截断 0"));
    }
}
