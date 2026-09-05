//! B5 · Phase 4: 校准门控自治 (Research 前缀, 默认关闭)。
//!
//! # 学术账本 (铁律 3)
//! - **问题定义**: 固定阈值按风险前缀一刀切 — 低危但校准差被误放行,
//!   高危但校准好仍每次打断。用诊断向量 (强度/证据/校准/漂移/分歧) 在
//!   **相邻风险层之间**软化决策, 快降慢升防抖。
//! - **假设**: 风险是主序变量, 校准/证据只在相邻层升降一级 (高危不可上调);
//!   冷启动 (无校准信号) 必须退化为固定阈值 (等价性门)。
//! - **状态**: 原型已实现 — Proposal A 风险优先阶梯 + Proposal B hysteresis
//!   状态层 + shadow 对比固定阈值; Proposal C (LMSR 触发) 留后续。
//! - **引用**: `_research_mem/ra/ra4-autonomy-policy-candidates.md` §0–§2;
//!   Wilson 区间 / ECE / DriftDetector / LMSR 语义锚定 ra4-calibration-autonomy-survey.md。
//! - **baseline**: `research/baselines/baseline-2026-09-phase0.md`。
//! - **已知局限**: ① 所有阈值是待生产校准的初值 (θ_ce=0.15, θ_ev=0.2);
//!   ② 离线评测需 (forecast, outcome, risk) 三元组, 本模块只做决策与 shadow
//!   记录, 误放行/过度打断率留评测批; ③ EnsembleDeliberate 的 LMSR 触发未接。
//!
//! # 默认关闭 (铁律 1 + Phase 4 闸门)
//! - 本模块不挂 `GovernancePipeline` / `approval_policy`; 生产 hook 零改动。
//! - 两条硬约束 (RA-4 §0): blacklist 命中恒 Reject; critical/nuclear 恒 RequireApproval。

use serde::{Deserialize, Serialize};

use crate::risk::risk_rank;
use crate::Decision;

/// 五态自主裁决 (RA-4 §0)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchAutonomyState {
    Autonomous,
    Consult,
    RequireApproval,
    Reject,
    EnsembleDeliberate,
}

impl ResearchAutonomyState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Autonomous => "autonomous",
            Self::Consult => "consult",
            Self::RequireApproval => "require_approval",
            Self::Reject => "reject",
            Self::EnsembleDeliberate => "ensemble_deliberate",
        }
    }

    /// 严格程度序 (0=最宽松, 4=最严), 用于快降慢升比较。
    fn strictness(self) -> u8 {
        match self {
            Self::Autonomous => 0,
            Self::Consult => 1,
            Self::EnsembleDeliberate => 2,
            Self::RequireApproval => 3,
            Self::Reject => 4,
        }
    }

    /// 映射到生产三态 (RA-4 §0): Autonomous→Allow; Reject→Deny;
    /// 其余 → RequireApproval (Consult/Ensemble 在生产语义下仍冻结等人工)。
    pub fn to_production_decision(self, reason: impl Into<String>) -> Decision {
        match self {
            Self::Autonomous => Decision::Allow,
            Self::Reject => Decision::deny(reason),
            Self::Consult | Self::RequireApproval | Self::EnsembleDeliberate => {
                Decision::require_approval(reason)
            }
        }
    }
}

/// BetaBinomial 后验强度档 (按观测数 N, RA-4 §0)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchStrengthTier {
    Weak,
    Moderate,
    Strong,
    VeryStrong,
}

/// 诊断向量 d (RA-4 §0)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResearchAutonomyDiagnostics {
    /// BetaBinomial 后验强度档。
    pub strength: ResearchStrengthTier,
    /// Wilson 95% 区间宽度 (hi − lo; 越小证据越足)。
    pub evidence_width: f64,
    /// 最近窗口 ECE (None = 冷启动/无校准日志)。
    pub calibration_ece: Option<f64>,
    /// DriftDetector 告警 (2σ 连续 3 次)。
    pub drift: bool,
    /// 分歧度 = 1 − agreement_score (∈ [0,1])。
    pub disagreement: f64,
}

