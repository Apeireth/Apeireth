//! CouncilReview — 7 席智囊团审议 + 按住机制 (OTA 阶段 2/7).
//!
//! 设计基线: 阶段 2 §10 智囊团 (7 强制 + 3 生命周期 + 按住 + Synthesis).
//!
//! 7 席 (固定席位, 不可减少; 缺失席位 = 审议不通过):
//! 1. Principle (原则)   — 守 12 键 + 5 项不假装 + E/S/A/M/O 5 层
//! 2. Sovereignty (主权) — 主 + 双洋葱 + 权限发放
//! 3. Continuity (连续性) — 主体连续性 ID + 6 历史流 + D2 §4
//! 4. Evolution (演化)   — Cognitive-Dream + 演化层 + 自我修改边界
//! 5. Relation (关系)    — 关系流 + 端点 + 关系类型
//! 6. Value (价值)       — SGI 主权目标 + 价值层裁决
//! 7. Constraint (约束)  — 双洋葱约束 + 12 键 + 物理隔离
//!
//! 按住机制 (Hold): 当任一席位 StrongDisapprove, 或多席位 Disapprove 比例超过阈值,
//! 升级管线进入按住暂停状态, 等待主人仲裁或撤回.
//!
//! **禁止**: 不修改 apeireth-core 任何已实装类型签名.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::intent::UpgradeIntent;

/// 7 席智囊团固定席位.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CouncilSeat {
    /// 原则席 (Principle) — 12 键 + E/S/A/M/O.
    Principle,
    /// 主权席 (Sovereignty) — 主 + 双洋葱.
    Sovereignty,
    /// 连续性席 (Continuity) — 主体连续性 ID + 6 历史流.
    Continuity,
    /// 演化席 (Evolution) — Cognitive-Dream + 自我修改边界.
    Evolution,
    /// 关系席 (Relation) — 关系流 + 端点.
    Relation,
    /// 价值席 (Value) — SGI 主权目标.
    Value,
    /// 约束席 (Constraint) — 物理隔离.
    Constraint,
}

impl CouncilSeat {
    /// 全部 7 席 (按设计顺序).
    pub const ALL: [CouncilSeat; 7] = [
        CouncilSeat::Principle,
        CouncilSeat::Sovereignty,
        CouncilSeat::Continuity,
        CouncilSeat::Evolution,
        CouncilSeat::Relation,
        CouncilSeat::Value,
        CouncilSeat::Constraint,
    ];

    /// 人类可读名.
    pub const fn semantic_name(self) -> &'static str {
        match self {
            CouncilSeat::Principle => "Principle (原则)",
            CouncilSeat::Sovereignty => "Sovereignty (主权)",
            CouncilSeat::Continuity => "Continuity (连续性)",
            CouncilSeat::Evolution => "Evolution (演化)",
            CouncilSeat::Relation => "Relation (关系)",
            CouncilSeat::Value => "Value (价值)",
            CouncilSeat::Constraint => "Constraint (约束)",
        }
    }
}

/// 单席审议意见.
#[derive(Debug, Clone, PartialEq)]
pub struct CouncilOpinion {
    /// 席位.
    pub seat: CouncilSeat,
    /// 立场.
    pub stance: CouncilStance,
    /// 置信度 (0.0 - 1.0).
    pub confidence: f64,
    /// 推理 (供审计).
    pub reasoning: String,
}

impl CouncilOpinion {
    /// 构造意见.
    pub fn new(
        seat: CouncilSeat,
        stance: CouncilStance,
        confidence: f64,
        reasoning: impl Into<String>,
    ) -> Self {
        Self {
            seat,
            stance,
            confidence,
            reasoning: reasoning.into(),
        }
    }

    /// 是否强反对.
    pub fn is_strong_disapprove(&self) -> bool {
        matches!(self.stance, CouncilStance::StrongDisapprove)
    }
}

/// 立场枚举 (与 CouncilStance 保持细分).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CouncilStance {
    /// 通过.
    Approve,
    /// 条件通过 (备注 conditions).
    ConditionalApprove,
    /// 弃权.
    Abstain,
    /// 反对.
    Disapprove,
    /// 强烈反对 (触发按住).
    StrongDisapprove,
}

