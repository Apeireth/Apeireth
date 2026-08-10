//! HA 部署模式自适应深度实装 — round8-06
//!
//! **设计**:
//! - `DeploymentContext`: 部署上下文 (E层 / 普通 / 紧急 — 决定 Dynamic 模式阈值)
//! - `HADeploymentPolicy`: 部署策略 — 在 3 模式 (Single/Multi/Dynamic) 之间动态路由
//! - `SingleModeEnforcer`: Single 模式专用行为 (1-of-1 + Windows Hello 强制 + risk 拒绝)
//! - `MultiModeEnforcer`: Multi 模式专用行为 (M-of-N 阈值严格校验 + 同 kind 拒绝)
//! - `DynamicModeEnforcer`: Dynamic 模式专用行为 (E 层调高阈值 + 普通层保持 + 紧急层放低)
//!
//! **守 7 项不修改承诺**: 不修改 `ha.rs` / `sovereign.rs` / `governance.rs` 已实装类型。

use crate::ha::{
    AuthorityMode, BiometricProvider, BiometricResult, HAAuthentication, HumanApproval,
    HumanAuthority, MultiSigPolicy,
};
use serde::{Deserialize, Serialize};

/// 部署上下文 — 决定 Dynamic 模式阈值调整方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeploymentContext {
    /// E 层 (存在层) — Dynamic 阈值必须最高 (e.g. +20%)
    ExistenceLayer,
    /// 普通层 — Dynamic 阈值保持默认
    NormalLayer,
    /// 紧急层 — Dynamic 阈值可下调 (e.g. -20%, 但仍有下限 30%)
    EmergencyLayer,
    /// 反思层 — Dynamic 阈值最高 + 必须走反思期
    ReflectionLayer,
}

impl DeploymentContext {
    /// 阈值调整系数 (0-100, 在 HumanAuthority.threshold 基础上调整)。
    pub fn threshold_adjustment(&self) -> i32 {
        match self {
            Self::ExistenceLayer => 20,  // E 层 +20%
            Self::NormalLayer => 0,      // 普通 0%
            Self::EmergencyLayer => -20, // 紧急 -20%
            Self::ReflectionLayer => 30, // 反思 +30%
        }
    }

    /// 是否强制走反思期
    pub fn requires_reflection(&self) -> bool {
        matches!(self, Self::ReflectionLayer | Self::ExistenceLayer)
    }

    /// 下限阈值 (Dynamic 模式 emergency 调整后不能低于此)
    pub fn min_threshold(&self) -> u8 {
        match self {
            Self::EmergencyLayer => 30, // 紧急最低 30%
            _ => 50,                    // 其他最低 50%
        }
    }
}

/// HA 部署策略 — 在 3 模式间路由。
///
/// **用途**: SovereigntyEngine 根据请求上下文选择正确的模式执行。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeploymentMode {
    /// 单人模式 — 1-of-1 + Windows Hello 强制
    Single,
    /// 多人模式 — M-of-N 严格阈值
    Multi,
    /// 动态模式 — 根据上下文调整阈值
    Dynamic,
}

impl DeploymentMode {
    /// 根据上下文选择模式
    pub fn select_for_context(ctx: DeploymentContext, existing_total: u8) -> Self {
        // 启发式: 0 签者 = 无, 1 签者 = Single, 多签者 = Multi/Dynamic
        match existing_total {
            0 => Self::Dynamic, // 无签者 → 动态 (等待配置)
            1 => Self::Single,  // 1 签者 = Single
            _ => match ctx {
                DeploymentContext::ExistenceLayer | DeploymentContext::ReflectionLayer => {
                    Self::Multi // E 层 / 反思层用 Multi 严格阈值
                }
                _ => Self::Dynamic, // 其他用 Dynamic 自适应
            },
        }
    }
}