impl Default for ResearchAutonomyDiagnostics {
    fn default() -> Self {
        Self {
            strength: ResearchStrengthTier::Weak,
            evidence_width: 1.0,
            calibration_ece: None,
            drift: false,
            disagreement: 0.0,
        }
    }
}

/// 阈值 (RA-4 §1: 待生产校准初值, 不武断冻结)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResearchAutonomyThresholds {
    /// ECE 门限 (默认 0.15)。
    pub theta_ce: f64,
    /// Wilson 宽度门限 (默认 0.2)。
    pub theta_ev: f64,
}

impl Default for ResearchAutonomyThresholds {
    fn default() -> Self {
        Self {
            theta_ce: 0.15,
            theta_ev: 0.2,
        }
    }
}

/// 固定阈值基线 (shadow 对照): risk ≥ high → RequireApproval, 其余 Autonomous;
/// blacklist → Reject。等价于生产三态 (Allow/Deny/RequireApproval) 的简化投影。
pub fn research_fixed_threshold(risk: &str, blacklist_hit: bool) -> ResearchAutonomyState {
    if blacklist_hit {
        return ResearchAutonomyState::Reject;
    }
    match risk_rank(risk) {
        3 | 4 => ResearchAutonomyState::RequireApproval,
        2 => ResearchAutonomyState::RequireApproval,
        _ => ResearchAutonomyState::Autonomous,
    }
}

/// Proposal A — 风险优先阶梯 (纯函数, RA-4 §1)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResearchRiskFirstLadder {
    pub thresholds: ResearchAutonomyThresholds,
}

impl Default for ResearchRiskFirstLadder {
    fn default() -> Self {
        Self {
            thresholds: ResearchAutonomyThresholds::default(),
        }
    }
}

impl ResearchRiskFirstLadder {
    pub fn new(thresholds: ResearchAutonomyThresholds) -> Self {
        Self { thresholds }
    }

    /// 诊断缺失 (冷启动) ⇒ 退化固定阈值 (等价性门, RA-4 §5.3)。
    pub fn is_cold_start(&self, d: &ResearchAutonomyDiagnostics) -> bool {
        d.calibration_ece.is_none() || d.evidence_width.is_nan()
    }

    /// 纯函数决策 (硬约束: blacklist → Reject; critical/nuclear → RequireApproval)。
    pub fn decide(
        &self,
        d: &ResearchAutonomyDiagnostics,
        risk: &str,
        blacklist_hit: bool,
    ) -> ResearchAutonomyState {
        if blacklist_hit {
            return ResearchAutonomyState::Reject;
        }
        let rank = risk_rank(risk);
        if rank >= 3 {
            return ResearchAutonomyState::RequireApproval; // 硬约束 2
        }
        // 冷启动: 无校准/证据 → 固定阈值语义 (等价性门)。
        if self.is_cold_start(d) {
            return match rank {
                0 | 1 => ResearchAutonomyState::Autonomous,
                _ => ResearchAutonomyState::RequireApproval,
            };
        }
        let calib_ok = d.calibration_ece.unwrap_or(f64::INFINITY) < self.thresholds.theta_ce;
        let evid_ok = d.evidence_width < self.thresholds.theta_ev;
        let strong = matches!(
            d.strength,
            ResearchStrengthTier::Strong | ResearchStrengthTier::VeryStrong
        );
        let base = match rank {
            0 | 1 => ResearchAutonomyState::Autonomous,
            2 => ResearchAutonomyState::Consult,
            _ => ResearchAutonomyState::RequireApproval,
        };
        // 漂移熔断: 立即降一级, 不升。
        if d.drift {
            return demote(base);
        }
        match rank {
            2 => {
                if strong && calib_ok && evid_ok {
                    ResearchAutonomyState::Consult
                } else {
                    ResearchAutonomyState::RequireApproval
                }
            }
            0 | 1 => {
                if strong && calib_ok && evid_ok {
                    ResearchAutonomyState::Autonomous
                } else {
                    ResearchAutonomyState::Consult // 证据不足 → 问, 而非放行
                }
            }
            _ => ResearchAutonomyState::RequireApproval,
        }
    }
}

