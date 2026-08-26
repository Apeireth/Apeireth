//! `apeireth-companion::simulation` — 合成用户模拟 (参数恢复 + 快速调参工具).
//!
//! **用途** (真人实验前):
//! - 验证节律直方图能否收敛到已知的真实作息 (参数恢复), 支持多峰/多活动.
//! - 扫描 LoopConfig 参数组合, 对比合成用户的「回应率」等客观指标.
//! - 每个时段带**活动标签**, Initiative 携带「你在做什么」的事实流 (供 LLM 意识到
//!   「用户在升级我 / 用户在学什么」).
//!
//! **诚实边界 (重要, 不装)**:
//! - 合成用户**不是真人**; sim-to-real gap 必须记住.
//! - 扫出来的参数是「经验证于合成用户的先验」, 最终校准仍需真人数据.

use std::collections::HashMap;

use chrono::{TimeZone, Utc};

use crate::emergence::{Boundaries, Feedback, LoopConfig};
use crate::organs::AwakeCompanion;
use crate::{Bond, BondStage};
use apeireth_evolution::state::EvolutionState;

/// 自研确定性 RNG (xorshift64*): 可复现, 0 外部依赖.
#[derive(Debug, Clone)]
pub struct XorShift64(u64);

impl XorShift64 {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn gen_bool(&mut self, p: f64) -> bool {
        (self.next_u64() as f64 / u64::MAX as f64) < p
    }
}

/// 作息窗口: (开始分钟, 结束分钟, 出现概率, 活动标签)
pub type ScheduleWindow = (u32, u32, f64, &'static str);

/// 合成用户: **已知** 真实作息 + **已知** 回应函数 (ground truth).
#[derive(Debug, Clone)]
pub struct SimulatedUser {
    rng: XorShift64,
    pub schedule: Vec<ScheduleWindow>,
    /// 活跃时段回应概率 = respond_base + respond_time_bonus; 非活跃时段 = respond_base - 0.2
    pub respond_base: f64,
    pub respond_time_bonus: f64,
}

impl SimulatedUser {
    /// 主人真实作息 (2026-08-15 口述):
    /// 6-8 起床早饭 / 8-12 工程(Apeireth) / 12-13 午饭小说 / 13-14 午觉 /
    /// 14-18 线性代数 / 18-19 吃饭休息 / 19-22 高数 / 22-6 睡觉.
    pub fn real_schedule(seed: u64) -> Self {
        Self {
            rng: XorShift64::new(seed),
            schedule: vec![
                (6 * 60, 8 * 60, 0.8, "6-8点起床吃早饭"),
                (8 * 60, 12 * 60, 0.9, "8-12点在搞Apeireth工程(在升级我)"),
                (12 * 60, 13 * 60, 0.6, "12-13点吃午饭看小说"),
                (13 * 60, 14 * 60, 0.05, "13-14点睡午觉"),
                (14 * 60, 18 * 60, 0.7, "14-18点在学线性代数"),
                (18 * 60, 19 * 60, 0.6, "18-19点吃饭休息"),
                (19 * 60, 22 * 60, 0.7, "19-22点在学高数"),
                (22 * 60, 24 * 60, 0.02, "深夜在睡觉"),
                (0, 6 * 60, 0.02, "深夜在睡觉"),
            ],
            respond_base: 0.55,
            respond_time_bonus: 0.25,
        }
    }

    fn in_active_window(&self, minutes: u32) -> bool {
        self.schedule
            .iter()
            .any(|(s, e, p, _)| minutes >= *s && minutes < *e && *p >= 0.5)
    }

    /// 该时刻的活动标签 (供 Initiative 事实流).
    pub fn activity_label(&self, minutes: u32) -> Option<&'static str> {
        self.schedule
            .iter()
            .find(|(s, e, _, _)| minutes >= *s && minutes < *e)
            .map(|(_, _, _, l)| *l)
    }

    /// 该时刻用户是否「出现」(产生一次交互) — 由真实作息决定.
    pub fn appears_at(&mut self, minutes: u32) -> bool {
        self.schedule
            .iter()
            .any(|(s, e, p, _)| minutes >= *s && minutes < *e && self.rng.gen_bool(*p))
    }

    /// 面对一次主动, 用户回不回 (ground truth 回应函数: 活跃时段更可能回).
    pub fn responds(&mut self, minutes: u32) -> bool {
        let p = if self.in_active_window(minutes) {
            self.respond_base + self.respond_time_bonus
        } else {
            (self.respond_base - 0.2).max(0.05)
        };
        self.rng.gen_bool(p.clamp(0.05, 0.95))
    }
}

/// 一次模拟的结果.
#[derive(Debug, Clone)]
pub struct SimReport {
    pub name: String,
    pub days: u32,
    pub initiatives: u32,
    pub responded: u32,
    pub response_rate: f64,
    /// 节律估计 vs 真实分布的 MAE (48 桶, 越低 = 收敛越好)
    pub rhythm_mae: f64,
    pub final_bond: f64,
    pub policy_retires: u32,
    /// 各活动时段的主动次数 (他更爱在哪个时段开口)
    pub initiatives_by_activity: Vec<(String, u32)>,
    /// 各动作的选用次数 (他更爱做什么)
    pub initiatives_by_action: Vec<(String, u32)>,
}

/// 跑一次模拟 (用主人真实作息).
pub fn run_simulation(name: &str, config: LoopConfig, seed: u64, days: u32) -> SimReport {
    run_simulation_with_user(name, config, SimulatedUser::real_schedule(seed), days)
}