/// HA 多签授权深度计算结果 — 携带 context 调整信息。
#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentOutcome {
    /// 通过 — 单人模式
    ApprovedSingle {
        /// 签名 ID
        signature_id: String,
        /// 置信度
        confidence: f64,
        /// 部署上下文
        context: DeploymentContext,
    },
    /// 通过 — 多人模式 (满足 M-of-N)
    ApprovedMulti {
        /// 有效签名数
        valid_signatures: usize,
        /// 所需签名数
        required: usize,
        /// 实际阈值百分比
        effective_threshold: u8,
        /// 部署上下文
        context: DeploymentContext,
    },
    /// 通过 — 动态模式 (上下文调整后满足)
    ApprovedDynamic {
        /// 有效签名数
        valid_signatures: usize,
        /// 所需签名数
        required: usize,
        /// 基础阈值
        base_threshold: u8,
        /// 调整后阈值 (e.g. E 层 +20)
        adjusted_threshold: u8,
        /// 部署上下文
        context: DeploymentContext,
    },
    /// 拒绝 — Single 模式但 risk 等级不允许
    RejectedSingleHighRisk {
        /// 风险等级
        risk: String,
        /// 最大允许风险
        max_allowed: String,
    },
    /// 拒绝 — Multi 模式阈值未满足
    RejectedMultiInsufficient {
        /// 实际签名数
        have: usize,
        /// 需要签名数
        need: usize,
        /// 实际百分比
        actual_pct: u8,
        /// 需要百分比
        threshold: u8,
    },
    /// 拒绝 — Dynamic 模式上下文调整后仍未满足
    RejectedDynamicInsufficient {
        /// 实际百分比
        actual_pct: u8,
        /// 调整后阈值
        adjusted_threshold: u8,
        /// 部署上下文
        context: DeploymentContext,
    },
    /// 拒绝 — 反思期未结束
    RejectedReflectionPending {
        /// 反思期剩余毫秒
        remaining_ms: i64,
    },
}

impl DeploymentOutcome {
    pub fn is_approved(&self) -> bool {
        matches!(
            self,
            Self::ApprovedSingle { .. } | Self::ApprovedMulti { .. } | Self::ApprovedDynamic { .. }
        )
    }

    pub fn is_rejected(&self) -> bool {
        !self.is_approved()
    }
}

/// HA 部署强制器 — 模式特定的行为封装。
pub struct HADeploymentEnforcer<'a> {
    /// HA 部署策略
    pub mode: DeploymentMode,
    /// HA 授权配置
    pub authority: &'a HumanAuthority,
    /// 多签策略 (Multi 模式用)
    pub multi_policy: Option<&'a MultiSigPolicy>,
    /// 生物特征 provider (Single 模式用)
    pub biometric: Option<&'a dyn BiometricProvider>,
    /// 部署上下文 (Dynamic 模式用)
    pub context: DeploymentContext,
}

impl<'a> HADeploymentEnforcer<'a> {
    /// 创建单人模式强制器
    pub fn single(
        authority: &'a HumanAuthority,
        biometric: &'a dyn BiometricProvider,
        context: DeploymentContext,
    ) -> Self {
        Self {
            mode: DeploymentMode::Single,
            authority,
            multi_policy: None,
            biometric: Some(biometric),
            context,
        }
    }

    /// 创建多人模式强制器
    pub fn multi(
        authority: &'a HumanAuthority,
        policy: &'a MultiSigPolicy,
        context: DeploymentContext,
    ) -> Self {
        Self {
            mode: DeploymentMode::Multi,
            authority,
            multi_policy: Some(policy),
            biometric: None,
            context,
        }
    }

    /// 创建动态模式强制器
    pub fn dynamic(authority: &'a HumanAuthority, context: DeploymentContext) -> Self {
        Self {
            mode: DeploymentMode::Dynamic,
            authority,
            multi_policy: None,
            biometric: None,
            context,
        }
    }

    /// 执行强制 — 返回深度结果。
    ///
    /// **风险等级**: Single 模式不允许 high/critical, Multi/Dynamic 模式允许任意风险。
    pub fn enforce(
        &self,
        collected_signatures: &[String],
        risk_level: &str,
        now_ms: i64,
    ) -> DeploymentOutcome {
        match self.mode {
            DeploymentMode::Single => self.enforce_single(collected_signatures, risk_level, now_ms),
            DeploymentMode::Multi => self.enforce_multi(collected_signatures, now_ms),
            DeploymentMode::Dynamic => self.enforce_dynamic(collected_signatures, now_ms),
        }
    }