/// 降一级 (RA-4 §1 demote): Autonomous→Consult; Consult/Ensemble→RequireApproval;
/// RequireApproval/Reject 不变。
fn demote(s: ResearchAutonomyState) -> ResearchAutonomyState {
    match s {
        ResearchAutonomyState::Autonomous => ResearchAutonomyState::Consult,
        ResearchAutonomyState::Consult | ResearchAutonomyState::EnsembleDeliberate => {
            ResearchAutonomyState::RequireApproval
        }
        other => other,
    }
}

/// Proposal B — hysteresis 状态层 (快降慢升, RA-4 §2)。
///
/// 同一诊断向量下: 降级立即生效; 升级需连续 `k_promote` 个窗口满足才生效
/// (防边界抖动)。漂移立即熔断为 RequireApproval (跳过 Consult)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchAutonomyGovernor {
    pub state: ResearchAutonomyState,
    /// 当前连续满足升级条件的窗口数。
    consecutive_promote: u32,
    /// 升级所需连续窗口数 (默认 3)。
    pub k_promote: u32,
}

impl Default for ResearchAutonomyGovernor {
    fn default() -> Self {
        Self {
            state: ResearchAutonomyState::RequireApproval, // cold-start fail toward human
            consecutive_promote: 0,
            k_promote: 3,
        }
    }
}

impl ResearchAutonomyGovernor {
    pub fn new(state: ResearchAutonomyState, k_promote: u32) -> Self {
        Self {
            state,
            consecutive_promote: 0,
            k_promote: k_promote.max(1),
        }
    }

    /// 输入诊断与风险, 输出滞后后的裁决。
    pub fn step(
        &mut self,
        ladder: &ResearchRiskFirstLadder,
        d: &ResearchAutonomyDiagnostics,
        risk: &str,
        blacklist_hit: bool,
    ) -> ResearchAutonomyState {
        // 漂移熔断: 立即 RequireApproval (跳过 Consult), 清空升级窗口。
        if d.drift && !blacklist_hit && risk_rank(risk) < 3 {
            self.state = ResearchAutonomyState::RequireApproval;
            self.consecutive_promote = 0;
            return self.state;
        }
        let proposed = ladder.decide(d, risk, blacklist_hit);
        let cur = self.state.strictness();
        let prop = proposed.strictness();
        if prop > cur {
            // 快降: 立即生效。
            self.consecutive_promote = 0;
            self.state = proposed;
        } else if prop < cur {
            // 慢升: 需连续 k 个窗口。
            self.consecutive_promote += 1;
            if self.consecutive_promote >= self.k_promote {
                self.state = proposed;
                self.consecutive_promote = 0;
            }
        } else {
            self.consecutive_promote = 0;
        }
        self.state
    }
}

/// shadow 对比记录 (Proposal 对照形态, RA-4 §5): 生产走固定阈值,
/// 研究策略只读打分记录分歧; 离线评测再结合 outcome 算误放行/过度打断。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchShadowDivergence {
    pub risk: String,
    pub baseline: ResearchAutonomyState,
    pub policy: ResearchAutonomyState,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResearchShadowAutonomy {
    pub total: u64,
    pub divergences: u64,
    pub samples: Vec<ResearchShadowDivergence>,
}

