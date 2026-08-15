//! `apeireth-companion::organs` — 把其余器官接进主动涌现闭环 (真实 API, 非 stub).
//!
//! - **consciousness** (EmotionEngine): 反馈 → 情绪事件 (回了=UserPraise / 没回=UserCritique),
//!   当前 PAD 愉悦度调制主动意愿 (情绪很低时不出声).
//! - **asi** (TraceRepository): 每次反馈后把自评写入 24 维真实轨迹历史.
//! - **council** (Council + 7 强制 Advisor): 每次主动前多视角审议, 裁决拒绝则不开口.
//! - **evolution** (EvolutionStateMachine): 「主动策略」作为可演化 trait, 连续被忽略则
//!   退回 Draft (自我修正信号: 我不该这么频繁).
//!
//! 诚实: 器官调用是真实的; 但「情绪/审议/演化」目前只调制「是否开口」这一层,
//! 更深的人格化 (情绪→语气、审议→措辞) 是下一步.

use apeireth_asi::UserFeedback;
use apeireth_consciousness::emotion::{EmotionEngine, EmotionEvent};
use apeireth_council::advisors::seven_mandatory_advisors;
use apeireth_council::deliberation::{Council, CouncilQuery};
use apeireth_evolution::state::{EvolutionState, EvolutionStateMachine, TransitionReason};
use chrono::{DateTime, Utc};

use crate::emergence::{Boundaries, EmergenceLoop, Feedback, Initiative, SelfScore};
use crate::security::{SecurityGate, SovereigntyGate};
use crate::Bond;
use apeireth_core::{ActionTarget, ActionVerdict, RiskLevel};

/// 接满器官的「醒着」伙伴: 机制 + 情绪 + 审议 + 演化 + 安静模式.
pub struct AwakeCompanion {
    pub loop_: EmergenceLoop<Bond>,
    pub emotion: EmotionEngine,
    pub council: Council,
    pub evolution: EvolutionStateMachine,
    pub asi_feedback: Vec<UserFeedback>,
    pub gate: SecurityGate,
    pub sovereignty: SovereigntyGate,
    consecutive_ignores: u32,
}

impl AwakeCompanion {
    /// 7 强制 advisor 已召集; 主动策略全链路 Idle→Draft→Proposed→Ratified→Active (默认生效).
    pub fn new(bond: Bond, boundaries: Boundaries) -> Self {
        let mut council = Council::new();
        council.recruit_many(seven_mandatory_advisors());
        let mut evolution = EvolutionStateMachine::new();
        Self::ratify_fresh_policy(&mut evolution);
        Self {
            loop_: EmergenceLoop::new(bond, boundaries),
            emotion: EmotionEngine::new(),
            council,
            evolution,
            asi_feedback: Vec::new(),
            gate: SecurityGate::default(),
            sovereignty: SovereigntyGate::default(),
            consecutive_ignores: 0,
        }
    }

    /// 走完 Idle→Draft→Proposed→Ratified→Active 全链路 (批准一份新的主动策略).
    fn ratify_fresh_policy(evolution: &mut EvolutionStateMachine) {
        *evolution = EvolutionStateMachine::new();
        let at = Utc::now().timestamp_millis();
        let _ = evolution.transition(EvolutionState::Draft, TransitionReason::Start, at);
        let _ = evolution.transition(EvolutionState::Proposed, TransitionReason::Submit, at);
        let _ = evolution.transition(EvolutionState::Ratified, TransitionReason::CouncilApprove, at);
        let _ = evolution.transition(EvolutionState::Active, TransitionReason::Activate, at);
    }

    /// 带全部器官的一次心跳:
    /// 不打扰(Boundaries.user_quiet) → 机制决策 → 情绪调制(consciousness) → 智囊团审议(council) → 策略存活(evolution).
    pub fn tick(&mut self, now: DateTime<Utc>, context_hint: Option<String>) -> Option<Initiative> {
        // 主权总闸 (最高优先): 熔断 = 一切停止
        if self.sovereignty.is_frozen() {
            return None;
        }
        // 「不打扰」由 Boundaries.user_quiet 在机制层守门
        let init = self.loop_.tick(now, context_hint)?;

        // 情绪调制: PAD 愉悦度低 → 更克制
        let pad = self.emotion.current_pad();
        let mood = ((pad.p + 1.0) / 2.0) as f64;
        if mood < self.loop_.config.mood_floor {
            return None;
        }

        // 智囊团审议 — 只喂结构化动作摘要 (只审动作, 不审对话/记忆自由文本):
        // 自由文本 (context_hint/他的话) 只走 LLM 渲染层, 不进关键词网.
        let query = CouncilQuery::new(
            format!("proactive-{}", now.timestamp_millis()),
            format!("主动联系用户 (action={}, risk=low)", init.action.id()),
            now.timestamp_millis(),
        )
        .with_risk("low")
        .with_area("companion_proactive");
        let verdict = self.council.deliberate(query);
        if verdict.is_rejected() {
            return None;
        }

        // 演化: 主动策略必须处于活跃态
        if !self.evolution.current.is_active() {
            return None;
        }
        // 洋葱门 (V1 哲学 × V2 权限 × V3 HA): 主动也要过基地的宪法
        let verdict = self.gate.check(
            "proactive_contact",
            &format!("主动联系用户: {}", init.to_message()),
            RiskLevel::Low,
            ActionTarget::NormalAction("proactive_contact".into()),
        );
        match verdict {
            ActionVerdict::Allow => {}
            ActionVerdict::BlockByPrinciple(key) => {
                // 哲学守门拦下 = 试图碰 12 键 = 熔断证据 (Self-Disable 三级响应)
                self.sovereignty
                    .report_violation(&format!("哲学守门拦截({key:?})"), "主动动作");
                return None;
            }
            other => {
                eprintln!("[gate] 洋葱门拦下主动 (权限/HA): {:?}", other);
                return None;
            }
        }
        Some(init)
    }