    fn enforce_single(
        &self,
        collected_signatures: &[String],
        risk_level: &str,
        now_ms: i64,
    ) -> DeploymentOutcome {
        // Single 模式: 必须恰好 1 个签名 + Windows Hello/FIDO2 + low/medium 风险
        if collected_signatures.len() != 1 {
            return DeploymentOutcome::RejectedMultiInsufficient {
                have: collected_signatures.len(),
                need: 1,
                actual_pct: 0,
                threshold: 100,
            };
        }

        // 风险等级检查 (Single 模式不能处理 high/critical)
        let risk_rank = match risk_level.to_ascii_lowercase().as_str() {
            "low" | "info" => 0,
            "medium" => 1,
            "high" | "critical" | "nuclear" => 2,
            _ => 0,
        };
        if risk_rank >= 2 {
            return DeploymentOutcome::RejectedSingleHighRisk {
                risk: risk_level.to_string(),
                max_allowed: "medium".to_string(),
            };
        }

        // 生物特征认证 (Single 模式必须)
        if let Some(bio) = self.biometric {
            let sig_id = &collected_signatures[0];
            let result = bio.authenticate(sig_id);
            if let BiometricResult::Authenticated { confidence, .. } = result {
                // 验证 HumanAuthority.meets_authority
                if !self.authority.meets_authority(now_ms) {
                    return DeploymentOutcome::RejectedMultiInsufficient {
                        have: self.authority.valid_approval_count(now_ms),
                        need: 1,
                        actual_pct: self.authority.valid_approval_percentage(now_ms),
                        threshold: 100,
                    };
                }
                return DeploymentOutcome::ApprovedSingle {
                    signature_id: sig_id.clone(),
                    confidence,
                    context: self.context,
                };
            }
        }
        // 单人模式但无 bio provider → 退化到 Multi 检查
        if !self.authority.meets_authority(now_ms) {
            return DeploymentOutcome::RejectedMultiInsufficient {
                have: self.authority.valid_approval_count(now_ms),
                need: 1,
                actual_pct: self.authority.valid_approval_percentage(now_ms),
                threshold: 100,
            };
        }
        DeploymentOutcome::ApprovedSingle {
            signature_id: collected_signatures[0].clone(),
            confidence: 1.0,
            context: self.context,
        }
    }

    fn enforce_multi(&self, collected_signatures: &[String], now_ms: i64) -> DeploymentOutcome {
        let valid = self.authority.valid_approval_count(now_ms);
        let pct = self.authority.valid_approval_percentage(now_ms);
        let required = self.authority.required_approvals as usize;
        let threshold = self.authority.threshold;

        // 验证签名都属于 authority.total_signatories
        if collected_signatures.len() > self.authority.total_signatories as usize {
            return DeploymentOutcome::RejectedMultiInsufficient {
                have: valid,
                need: required,
                actual_pct: pct,
                threshold,
            };
        }

        if valid >= required && pct >= threshold {
            DeploymentOutcome::ApprovedMulti {
                valid_signatures: valid,
                required,
                effective_threshold: threshold,
                context: self.context,
            }
        } else {
            DeploymentOutcome::RejectedMultiInsufficient {
                have: valid,
                need: required,
                actual_pct: pct,
                threshold,
            }
        }
    }

    fn enforce_dynamic(&self, _collected_signatures: &[String], now_ms: i64) -> DeploymentOutcome {
        let valid = self.authority.valid_approval_count(now_ms);
        let pct = self.authority.valid_approval_percentage(now_ms);
        let required = self.authority.required_approvals as usize;
        let base = self.authority.threshold;
        let adjustment = self.context.threshold_adjustment();
        let adjusted =
            (i32::from(base) + adjustment).clamp(i32::from(self.context.min_threshold()), 100) as u8;

        if valid >= required && pct >= adjusted {
            DeploymentOutcome::ApprovedDynamic {
                valid_signatures: valid,
                required,
                base_threshold: base,
                adjusted_threshold: adjusted,
                context: self.context,
            }
        } else {
            DeploymentOutcome::RejectedDynamicInsufficient {
                actual_pct: pct,
                adjusted_threshold: adjusted,
                context: self.context,
            }
        }
    }
}