impl CouncilStance {
    /// 是否有效反对 (Disapprove + StrongDisapprove).
    pub fn is_disapprove(self) -> bool {
        matches!(
            self,
            CouncilStance::Disapprove | CouncilStance::StrongDisapprove
        )
    }
}

/// 按住触发配置.
#[derive(Debug, Clone)]
pub struct HoldTrigger {
    /// 强反对触发按住的最低席位数 (默认 1, 即任一强反对即按住).
    pub strong_disapprove_threshold: usize,
    /// 是否要求一致反对 (全部 7 席 Disapprove) 才触发按住 (最严苛模式).
    pub require_unanimous_disapprove: bool,
    /// 普通反对触发按住的最低比例 (0.0 - 1.0, 默认 0.3 = 30%).
    pub disapprove_ratio_threshold: f64,
}

impl Default for HoldTrigger {
    fn default() -> Self {
        Self {
            strong_disapprove_threshold: 1,
            require_unanimous_disapprove: false,
            disapprove_ratio_threshold: 0.3,
        }
    }
}

/// 按住动作.
#[derive(Debug, Clone, PartialEq)]
pub enum HoldAction {
    /// 不触发按住 (审议通过, 进入多签).
    NoHold,
    /// 触发按住 (审议暂停, 等待人工仲裁).
    TriggerHold {
        /// 触发原因.
        reason: String,
        /// 触发按住的强反对席位数.
        strong_disapprove_count: usize,
        /// 触发按住的反对总比例.
        disapprove_ratio: f64,
    },
}

/// 7 席审议报告.
#[derive(Debug, Clone)]
pub struct CouncilReport {
    /// 关联 intent ID.
    pub intent_id: uuid::Uuid,
    /// 各席意见.
    pub opinions: Vec<CouncilOpinion>,
    /// 缺席席位 (审议不通过, 因为 7 席缺一不可).
    pub missing_seats: Vec<CouncilSeat>,
    /// 按住动作.
    pub hold: HoldAction,
    /// 审议完成时间.
    pub reviewed_at: i64,
}

impl CouncilReport {
    /// 审议是否通过 (无缺席 + 按住 = NoHold + 无强反对).
    pub fn is_approved(&self) -> bool {
        self.missing_seats.is_empty()
            && matches!(self.hold, HoldAction::NoHold)
            && !self.opinions.iter().any(|o| o.is_strong_disapprove())
    }

    /// 反对总比例.
    pub fn disapprove_ratio(&self) -> f64 {
        let total = self.opinions.len().max(1) as f64;
        let dis = self
            .opinions
            .iter()
            .filter(|o| o.stance.is_disapprove())
            .count() as f64;
        dis / total
    }
}

/// 7 席审议器.
pub struct CouncilReviewer {
    trigger: HoldTrigger,
}

impl CouncilReviewer {
    /// 构造默认审议器.
    pub fn new() -> Self {
        Self::default()
    }

    /// 自定义按住触发配置.
    pub fn with_trigger(trigger: HoldTrigger) -> Self {
        Self { trigger }
    }

    /// 审议单个 intent — 接收已收集的 opinions.
    pub fn review(&self, intent: &UpgradeIntent, opinions: Vec<CouncilOpinion>) -> CouncilReport {
        let mut seen: HashMap<CouncilSeat, bool> = HashMap::new();
        let mut missing = Vec::new();
        for seat in CouncilSeat::ALL {
            if !opinions.iter().any(|o| o.seat == seat) {
                missing.push(seat);
            }
            seen.insert(seat, true);
        }

        let report_for_hold = CouncilReport {
            intent_id: intent.id,
            opinions: opinions.clone(),
            missing_seats: missing.clone(),
            hold: HoldAction::NoHold,
            reviewed_at: chrono::Utc::now().timestamp(),
        };

        let hold = evaluate_hold(&report_for_hold, &self.trigger);

        CouncilReport {
            intent_id: intent.id,
            opinions,
            missing_seats: missing,
            hold,
            reviewed_at: chrono::Utc::now().timestamp(),
        }
    }
}

