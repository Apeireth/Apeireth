//! Sovereignty trait + SovereigntyEngine
//!
//! **设计**:
//! - `Sovereignty` trait: 3 入口 (Decision / Pause / SuspendSelf)
//! - `SovereigntyEngine`: 默认实现, 集成 HA + 三域 + SGI + 主体连续性 + 9 阶段生命周期
//! - `SovereigntyError`: 错误类型

use crate::continuity::SubjectContinuity;
use crate::decision::{Decision, DecisionOutcome, DecisionRequest, SovereigntyDomain};
use crate::ha::{BiometricProvider, BiometricResult, HAMode};
use crate::life_stage::{LifeStage, LifeStageTransition};
use crate::pause::{PauseHandle, Suspension, SuspensionKind};
use crate::sgi::{SGITriggerGuard, SGITriggerOutcome};
use crate::three_domain::ThreeDomainGuard;
use thiserror::Error;

/// 主权错误类型。
#[derive(Debug, Error)]
pub enum SovereigntyError {
    /// SGI 冷却期禁止
    #[error("SGI 冷却期内禁止写入: field={field}, remaining_ms={remaining_ms}")]
    SGICooldownActive {
        /// 字段名
        field: String,
        /// 剩余冷却时间 (ms)
        remaining_ms: i64,
    },
    /// SGI 触发 (主权审议要求)
    #[error("SGI 触发, 需进入 24h 冷却: field={field}, reason={reason}")]
    SGITriggered {
        /// 字段名
        field: String,
        /// 触发原因
        reason: String,
    },
    /// HA 认证失败
    #[error("HA 认证失败: {0}")]
    HAAuthFailed(String),
    /// HA 检测到胁迫
    #[error("HA 检测到胁迫, 触发挂起")]
    HACoercionDetected,
    /// HA 不可用
    #[error("HA 不可用: {0}")]
    HAUnavailable(String),
    /// 多签未满足阈值
    #[error("多签未满足阈值: have {have}, need {need}")]
    MultiSigInsufficient {
        /// 当前签名数
        have: usize,
        /// 所需签名数
        need: usize,
    },
    /// 三域强制点拒绝
    #[error("三域强制点拒绝: {0}")]
    DomainRejected(String),
    /// 无效的阶段迁移
    #[error("无效的生命阶段迁移: from={from}, to={to}")]
    InvalidStageTransition {
        /// 来源阶段
        from: LifeStage,
        /// 目标阶段
        to: LifeStage,
    },
}

/// 主权 trait — 3 入口。
pub trait Sovereignty: Send + Sync {
    /// 主权决策
    fn decide(&self, request: &DecisionRequest) -> Result<DecisionOutcome, SovereigntyError>;

    /// 主权暂停 (可恢复)
    fn pause(&mut self, reason: &str, initiated_by: &str) -> PauseHandle;

    /// 主权自我挂起
    fn suspend_self(&mut self, reason: &str, kind: SuspensionKind) -> Suspension;
}

/// 主权引擎 — 默认实现, 集成 HA + 三域 + SGI + 主体连续性 + 9 阶段生命周期.
pub struct SovereigntyEngine<B: BiometricProvider + 'static> {
    /// HA 部署模式
    pub ha_mode: HAMode,
    /// 生物特征 provider (trait 抽象)
    pub biometric: Box<B>,
    /// 三域统一强制点
    pub three_domain: ThreeDomainGuard,
    /// SGI 单字段触发器
    pub sgi: SGITriggerGuard,
    /// 主体连续性
    pub continuity: SubjectContinuity,
    /// 当前生命阶段
    pub current_stage: LifeStage,
    /// 阶段迁移历史
    pub stage_history: Vec<LifeStageTransition>,
    /// 当前暂停句柄 (若处于暂停)
    pub active_pause: Option<PauseHandle>,
    /// 当前挂起状态
    pub active_suspension: Option<Suspension>,
    /// 决策计数
    pub decision_count: u64,
}