/// 跑一次模拟 (自定义用户).
pub fn run_simulation_with_user(
    name: &str,
    config: LoopConfig,
    mut user: SimulatedUser,
    days: u32,
) -> SimReport {
    let mut bond = Bond::new();
    bond.evolve(BondStage::Trusted, 0.6);
    // Trusted 关系应带基线信任/共鸣 (否则 warmth 冷启动为 0.3 死锁)
    bond.character_mut().trust = 0.5;
    bond.character_mut().resonance = 0.4;
    let mut c = AwakeCompanion::new(bond, Boundaries::default()).with_config(config);

    let mut initiatives = 0u32;
    let mut responded = 0u32;
    let mut retires = 0u32;
    let mut by_activity: HashMap<&'static str, u32> = HashMap::new();
    let mut by_action: HashMap<&'static str, u32> = HashMap::new();

    for day in 1..=days {
        let mut minute = 0u32;
        while minute < 1440 {
            let now = Utc
                .with_ymd_and_hms(2026, 8, day, minute / 60, minute % 60, 0)
                .single()
                .unwrap();
            if user.appears_at(minute) {
                c.observe_interaction(now);
            }
            // 事实流: 把「你现在在做什么」作为上下文交给机制 (真实 daemon 里来自记忆检索)
            let hint = user.activity_label(minute).map(|l| format!("你在{}", l));
            if let Some(init) = c.tick(now, hint) {
                initiatives += 1;
                let label = user.activity_label(minute).unwrap_or("(时段外)");
                *by_activity.entry(label).or_insert(0) += 1;
                *by_action.entry(init.action.id()).or_insert(0) += 1;
                let f = if user.responds(minute) {
                    responded += 1;
                    Feedback::Responded
                } else {
                    Feedback::Ignored
                };
                c.apply_feedback(f, now);
                if c.evolution.current == EvolutionState::Retired {
                    retires += 1;
                }
            }
            minute += 10;
        }
    }

    // 节律 MAE: 估计 vs 真实分布 (48 桶, 真实 = 桶是否落在「活跃」窗内)
    let mut mae = 0.0f64;
    for b in 0..48u32 {
        let m = b * 30 + 15;
        let truth = if user
            .schedule
            .iter()
            .any(|(s, e, p, _)| m >= *s && m < *e && *p >= 0.5)
        {
            1.0
        } else {
            0.0
        };
        let est = c.loop_.rhythm.estimate(m).active_probability;
        mae += (est - truth).abs();
    }
    mae /= 48.0;

    let mut sorted: Vec<(String, u32)> = by_activity
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let mut sorted_action: Vec<(String, u32)> = by_action
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    sorted_action.sort_by(|a, b| b.1.cmp(&a.1));

    SimReport {
        name: name.to_string(),
        days,
        initiatives,
        responded,
        response_rate: if initiatives > 0 {
            f64::from(responded) / f64::from(initiatives)
        } else {
            0.0
        },
        rhythm_mae: mae,
        final_bond: c.depth(),
        policy_retires: retires,
        initiatives_by_activity: sorted,
        initiatives_by_action: sorted_action,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_deterministic() {
        let mut a = XorShift64::new(42);
        let mut b = XorShift64::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn simulation_recovers_real_schedule() {
        let r = run_simulation("t", LoopConfig::default(), 42, 14);
        assert!(r.rhythm_mae < 0.3, "rhythm mae = {}", r.rhythm_mae);
        assert!(r.initiatives > 0, "应该至少主动过一次");
    }

    #[test]
    fn baseline_works_on_real_schedule() {
        let r = run_simulation("base", LoopConfig::default(), 7, 21);
        assert!(r.initiatives > 0, "默认参数应该至少主动一次: {:?}", r);
        assert!(r.response_rate >= 0.6, "默认参数回应率应 >= 0.6: {:?}", r);
    }

    #[test]
    fn nap_window_is_learned_quiet() {
        // 午觉时段 (13:30 桶) 活跃概率应显著低于工程时段 (10:30 桶)
        let mut user = SimulatedUser::real_schedule(11);
        let mut bond = Bond::new();
        bond.evolve(BondStage::Trusted, 0.6);
        bond.character_mut().trust = 0.5;
        bond.character_mut().resonance = 0.4;
        let mut c = AwakeCompanion::new(bond, Boundaries::default());
        for day in 1..=14 {
            let mut minute = 0u32;
            while minute < 1440 {
                let now = Utc
                    .with_ymd_and_hms(2026, 8, day, minute / 60, minute % 60, 0)
                    .single()
                    .unwrap();
                if user.appears_at(minute) {
                    c.observe_interaction(now);
                }
                minute += 10;
            }
        }
        let work_prob = c.loop_.rhythm.estimate(10 * 60 + 30).active_probability; // 10:30 工程
        let nap_prob = c.loop_.rhythm.estimate(13 * 60 + 30).active_probability; // 13:30 午觉
        assert!(work_prob > 0.8, "工程时段应高概率: {}", work_prob);
        assert!(nap_prob < 0.2, "午觉时段应低概率(不打扰): {}", nap_prob);
    }

    #[test]
    fn conservative_threshold_reduces_noise() {
        let baseline = run_simulation("base", LoopConfig::default(), 7, 21);
        let conservative = run_simulation(
            "cons",
            LoopConfig {
                drive_threshold: 0.6,
                ..LoopConfig::default()
            },
            7,
            21,
        );
        assert!(conservative.initiatives <= baseline.initiatives);
    }
}