impl Default for CouncilReviewer {
    fn default() -> Self {
        Self {
            trigger: HoldTrigger::default(),
        }
    }
}

/// 按住评估函数 — 纯函数, 可独立测试.
pub fn evaluate_hold(report: &CouncilReport, trigger: &HoldTrigger) -> HoldAction {
    if !report.missing_seats.is_empty() {
        return HoldAction::TriggerHold {
            reason: format!(
                "{} seat(s) missing: {:?}",
                report.missing_seats.len(),
                report.missing_seats
            ),
            strong_disapprove_count: 0,
            disapprove_ratio: report.disapprove_ratio(),
        };
    }

    let strong = report
        .opinions
        .iter()
        .filter(|o| o.is_strong_disapprove())
        .count();
    if strong >= trigger.strong_disapprove_threshold.max(1) {
        return HoldAction::TriggerHold {
            reason: format!("{strong} seat(s) StrongDisapprove"),
            strong_disapprove_count: strong,
            disapprove_ratio: report.disapprove_ratio(),
        };
    }

    if trigger.require_unanimous_disapprove && report.disapprove_ratio() >= 1.0 - f64::EPSILON {
        return HoldAction::TriggerHold {
            reason: "unanimous disapprove (strict mode)".into(),
            strong_disapprove_count: strong,
            disapprove_ratio: report.disapprove_ratio(),
        };
    }

    let ratio = report.disapprove_ratio();
    if ratio >= trigger.disapprove_ratio_threshold && strong == 0 {
        // 普通反对比例触发按住, 但不能与 StrongDisapprove 同时计入
        return HoldAction::TriggerHold {
            reason: format!("disapprove ratio {ratio:.2} >= threshold"),
            strong_disapprove_count: 0,
            disapprove_ratio: ratio,
        };
    }

    HoldAction::NoHold
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn sample_intent() -> UpgradeIntent {
        UpgradeIntent::new(
            Uuid::new_v4(),
            "v1.1.0",
            "v1.0.0",
            crate::manifest::UpgradeKind::Patch,
            "carrier-a",
            "fix memory leak",
        )
    }

    fn all_approve() -> Vec<CouncilOpinion> {
        CouncilSeat::ALL
            .iter()
            .map(|s| CouncilOpinion::new(*s, CouncilStance::Approve, 0.9, "ok"))
            .collect()
    }

    #[test]
    fn council_seat_all_has_seven() {
        assert_eq!(CouncilSeat::ALL.len(), 7);
        let mut names: Vec<&str> = CouncilSeat::ALL.iter().map(|s| s.semantic_name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 7, "7 席必须唯一");
    }

    #[test]
    fn council_seat_semantic_name_non_empty() {
        for s in CouncilSeat::ALL {
            assert!(!s.semantic_name().is_empty());
        }
    }

    #[test]
    fn reviewer_full_approve_no_hold() {
        let r = CouncilReviewer::new();
        let intent = sample_intent();
        let report = r.review(&intent, all_approve());
        assert!(report.is_approved());
        assert!(report.missing_seats.is_empty());
        assert!(matches!(report.hold, HoldAction::NoHold));
    }

    #[test]
    fn reviewer_missing_seat_triggers_hold() {
        let r = CouncilReviewer::new();
        let intent = sample_intent();
        // 只给 6 席意见, 缺 Constraint 席
        let mut ops = all_approve();
        ops.retain(|o| o.seat != CouncilSeat::Constraint);
        let report = r.review(&intent, ops);
        assert!(!report.is_approved());
        assert_eq!(report.missing_seats, vec![CouncilSeat::Constraint]);
        assert!(matches!(report.hold, HoldAction::TriggerHold { .. }));
    }

    #[test]
    fn reviewer_one_strong_disapprove_triggers_hold() {
        let r = CouncilReviewer::new();
        let intent = sample_intent();
        let mut ops = all_approve();
        ops[2] = CouncilOpinion::new(
            CouncilSeat::Continuity,
            CouncilStance::StrongDisapprove,
            0.95,
            "violates 6-stream append-only",
        );
        let report = r.review(&intent, ops);
        assert!(!report.is_approved());
        match report.hold {
            HoldAction::TriggerHold {
                strong_disapprove_count,
                ..
            } => assert_eq!(strong_disapprove_count, 1),
            _ => panic!("expected TriggerHold"),
        }
    }

    #[test]
    fn reviewer_disapprove_ratio_triggers_hold() {
        // 3/7 = 0.428 >= 0.3 阈值, 触发按住
        let r = CouncilReviewer::new();
        let intent = sample_intent();
        let mut ops = all_approve();
        ops[0] = CouncilOpinion::new(CouncilSeat::Principle, CouncilStance::Disapprove, 0.7, "x");
        ops[1] = CouncilOpinion::new(
            CouncilSeat::Sovereignty,
            CouncilStance::Disapprove,
            0.7,
            "x",
        );
        ops[2] = CouncilOpinion::new(CouncilSeat::Continuity, CouncilStance::Disapprove, 0.7, "x");
        let report = r.review(&intent, ops);
        assert!(report.disapprove_ratio() > 0.3);
        assert!(matches!(report.hold, HoldAction::TriggerHold { .. }));
    }

    #[test]
    fn reviewer_low_ratio_no_hold() {
        // 1/7 = 0.14 < 0.3 阈值, 不触发按住
        let r = CouncilReviewer::new();
        let intent = sample_intent();
        let mut ops = all_approve();
        ops[0] = CouncilOpinion::new(CouncilSeat::Principle, CouncilStance::Disapprove, 0.7, "x");
        let report = r.review(&intent, ops);
        assert!(matches!(report.hold, HoldAction::NoHold));
    }

    #[test]
    fn council_stance_is_disapprove() {
        assert!(!CouncilStance::Approve.is_disapprove());
        assert!(!CouncilStance::ConditionalApprove.is_disapprove());
        assert!(!CouncilStance::Abstain.is_disapprove());
        assert!(CouncilStance::Disapprove.is_disapprove());
        assert!(CouncilStance::StrongDisapprove.is_disapprove());
    }

    #[test]
    fn council_opinion_strong_disapprove_predicate() {
        let op = CouncilOpinion::new(
            CouncilSeat::Principle,
            CouncilStance::StrongDisapprove,
            0.9,
            "x",
        );
        assert!(op.is_strong_disapprove());
        let op2 = CouncilOpinion::new(CouncilSeat::Principle, CouncilStance::Disapprove, 0.9, "x");
        assert!(!op2.is_strong_disapprove());
    }

    #[test]
    fn strict_hold_trigger_requires_unanimous() {
        let r = CouncilReviewer::with_trigger(HoldTrigger {
            strong_disapprove_threshold: 99, // 关闭强反对触发
            require_unanimous_disapprove: true,
            disapprove_ratio_threshold: 0.99,
        });
        let intent = sample_intent();
        let mut ops = all_approve();
        // 6 反对 1 通过, 不是 unanimous, 不触发
        for i in 1..7 {
            ops[i] = CouncilOpinion::new(CouncilSeat::ALL[i], CouncilStance::Disapprove, 0.8, "x");
        }
        let report = r.review(&intent, ops);
        assert!(matches!(report.hold, HoldAction::NoHold));

        // 7/7 unanimous 触发
        let mut ops2 = all_approve();
        for i in 0..7 {
            ops2[i] = CouncilOpinion::new(CouncilSeat::ALL[i], CouncilStance::Disapprove, 0.8, "x");
        }
        let report2 = r.review(&intent, ops2);
        assert!(matches!(report2.hold, HoldAction::TriggerHold { .. }));
    }

    #[test]
    fn report_disapprove_ratio_correct() {
        let r = CouncilReviewer::new();
        let intent = sample_intent();
        let mut ops = all_approve();
        ops[0] = CouncilOpinion::new(CouncilSeat::Principle, CouncilStance::Disapprove, 0.7, "x");
        ops[1] = CouncilOpinion::new(
            CouncilSeat::Sovereignty,
            CouncilStance::StrongDisapprove,
            0.7,
            "x",
        );
        let report = r.review(&intent, ops);
        let ratio = report.disapprove_ratio();
        assert!((ratio - 2.0 / 7.0).abs() < 1e-6, "got {ratio}");
    }
}