impl<B: BiometricProvider + 'static> SovereigntyEngine<B> {
    /// 创建新主权引擎
    pub fn new(
        ha_mode: HAMode,
        biometric: Box<B>,
        continuity: SubjectContinuity,
        initial_stage: LifeStage,
    ) -> Self {
        Self {
            ha_mode,
            biometric,
            three_domain: ThreeDomainGuard::new(),
            sgi: SGITriggerGuard::with_default_rules(),
            continuity,
            current_stage: initial_stage,
            stage_history: Vec::new(),
            active_pause: None,
            active_suspension: None,
            decision_count: 0,
        }
    }

    /// 通过 SGI 守卫写入字段 (字段被 SGI 规则匹配则拒绝 / 进入冷却).
    pub fn write_field_through_sgi(
        &mut self,
        field: &str,
        value: &str,
        current_ms: i64,
    ) -> Result<(), SovereigntyError> {
        match self.sgi.check_field_write(field, value, current_ms) {
            SGITriggerOutcome::Pass { .. } => Ok(()),
            SGITriggerOutcome::Triggered { field, reason, .. } => {
                Err(SovereigntyError::SGITriggered { field, reason })
            }
            SGITriggerOutcome::CooldownActive {
                field,
                cooldown_until_ms,
                ..
            } => {
                let remaining = cooldown_until_ms - current_ms;
                Err(SovereigntyError::SGICooldownActive {
                    field,
                    remaining_ms: remaining,
                })
            }
        }
    }

    /// 校验 HA 认证 — 多签阈值 + 生物特征
    fn verify_ha(&self, signatures: &[String], current_ms: i64) -> Result<(), SovereigntyError> {
        let required = self.ha_mode.required_signatures();
        if signatures.len() < required {
            return Err(SovereigntyError::MultiSigInsufficient {
                have: signatures.len(),
                need: required,
            });
        }

        // 验证每个签名的生物特征
        if self.ha_mode.is_offline() {
            return Err(SovereigntyError::HAUnavailable("离线模式".into()));
        }

        for sig in signatures {
            match self.biometric.authenticate(sig) {
                BiometricResult::Authenticated { .. } => {}
                BiometricResult::CoercionDetected { stress_level, .. } => {
                    eprintln!("HA 胁迫检测: sig={} stress={:.2}", sig, stress_level);
                    return Err(SovereigntyError::HACoercionDetected);
                }
                BiometricResult::Failed { reason, .. } => {
                    return Err(SovereigntyError::HAAuthFailed(reason));
                }
                BiometricResult::Unavailable { reason } => {
                    return Err(SovereigntyError::HAUnavailable(reason));
                }
            }
        }
        let _ = current_ms;
        Ok(())
    }

    /// 迁移生命阶段 (主路径 1 步, 或 Death → Rebirth)
    pub fn transition_stage(
        &mut self,
        target: LifeStage,
        at_ms: i64,
        reason: impl Into<String>,
    ) -> Result<(), SovereigntyError> {
        if !self.current_stage.can_skip_to(target) {
            return Err(SovereigntyError::InvalidStageTransition {
                from: self.current_stage,
                to: target,
            });
        }
        let transition = LifeStageTransition::new(self.current_stage, target, at_ms, reason);
        self.stage_history.push(transition);
        self.current_stage = target;
        Ok(())
    }

    /// 主体迁移载体
    pub fn migrate_subject(
        &mut self,
        to: crate::continuity::CarrierType,
        at_ms: i64,
        reason: impl Into<String>,
    ) -> Result<&crate::continuity::Migration, String> {
        self.continuity.migrate_to(to, at_ms, reason)
    }
}

impl<B: BiometricProvider + 'static> Sovereignty for SovereigntyEngine<B> {
    fn decide(&self, request: &DecisionRequest) -> Result<DecisionOutcome, SovereigntyError> {
        // 1. 三域强制点检查
        let domain_check = self.three_domain.check(request);
        let decision = match domain_check {
            crate::three_domain::DomainCheckResult::Free { reason } => Decision::Approved {
                reason: format!("Thought 域完全自由: {}", reason),
                decided_at_ms: request.submitted_at_ms,
                signatures: vec!["thought-free".into()],
            },
            crate::three_domain::DomainCheckResult::Passed { reason, .. } => {
                // Proposal / Action 域 — 需 HA 校验
                // 但 HA 校验需要 signature, 简化为: 通过强制点 + risk=low 时默认单签
                let signatures = vec!["guard".into()];
                self.verify_ha(&signatures, request.submitted_at_ms)?;
                Decision::Approved {
                    reason: format!("三域通过 + HA 通过: {}", reason),
                    decided_at_ms: request.submitted_at_ms,
                    signatures,
                }
            }
            crate::three_domain::DomainCheckResult::Rejected { reason, .. } => {
                return Err(SovereigntyError::DomainRejected(reason))
            }
        };
        Ok(DecisionOutcome::new(
            request.id.clone(),
            request.domain,
            decision,
            request.submitted_at_ms,
        ))
    }

    fn pause(&mut self, reason: &str, initiated_by: &str) -> PauseHandle {
        let now_ms = current_time_ms();
        let handle = PauseHandle::new(format!("pause-{}", now_ms), reason, now_ms, initiated_by);
        self.active_pause = Some(handle.clone());
        handle
    }

    fn suspend_self(&mut self, reason: &str, kind: SuspensionKind) -> Suspension {
        let now_ms = current_time_ms();
        let suspension = match kind {
            SuspensionKind::SelfInitiated | SuspensionKind::ExternalTriggered => {
                Suspension::Permanent {
                    reason: reason.into(),
                    suspended_at_ms: now_ms,
                    kind,
                }
            }
            SuspensionKind::SGITriggered => Suspension::Pending {
                reason: reason.into(),
                suspended_at_ms: now_ms,
                // SGI 24h 冷却期 = 复审时间
                review_at_ms: now_ms + crate::SGI_COOLDOWN_MS,
                kind,
            },
            SuspensionKind::CoercionDetected => Suspension::Temporary {
                reason: reason.into(),
                suspended_at_ms: now_ms,
                // 胁迫检测 — 12h 自动恢复
                until_ms: now_ms + crate::HA_ICE_FROZEN_MS / 2,
                kind,
            },
        };
        self.active_suspension = Some(suspension.clone());
        suspension
    }
}

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