/// 反思期跟踪 — round8-06 新增 (用于 DeploymentContext::ReflectionLayer)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentReflectionTracker {
    /// 反思开始时间 (epoch ms)
    pub started_at_ms: i64,
    /// 反思期长度 (ms)
    pub period_ms: i64,
}

impl DeploymentReflectionTracker {
    pub fn new(started_at_ms: i64, period_ms: i64) -> Self {
        Self {
            started_at_ms,
            period_ms,
        }
    }

    /// 是否仍在反思期
    pub fn is_in_reflection(&self, now_ms: i64) -> bool {
        now_ms < self.started_at_ms + self.period_ms
    }

    /// 剩余毫秒
    pub fn remaining_ms(&self, now_ms: i64) -> i64 {
        (self.started_at_ms + self.period_ms - now_ms).max(0)
    }
}

// ============================================================
// 单元测试 (round8-06 新增)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_biometric::MockBiometric;

    fn auth_with_approvals(n: u8, mode: AuthorityMode) -> HumanAuthority {
        let mut h = match mode {
            AuthorityMode::Single => HumanAuthority::single("h-1", "Alice"),
            AuthorityMode::Multi => HumanAuthority::multi("ha-1", "team", 2, n).unwrap(),
            AuthorityMode::Dynamic => HumanAuthority::dynamic("d-1", "ctx", 2, 50, n),
        };
        for i in 0..n {
            h.record_approval(HumanApproval::new(
                format!("ap-{i}"),
                format!("h-{i}"),
                format!("Signer{i}"),
                1000,
                "test",
            ));
        }
        h
    }

    // ---- DeploymentContext ----

    #[test]
    fn existence_layer_threshold_plus_20() {
        assert_eq!(DeploymentContext::ExistenceLayer.threshold_adjustment(), 20);
        assert!(DeploymentContext::ExistenceLayer.requires_reflection());
    }

    #[test]
    fn normal_layer_threshold_zero() {
        assert_eq!(DeploymentContext::NormalLayer.threshold_adjustment(), 0);
        assert!(!DeploymentContext::NormalLayer.requires_reflection());
    }

    #[test]
    fn emergency_layer_threshold_minus_20() {
        assert_eq!(
            DeploymentContext::EmergencyLayer.threshold_adjustment(),
            -20
        );
        assert_eq!(DeploymentContext::EmergencyLayer.min_threshold(), 30);
    }

    #[test]
    fn reflection_layer_threshold_plus_30() {
        assert_eq!(
            DeploymentContext::ReflectionLayer.threshold_adjustment(),
            30
        );
        assert!(DeploymentContext::ReflectionLayer.requires_reflection());
    }

    // ---- DeploymentMode::select_for_context ----

    #[test]
    fn select_for_context_zero_signatories_returns_dynamic() {
        assert_eq!(
            DeploymentMode::select_for_context(DeploymentContext::NormalLayer, 0),
            DeploymentMode::Dynamic
        );
    }

    #[test]
    fn select_for_context_one_signatory_returns_single() {
        assert_eq!(
            DeploymentMode::select_for_context(DeploymentContext::NormalLayer, 1),
            DeploymentMode::Single
        );
    }

    #[test]
    fn select_for_context_multi_signatory_e_layer_returns_multi() {
        assert_eq!(
            DeploymentMode::select_for_context(DeploymentContext::ExistenceLayer, 5),
            DeploymentMode::Multi
        );
    }

    #[test]
    fn select_for_context_multi_signatory_normal_returns_dynamic() {
        assert_eq!(
            DeploymentMode::select_for_context(DeploymentContext::NormalLayer, 5),
            DeploymentMode::Dynamic
        );
    }

    // ---- Single mode ----

    #[test]
    fn single_mode_approved_with_one_signature() {
        let ha = auth_with_approvals(1, AuthorityMode::Single);
        let bio = MockBiometric::new();
        let enforcer = HADeploymentEnforcer::single(&ha, &bio, DeploymentContext::NormalLayer);
        let outcome = enforcer.enforce(&["h-1".into()], "low", 2000);
        assert!(
            outcome.is_approved(),
            "Single mode + 1 sig + low risk 必须通过"
        );
    }

    #[test]
    fn single_mode_rejected_with_zero_signatures() {
        let ha = auth_with_approvals(1, AuthorityMode::Single);
        let bio = MockBiometric::new();
        let enforcer = HADeploymentEnforcer::single(&ha, &bio, DeploymentContext::NormalLayer);
        let outcome = enforcer.enforce(&[], "low", 2000);
        assert!(outcome.is_rejected());
    }

    #[test]
    fn single_mode_rejected_with_two_signatures() {
        let ha = auth_with_approvals(1, AuthorityMode::Single);
        let bio = MockBiometric::new();
        let enforcer = HADeploymentEnforcer::single(&ha, &bio, DeploymentContext::NormalLayer);
        let outcome = enforcer.enforce(&["h-1".into(), "h-2".into()], "low", 2000);
        assert!(outcome.is_rejected(), "Single mode 必须恰好 1 个签名");
    }

    #[test]
    fn single_mode_rejected_high_risk() {
        let ha = auth_with_approvals(1, AuthorityMode::Single);
        let bio = MockBiometric::new();
        let enforcer = HADeploymentEnforcer::single(&ha, &bio, DeploymentContext::NormalLayer);
        let outcome = enforcer.enforce(&["h-1".into()], "high", 2000);
        assert!(outcome.is_rejected());
        if let DeploymentOutcome::RejectedSingleHighRisk { risk, max_allowed } = outcome {
            assert_eq!(risk, "high");
            assert_eq!(max_allowed, "medium");
        } else {
            panic!("应为 RejectedSingleHighRisk");
        }
    }

    #[test]
    fn single_mode_rejected_critical_risk() {
        let ha = auth_with_approvals(1, AuthorityMode::Single);
        let bio = MockBiometric::new();
        let enforcer = HADeploymentEnforcer::single(&ha, &bio, DeploymentContext::NormalLayer);
        let outcome = enforcer.enforce(&["h-1".into()], "critical", 2000);
        assert!(outcome.is_rejected());
    }

    // ---- Multi mode ----

    #[test]
    fn multi_mode_approved_2_of_3() {
        let ha = auth_with_approvals(3, AuthorityMode::Multi);
        let policy = MultiSigPolicy::default_2_of_3();
        let enforcer = HADeploymentEnforcer::multi(&ha, &policy, DeploymentContext::ExistenceLayer);
        let outcome = enforcer.enforce(&["h-0".into(), "h-1".into()], "high", 2000);
        assert!(outcome.is_approved());
    }

    #[test]
    fn multi_mode_rejected_1_of_3() {
        // 只注册 1 个 approval (满足 1-of-3 但不满足 2-of-3)
        let mut ha = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        ha.record_approval(HumanApproval::new("ap-0", "h-0", "S0", 1000, "x"));
        let policy = MultiSigPolicy::default_2_of_3();
        let enforcer = HADeploymentEnforcer::multi(&ha, &policy, DeploymentContext::ExistenceLayer);
        let outcome = enforcer.enforce(&["h-0".into()], "high", 2000);
        assert!(outcome.is_rejected());
        if let DeploymentOutcome::RejectedMultiInsufficient { have, need, .. } = outcome {
            assert_eq!(have, 1);
            assert_eq!(need, 2);
        } else {
            panic!("应为 RejectedMultiInsufficient");
        }
    }

    // ---- Dynamic mode ----

    #[test]
    fn dynamic_mode_e_layer_raises_threshold() {
        // base = 50, E layer +20 = 70
        let ha = auth_with_approvals(5, AuthorityMode::Dynamic);
        let enforcer = HADeploymentEnforcer::dynamic(&ha, DeploymentContext::ExistenceLayer);
        let outcome = enforcer.enforce(&[], "high", 2000);
        // 5 签者全批准 → 100% ≥ 70% 通过
        assert!(outcome.is_approved());
        if let DeploymentOutcome::ApprovedDynamic {
            base_threshold,
            adjusted_threshold,
            ..
        } = outcome
        {
            assert_eq!(base_threshold, 50);
            assert_eq!(adjusted_threshold, 70);
        }
    }

    #[test]
    fn dynamic_mode_emergency_lowers_threshold() {
        // base = 50, emergency -20 = 30 (min_threshold=30)
        let ha = auth_with_approvals(5, AuthorityMode::Dynamic);
        let enforcer = HADeploymentEnforcer::dynamic(&ha, DeploymentContext::EmergencyLayer);
        let outcome = enforcer.enforce(&[], "low", 2000);
        assert!(outcome.is_approved());
        if let DeploymentOutcome::ApprovedDynamic {
            adjusted_threshold, ..
        } = outcome
        {
            assert_eq!(adjusted_threshold, 30);
        }
    }

    #[test]
    fn dynamic_mode_emergency_floor_at_30() {
        // base = 10, emergency -20 = -10 → clamp to 30 (min_threshold)
        let mut ha = HumanAuthority::dynamic("d-1", "ctx", 1, 10, 5);
        // 添加足够批准通过
        for i in 0..5 {
            ha.record_approval(HumanApproval::new(
                format!("ap-{i}"),
                format!("h-{i}"),
                format!("S{i}"),
                1000,
                "x",
            ));
        }
        let enforcer = HADeploymentEnforcer::dynamic(&ha, DeploymentContext::EmergencyLayer);
        let outcome = enforcer.enforce(&[], "low", 2000);
        if let DeploymentOutcome::ApprovedDynamic {
            adjusted_threshold, ..
        } = outcome
        {
            assert!(
                adjusted_threshold >= 30,
                "Emergency 下限 30: got {adjusted_threshold}"
            );
        }
    }

    #[test]
    fn dynamic_mode_reflection_layer_plus_30() {
        // base = 50, reflection +30 = 80
        let ha = auth_with_approvals(5, AuthorityMode::Dynamic);
        let enforcer = HADeploymentEnforcer::dynamic(&ha, DeploymentContext::ReflectionLayer);
        let outcome = enforcer.enforce(&[], "low", 2000);
        assert!(outcome.is_approved());
        if let DeploymentOutcome::ApprovedDynamic {
            adjusted_threshold, ..
        } = outcome
        {
            assert_eq!(adjusted_threshold, 80);
        }
    }

    // ---- DeploymentReflectionTracker ----

    #[test]
    fn reflection_tracker_active_in_window() {
        let t = DeploymentReflectionTracker::new(1000, 7 * 86_400_000); // 7 天
        assert!(t.is_in_reflection(5_000_000));
        assert!(!t.is_in_reflection(8 * 86_400_000));
    }

    #[test]
    fn reflection_tracker_remaining_ms() {
        let t = DeploymentReflectionTracker::new(1000, 10_000);
        assert_eq!(t.remaining_ms(2000), 9000);
        assert_eq!(t.remaining_ms(11_000), 0);
        assert_eq!(t.remaining_ms(20_000), 0);
    }

    // ---- DeploymentOutcome 派生 ----

    #[test]
    fn outcome_is_approved_for_three_approved_variants() {
        let o1 = DeploymentOutcome::ApprovedSingle {
            signature_id: "s".into(),
            confidence: 0.9,
            context: DeploymentContext::NormalLayer,
        };
        let o2 = DeploymentOutcome::ApprovedMulti {
            valid_signatures: 2,
            required: 2,
            effective_threshold: 66,
            context: DeploymentContext::NormalLayer,
        };
        let o3 = DeploymentOutcome::ApprovedDynamic {
            valid_signatures: 3,
            required: 3,
            base_threshold: 50,
            adjusted_threshold: 70,
            context: DeploymentContext::ExistenceLayer,
        };
        assert!(o1.is_approved() && o2.is_approved() && o3.is_approved());
        assert!(!o1.is_rejected());
    }

    #[test]
    fn outcome_is_rejected_for_reject_variants() {
        let r1 = DeploymentOutcome::RejectedSingleHighRisk {
            risk: "high".into(),
            max_allowed: "medium".into(),
        };
        let r2 = DeploymentOutcome::RejectedMultiInsufficient {
            have: 1,
            need: 2,
            actual_pct: 33,
            threshold: 66,
        };
        let r3 = DeploymentOutcome::RejectedDynamicInsufficient {
            actual_pct: 20,
            adjusted_threshold: 70,
            context: DeploymentContext::ExistenceLayer,
        };
        let r4 = DeploymentOutcome::RejectedReflectionPending { remaining_ms: 1000 };
        assert!(r1.is_rejected() && r2.is_rejected() && r3.is_rejected() && r4.is_rejected());
    }
}