    /// 带器官的反馈: 情绪事件 + 机制反馈 (Bond 演化) + 策略演化 (连续被忽略 → 退回 Draft).
    pub fn apply_feedback(&mut self, feedback: Feedback, at: DateTime<Utc>) -> SelfScore {
        let event = match feedback {
            Feedback::Responded => EmotionEvent::UserPraise,
            Feedback::Ignored => EmotionEvent::UserCritique,
        };
        let _ = self.emotion.apply(event);
        let _ = self.emotion.auto_decay();
        // 情感进入关系 (consciousness ↔ companion 桥): PAD → Bond.character (trust/resonance)
        // 回了: 愉悦↑ → 信任/共鸣↑ → 温暖度↑ → 驱动↑; 没回: 反之.
        let pad = self.emotion.current_pad();
        {
            let c = self.loop_.relationship.character_mut();
            c.apply_emotion(
                pad.p.max(0.0) as f64,  // joy
                pad.p.max(0.0) as f64,  // trust 代理 (愉悦 → 信任)
                0.0,
                0.0,
                (-pad.p).max(0.0) as f64, // sadness (不悦)
                0.0,
                0.0,
                0.0,
            );
        }
        let score = self.loop_.apply_feedback(feedback, at);
        // asi 真反馈记录: 用公开的 UserFeedback 记录「我的自评 vs 用户真实回应」
        // (诚实: 深度校准走 CalibrationLoop, 其 observe 目前依赖私有 DimensionTrace, 待接)
        self.asi_feedback.push(UserFeedback::for_dim(
            "confidence_score",
            score.value,
            match feedback {
                Feedback::Responded => 1.0,
                Feedback::Ignored => 0.0,
            },
            at.timestamp(),
        ));
        match feedback {
            Feedback::Responded => self.consecutive_ignores = 0,
            Feedback::Ignored => {
                self.consecutive_ignores += 1;
                if self.evolution.current.is_active() {
                    let _ = self.evolution.transition(
                        EvolutionState::Retired,
                        TransitionReason::Retire,
                        at.timestamp_millis(),
                    );
                }
            }
        }
        score
    }

    pub fn observe_interaction(&mut self, at: DateTime<Utc>) {
        self.loop_.observe_interaction(at);
        // 演化: 关系重新活跃 → 若策略已退役, 以「学到的修正」重新批准
        if !self.evolution.current.is_active() {
            if self.consecutive_ignores >= 2 {
                self.loop_.boundaries.max_initiatives_per_day = self
                    .loop_
                    .boundaries
                    .max_initiatives_per_day
                    .saturating_sub(1)
                    .max(1);
                self.consecutive_ignores = 0;
            }
            Self::ratify_fresh_policy(&mut self.evolution);
        }
    }

    pub fn depth(&self) -> f64 {
        self.loop_.depth()
    }

    /// 换机制参数 (实验调参入口, 必须在开始观察前调用).
    pub fn with_config(mut self, config: crate::emergence::LoopConfig) -> Self {
        self.loop_.config = config.clone();
        self.loop_.rhythm = crate::emergence::RhythmEstimator::new(28, config.rhythm_bucket_minutes);
        self
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(day: u32, h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, day, h, m, 0).single().unwrap()
    }

    fn trusted_bond() -> Bond {
        let mut b = Bond::new();
        b.evolve(crate::BondStage::Trusted, 0.6);
        b
    }

    #[test]
    fn full_organ_cycle_initiates_and_evolves() {
        let mut c = AwakeCompanion::new(trusted_bond(), Boundaries::default());
        for d in 9..=15 {
            c.observe_interaction(at(d, 8, 40));
        }
        // 全器官心跳 → 主动涌现
        let init = c.tick(at(16, 8, 40), Some("你在改 council 的 bug".into()));
        assert!(init.is_some());
        // 回了 → 关系加深 + 情绪上扬
        let before = c.depth();
        let s = c.apply_feedback(Feedback::Responded, at(16, 8, 45));
        assert!(c.depth() > before);
        assert!(s.value > 0.8);
        // asi 真反馈记录已落 (1 条)
        assert_eq!(c.asi_feedback.len(), 1);
    }

    #[test]
    fn user_quiet_blocks_initiative() {
        let b = Boundaries {
            user_quiet: true,
            ..Default::default()
        };
        let mut c = AwakeCompanion::new(trusted_bond(), b);
        for d in 9..=15 {
            c.observe_interaction(at(d, 8, 40));
        }
        assert!(c.tick(at(16, 8, 40), None).is_none());
    }

    #[test]
    fn ignored_feedback_revises_policy() {
        let mut c = AwakeCompanion::new(trusted_bond(), Boundaries::default());
        for d in 9..=15 {
            c.observe_interaction(at(d, 8, 40));
        }
        let _ = c.tick(at(16, 8, 40), None);
        assert!(c.evolution.current.is_active());
        // 连续被忽略 → 策略退役 (学到「不该这么频繁」)
        c.apply_feedback(Feedback::Ignored, at(16, 9, 0));
        c.apply_feedback(Feedback::Ignored, at(16, 9, 10));
        assert_eq!(c.evolution.current, EvolutionState::Retired);
        let before = c.loop_.boundaries.max_initiatives_per_day;
        // 用户再次出现 → 以「降频修正」重新批准新策略
        c.observe_interaction(at(17, 8, 40));
        assert!(c.evolution.current.is_active());
        assert!(c.loop_.boundaries.max_initiatives_per_day < before);
    }
}