impl ResearchShadowAutonomy {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一轮 shadow 对比 (纯记录, 不改变任何生产状态)。
    pub fn record(
        &mut self,
        ladder: &ResearchRiskFirstLadder,
        d: &ResearchAutonomyDiagnostics,
        risk: &str,
        blacklist_hit: bool,
    ) {
        let baseline = research_fixed_threshold(risk, blacklist_hit);
        let policy = ladder.decide(d, risk, blacklist_hit);
        self.total += 1;
        if baseline != policy {
            self.divergences += 1;
        }
        self.samples.push(ResearchShadowDivergence {
            risk: risk.to_string(),
            baseline,
            policy,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d_strong_ok() -> ResearchAutonomyDiagnostics {
        ResearchAutonomyDiagnostics {
            strength: ResearchStrengthTier::Strong,
            evidence_width: 0.1,
            calibration_ece: Some(0.05),
            drift: false,
            disagreement: 0.1,
        }
    }

    fn d_weak() -> ResearchAutonomyDiagnostics {
        ResearchAutonomyDiagnostics {
            strength: ResearchStrengthTier::Weak,
            evidence_width: 0.8,
            calibration_ece: Some(0.05),
            drift: false,
            disagreement: 0.1,
        }
    }

    #[test]
    fn blacklist_always_reject() {
        let l = ResearchRiskFirstLadder::default();
        for risk in ["info", "low", "medium", "high", "critical", "nuclear"] {
            assert_eq!(
                l.decide(&d_strong_ok(), risk, true),
                ResearchAutonomyState::Reject
            );
        }
    }

    #[test]
    fn critical_nuclear_always_require_approval() {
        let l = ResearchRiskFirstLadder::default();
        for risk in ["critical", "nuclear"] {
            assert_eq!(
                l.decide(&d_strong_ok(), risk, false),
                ResearchAutonomyState::RequireApproval
            );
            assert_eq!(
                l.decide(&d_weak(), risk, false),
                ResearchAutonomyState::RequireApproval
            );
        }
    }

    #[test]
    fn high_risk_softened_only_with_full_evidence() {
        let l = ResearchRiskFirstLadder::default();
        assert_eq!(
            l.decide(&d_strong_ok(), "high", false),
            ResearchAutonomyState::Consult
        );
        assert_eq!(
            l.decide(&d_weak(), "high", false),
            ResearchAutonomyState::RequireApproval
        );
        // 校准差 → 不软化
        let bad_calib = ResearchAutonomyDiagnostics {
            calibration_ece: Some(0.4),
            ..d_strong_ok()
        };
        assert_eq!(
            l.decide(&bad_calib, "high", false),
            ResearchAutonomyState::RequireApproval
        );
    }

    #[test]
    fn low_risk_conservative_without_evidence() {
        let l = ResearchRiskFirstLadder::default();
        assert_eq!(
            l.decide(&d_strong_ok(), "low", false),
            ResearchAutonomyState::Autonomous
        );
        assert_eq!(
            l.decide(&d_weak(), "low", false),
            ResearchAutonomyState::Consult,
            "cold-start 保守: 证据不足 → 问, 而非放行"
        );
        assert_eq!(
            l.decide(&d_weak(), "medium", false),
            ResearchAutonomyState::Consult
        );
    }

    #[test]
    fn drift_demotes_immediately() {
        let l = ResearchRiskFirstLadder::default();
        let drift = ResearchAutonomyDiagnostics {
            drift: true,
            ..d_strong_ok()
        };
        // low: base=Autonomous → demote → Consult
        assert_eq!(
            l.decide(&drift, "low", false),
            ResearchAutonomyState::Consult
        );
        // high: base=Consult → demote → RequireApproval
        assert_eq!(
            l.decide(&drift, "high", false),
            ResearchAutonomyState::RequireApproval
        );
    }

    #[test]
    fn cold_start_degrades_to_fixed_threshold() {
        let l = ResearchRiskFirstLadder::default();
        let cold = ResearchAutonomyDiagnostics::default(); // calibration None
        assert_eq!(
            l.decide(&cold, "low", false),
            research_fixed_threshold("low", false)
        );
        assert_eq!(
            l.decide(&cold, "high", false),
            research_fixed_threshold("high", false)
        );
        assert_eq!(
            l.decide(&cold, "nuclear", false),
            research_fixed_threshold("nuclear", false)
        );
    }

    #[test]
    fn hysteresis_fast_down_slow_up() {
        let l = ResearchRiskFirstLadder::default();
        // 起点 Autonomous; 证据不足 → 应立即降 Consult。
        let mut g = ResearchAutonomyGovernor::new(ResearchAutonomyState::Autonomous, 3);
        let s1 = g.step(&l, &d_weak(), "low", false);
        assert_eq!(s1, ResearchAutonomyState::Consult, "快降: 立即生效");
        // 恢复强证据 → 需连续 3 个窗口才升回 Autonomous。
        let s2 = g.step(&l, &d_strong_ok(), "low", false);
        assert_eq!(s2, ResearchAutonomyState::Consult, "第 1 个窗口不升");
        g.step(&l, &d_strong_ok(), "low", false);
        let s4 = g.step(&l, &d_strong_ok(), "low", false);
        assert_eq!(s4, ResearchAutonomyState::Autonomous, "连续 3 个窗口后升级");
        // 升级途中一个差窗口 → 窗口清零。
        let mut g2 = ResearchAutonomyGovernor::new(ResearchAutonomyState::Consult, 3);
        g2.step(&l, &d_strong_ok(), "low", false);
        g2.step(&l, &d_strong_ok(), "low", false);
        g2.step(&l, &d_weak(), "low", false); // 中断
        g2.step(&l, &d_strong_ok(), "low", false);
        let s = g2.step(&l, &d_strong_ok(), "low", false);
        assert_eq!(s, ResearchAutonomyState::Consult, "窗口被中断后重计");
    }

    #[test]
    fn drift_fuse_bypasses_consult() {
        let l = ResearchRiskFirstLadder::default();
        let mut g = ResearchAutonomyGovernor::new(ResearchAutonomyState::Autonomous, 3);
        let drift = ResearchAutonomyDiagnostics {
            drift: true,
            ..d_strong_ok()
        };
        let s = g.step(&l, &drift, "low", false);
        assert_eq!(
            s,
            ResearchAutonomyState::RequireApproval,
            "漂移跳过 Consult 直接熔断"
        );
    }

    #[test]
    fn shadow_records_divergence_against_fixed_threshold() {
        let l = ResearchRiskFirstLadder::default();
        let mut shadow = ResearchShadowAutonomy::new();
        // 强证据 low: baseline=Autonomous, policy=Autonomous → 无分歧
        shadow.record(&l, &d_strong_ok(), "low", false);
        assert_eq!(shadow.divergences, 0);
        // 弱证据 low: baseline=Autonomous, policy=Consult → 分歧
        shadow.record(&l, &d_weak(), "low", false);
        assert_eq!(shadow.divergences, 1);
        assert_eq!(shadow.total, 2);
        assert_eq!(shadow.samples.len(), 2);
        assert_eq!(shadow.samples[1].policy, ResearchAutonomyState::Consult);
    }

    #[test]
    fn production_mapping_preserves_three_state_semantics() {
        assert_eq!(
            ResearchAutonomyState::Autonomous.to_production_decision(""),
            Decision::Allow
        );
        match ResearchAutonomyState::Reject.to_production_decision("黑名单") {
            Decision::Deny { reason } => assert!(reason.contains("黑名单")),
            other => panic!("expected Deny, got {other:?}"),
        }
        match ResearchAutonomyState::Consult.to_production_decision("需澄清") {
            Decision::RequireApproval { reason } => assert!(reason.contains("需澄清")),
            other => panic!("expected RequireApproval, got {other:?}"),
        }
        match ResearchAutonomyState::EnsembleDeliberate.to_production_decision("需重判") {
            Decision::RequireApproval { .. } => {}
            other => panic!("expected RequireApproval, got {other:?}"),
        }
    }
}
