//! OTA 状态机 — 完整 7 阶段 (round10-01 升级: Download → Sandbox).
//!
//! 7 阶段 (完整升级管线):
//! 1. `IntentDraft` — 编写 UpgradeIntent (Intent 状态机)
//! 2. `CouncilReview` — 7 席智囊团审议 (含按住机制)
//! 3. `MultiSig` — 物理多签收集 (m-of-n)
//! 4. `Sandbox` — 沙盒隔离验证 (使用 SandboxValidator trait, 物理隔离守门 3)
//! 5. `Switchover` — 蓝绿切换
//! 6. `Monitor` — 监控期 (dashboard + smoke)
//! 7. `Done` (终态) / `Rollback` (终态)
//!
//! 初始状态: `Idle` (尚未启动).
//!
//! 合法转换:
//! Idle        -> IntentDraft
//! IntentDraft -> CouncilReview
//! CouncilReview -> MultiSig
//! MultiSig    -> Sandbox
//! Sandbox     -> Switchover
//! Switchover  -> Monitor
//! Monitor     -> Done | Rollback
//!
//! 异常转换 (任意阶段都可触发 Rollback):
//! 任意阶段 -> Rollback (经由 Hold / MultiSig Timeout / Sandbox Reject / Monitor Failed)
//!
//! **反向状态机 (rollback reverse path)**: Rollback 后回溯经过的阶段顺序:
//! Monitor -> Switchover -> Sandbox -> MultiSig -> CouncilReview -> IntentDraft -> Idle
//!
//! **历史保留**: 旧版 OtaStage 3 状态 (Idle / Downloading / Applying) 已在 A15 落地;
//! round6-03 升级为 7 阶段 (Download), round10-01 升级为 7 阶段 (Sandbox, 物理隔离守门 3).

use super::council::CouncilReport;
use super::intent::UpgradeIntent;
use super::manifest::UpgradeManifest;
use super::monitor::MonitorReport;
use super::multisig::MultiSigOutcome;
use super::sandbox::{SandboxValidator, SandboxVerdict};
use super::UpgradeError;

/// 7 阶段 OTA 阶段枚举.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OtaStage {
    /// 空闲, 等待升级触发.
    Idle,
    /// 1/7 编写 UpgradeIntent.
    IntentDraft,
    /// 2/7 智囊团审议.
    CouncilReview,
    /// 3/7 物理多签收集.
    MultiSig,
    /// 4/7 沙盒隔离验证 (round10-01 升级 — 物理隔离守门 3).
    Sandbox,
    /// 5/7 蓝绿切换.
    Switchover,
    /// 6/7 监控期.
    Monitor,
    /// 7/7 终态 — 成功 (Done).
    Done,
    /// 7/7 终态 — 回滚 (Rollback).
    Rollback,
}

impl OtaStage {
    /// 是否为终态 (Done / Rollback).
    pub fn is_terminal(self) -> bool {
        matches!(self, OtaStage::Done | OtaStage::Rollback)
    }

    /// 是否为成功终态.
    pub fn is_success(self) -> bool {
        matches!(self, OtaStage::Done)
    }

    /// 是否为失败终态.
    pub fn is_rollback(self) -> bool {
        matches!(self, OtaStage::Rollback)
    }

    /// 是否为 7 阶段中的一个 (非 Idle, 非终态).
    pub fn is_active(self) -> bool {
        !matches!(self, OtaStage::Idle | OtaStage::Done | OtaStage::Rollback)
    }

    /// 阶段序号 (1-7, 用于 "完整 7 阶段" 验证).
    pub fn phase_number(self) -> usize {
        match self {
            OtaStage::Idle => 0,
            OtaStage::IntentDraft => 1,
            OtaStage::CouncilReview => 2,
            OtaStage::MultiSig => 3,
            OtaStage::Sandbox => 4,
            OtaStage::Switchover => 5,
            OtaStage::Monitor => 6,
            OtaStage::Done => 7,
            OtaStage::Rollback => 7,
        }
    }

    /// 全部 7 阶段 (不含 Idle / 终态 Rollback).
    pub const SEVEN_STAGES: [OtaStage; 7] = [
        OtaStage::IntentDraft,
        OtaStage::CouncilReview,
        OtaStage::MultiSig,
        OtaStage::Sandbox,
        OtaStage::Switchover,
        OtaStage::Monitor,
        OtaStage::Done,
    ];

    /// 反向回溯顺序 — 用于 Rollback 后阶段逆序
    /// (从 Done/Rollback 反推到 Idle 的阶段顺序, 供 rollback_reverse_path 使用).
    pub const REVERSE_STAGES: [OtaStage; 7] = [
        OtaStage::Monitor,
        OtaStage::Switchover,
        OtaStage::Sandbox,
        OtaStage::MultiSig,
        OtaStage::CouncilReview,
        OtaStage::IntentDraft,
        OtaStage::Idle,
    ];
}

/// OTA 完整状态 — 阶段 + 关联数据 (审计可追溯).
#[derive(Debug, Clone)]
pub enum OtaState {
    /// 空闲.
    Idle,
    /// 1/7 Intent 草拟中.
    IntentDrafting(UpgradeIntent),
    /// 2/7 智囊团审议完成.
    CouncilReviewed(CouncilReport),
    /// 3/7 多签收集结果.
    MultiSigCollected(MultiSigOutcome),
    /// 4/7 沙盒验证完成 (round10-01: 取代 round6-03 的 Download 阶段).
    Sandboxed {
        /// 关联 intent ID.
        intent_id: uuid::Uuid,
        /// 蓝绿切换前 carrier.
        blue_carrier: String,
        /// 蓝绿切换后 carrier.
        green_carrier: String,
        /// 沙盒验证结果 (Accept / Reject).
        verdict: SandboxVerdict,
    },
    /// 5/7 蓝绿切换完成.
    SwitchedOver {
        /// 关联 intent ID.
        intent_id: uuid::Uuid,
        /// 蓝绿切换前 carrier.
        blue_carrier: String,
        /// 蓝绿切换后 carrier.
        green_carrier: String,
    },
    /// 6/7 监控期.
    Monitoring(MonitorReport),
    /// 7/7 终态 — 成功.
    Done(MonitorReport),
    /// 7/7 终态 — 回滚.
    Rollback {
        /// 回滚原因.
        reason: String,
        /// 在哪个阶段触发回滚.
        from_stage: OtaStage,
    },
}

impl OtaState {
    /// 是否为终态 (Done / Rollback).
    pub fn is_terminal(&self) -> bool {
        matches!(self, OtaState::Done(_) | OtaState::Rollback { .. })
    }

    /// 是否为成功终态.
    pub fn is_success(&self) -> bool {
        matches!(self, OtaState::Done(_))
    }

    /// 是否为失败终态.
    pub fn is_rollback(&self) -> bool {
        matches!(self, OtaState::Rollback { .. })
    }

    /// 当前阶段.
    pub fn stage(&self) -> OtaStage {
        match self {
            OtaState::Idle => OtaStage::Idle,
            OtaState::IntentDrafting(_) => OtaStage::IntentDraft,
            OtaState::CouncilReviewed(_) => OtaStage::CouncilReview,
            OtaState::MultiSigCollected(_) => OtaStage::MultiSig,
            OtaState::Sandboxed { .. } => OtaStage::Sandbox,
            OtaState::SwitchedOver { .. } => OtaStage::Switchover,
            OtaState::Monitoring(_) => OtaStage::Monitor,
            OtaState::Done(_) => OtaStage::Done,
            OtaState::Rollback { .. } => OtaStage::Rollback,
        }
    }

    /// **反向状态机** (round10-01 升级): Rollback 后回溯经过的阶段顺序.
    /// 从触发回滚的 `from_stage` 逆序到 Idle, 用于审计可追溯.
    /// - Rollback 终态: 返回 `[from_stage, ..., Idle]`
    /// - 非 Rollback 状态: 返回空数组
    /// - 终态 Done: 返回空数组
    pub fn rollback_reverse_path(&self) -> Vec<OtaStage> {
        match self {
            OtaState::Rollback { from_stage, .. } => {
                let mut path = Vec::new();
                let mut started = false;
                for stage in OtaStage::REVERSE_STAGES.iter() {
                    if *stage == *from_stage || started {
                        started = true;
                        path.push(*stage);
                    }
                }
                // 如果 from_stage 不在 REVERSE_STAGES, 仍返回整个 REVERSE_STAGES.
                if path.is_empty() {
                    path = OtaStage::REVERSE_STAGES.to_vec();
                }
                path
            }
            _ => Vec::new(),
        }
    }
}

/// OTA 升级管道 — 维护当前阶段 + 状态对象.
pub struct OtaPipeline {
    state: OtaState,
}

impl OtaPipeline {
    /// 构造初始管道 (Idle 状态).
    pub fn new(initial: OtaStage) -> Self {
        debug_assert!(
            matches!(initial, OtaStage::Idle),
            "OtaPipeline::new 必须以 Idle 启动"
        );
        Self {
            state: OtaState::Idle,
        }
    }

    /// 取当前阶段.
    pub fn stage(&self) -> OtaStage {
        self.state.stage()
    }

    /// 取当前状态.
    pub fn state(&self) -> &OtaState {
        &self.state
    }

    /// 启动 Intent 草拟 (Idle -> IntentDraft).
    pub fn start_intent(&mut self, intent: UpgradeIntent) -> Result<(), UpgradeError> {
        if self.stage() != OtaStage::Idle {
            return Err(UpgradeError::IllegalTransition(
                self.stage(),
                OtaStage::IntentDraft,
            ));
        }
        self.state = OtaState::IntentDrafting(intent);
        Ok(())
    }

    /// 进入 CouncilReview (IntentDraft -> CouncilReview).
    /// 按住机制: council 报告含 TriggerHold 则直接进入 Rollback.
    pub fn enter_council_review(&mut self, report: CouncilReport) -> Result<(), UpgradeError> {
        if self.stage() != OtaStage::IntentDraft {
            return Err(UpgradeError::IllegalTransition(
                self.stage(),
                OtaStage::CouncilReview,
            ));
        }
        if matches!(report.hold, super::council::HoldAction::TriggerHold { .. }) {
            self.state = OtaState::Rollback {
                reason: "council hold triggered".into(),
                from_stage: OtaStage::CouncilReview,
            };
            return Ok(());
        }
        self.state = OtaState::CouncilReviewed(report);
        Ok(())
    }

    /// 进入 MultiSig (CouncilReview -> MultiSig).
    /// 若多签 collect 结果为 Pending/Timeout/Invalid, 触发 Rollback.
    pub fn enter_multisig(&mut self, outcome: MultiSigOutcome) -> Result<(), UpgradeError> {
        if self.stage() != OtaStage::CouncilReview {
            return Err(UpgradeError::IllegalTransition(
                self.stage(),
                OtaStage::MultiSig,
            ));
        }
        if outcome.is_blocking() {
            self.state = OtaState::Rollback {
                reason: format!("multisig blocked: {outcome:?}"),
                from_stage: OtaStage::MultiSig,
            };
            return Ok(());
        }
        self.state = OtaState::MultiSigCollected(outcome);
        Ok(())
    }

    /// 进入 Sandbox (MultiSig -> Sandbox) — round10-01 升级, 物理隔离守门 3.
    /// 使用 sandbox validator 校验 manifest; 若 Reject 则触发 Rollback.
    /// `blue` / `green` 是蓝绿切换前后的 carrier 名称 (供后续 Switchover 使用).
    pub fn enter_sandbox<V: SandboxValidator>(
        &mut self,
        intent_id: uuid::Uuid,
        blue: String,
        green: String,
        manifest: &UpgradeManifest,
        sandbox: &V,
    ) -> Result<(), UpgradeError> {
        if self.stage() != OtaStage::MultiSig {
            return Err(UpgradeError::IllegalTransition(
                self.stage(),
                OtaStage::Sandbox,
            ));
        }
        let verdict = sandbox.validate(manifest);
        if matches!(verdict, SandboxVerdict::Reject(_)) {
            self.state = OtaState::Rollback {
                reason: format!("sandbox rejected: {:?}", verdict),
                from_stage: OtaStage::Sandbox,
            };
            return Ok(());
        }
        self.state = OtaState::Sandboxed {
            intent_id,
            blue_carrier: blue,
            green_carrier: green,
            verdict,
        };
        Ok(())
    }

    /// 进入 Switchover (Sandbox -> Switchover).
    pub fn enter_switchover(&mut self) -> Result<(), UpgradeError> {
        if self.stage() != OtaStage::Sandbox {
            return Err(UpgradeError::IllegalTransition(
                self.stage(),
                OtaStage::Switchover,
            ));
        }
        let (intent_id, blue, green) = match &self.state {
            OtaState::Sandboxed {
                intent_id,
                blue_carrier,
                green_carrier,
                ..
            } => (*intent_id, blue_carrier.clone(), green_carrier.clone()),
            _ => {
                return Err(UpgradeError::IllegalTransition(
                    self.stage(),
                    OtaStage::Switchover,
                ))
            }
        };
        self.state = OtaState::SwitchedOver {
            intent_id,
            blue_carrier: blue,
            green_carrier: green,
        };
        Ok(())
    }

    /// 进入 Monitor (Switchover -> Monitor).
    pub fn enter_monitor(&mut self, report: MonitorReport) -> Result<(), UpgradeError> {
        if self.stage() != OtaStage::Switchover {
            return Err(UpgradeError::IllegalTransition(
                self.stage(),
                OtaStage::Monitor,
            ));
        }
        self.state = OtaState::Monitoring(report);
        Ok(())
    }

    /// 收尾 — Done 或 Rollback (Monitor -> Done/Rollback).
    /// 返回最终阶段 (Done / Rollback).
    pub fn finalize(&mut self, report: MonitorReport) -> Result<OtaStage, UpgradeError> {
        if self.stage() != OtaStage::Monitor {
            return Err(UpgradeError::IllegalTransition(
                self.stage(),
                OtaStage::Done,
            ));
        }
        if report.should_rollback() {
            self.state = OtaState::Rollback {
                reason: "monitor recommended rollback".into(),
                from_stage: OtaStage::Monitor,
            };
            Ok(OtaStage::Rollback)
        } else {
            self.state = OtaState::Done(report);
            Ok(OtaStage::Done)
        }
    }

    /// 强制 Rollback (任意阶段 -> Rollback).
    pub fn rollback(&mut self, reason: impl Into<String>) -> Result<(), UpgradeError> {
        if self.stage().is_terminal() {
            return Err(UpgradeError::IllegalTransition(
                self.stage(),
                OtaStage::Rollback,
            ));
        }
        let from = self.stage();
        self.state = OtaState::Rollback {
            reason: reason.into(),
            from_stage: from,
        };
        Ok(())
    }

    /// 通用转换 (向后兼容 — 用于已落地的 7 阶段自循环检查).
    /// 7 阶段流程转换必须使用专门的 enter_* 方法; 此方法仅允许同 self -> self 的幂等转换.
    pub fn transition(&mut self, next: OtaStage) -> Result<(), UpgradeError> {
        let from = self.stage();
        let legal = matches!(
            (from, next),
            (OtaStage::Idle, OtaStage::Idle)
                | (OtaStage::IntentDraft, OtaStage::IntentDraft)
                | (OtaStage::CouncilReview, OtaStage::CouncilReview)
                | (OtaStage::MultiSig, OtaStage::MultiSig)
                | (OtaStage::Sandbox, OtaStage::Sandbox)
                | (OtaStage::Switchover, OtaStage::Switchover)
                | (OtaStage::Monitor, OtaStage::Monitor)
                | (OtaStage::Done, OtaStage::Done)
                | (OtaStage::Rollback, OtaStage::Rollback)
        );
        if !legal {
            return Err(UpgradeError::IllegalTransition(from, next));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::council::{CouncilOpinion, CouncilReviewer, CouncilSeat, CouncilStance, HoldAction};
    use crate::intent::{IntentStateMachine, UpgradeIntent};
    use crate::manifest::{ManifestBuilder, UpgradeKind};
    use crate::monitor::{MonitorDashboard, MonitorMetric};
    use crate::multisig::{
        intent_payload_hash, MultiSigCollector, MultiSigConfig, PhysicalSignature,
    };
    use crate::sandbox::{DefaultSandbox, SandboxValidator, SandboxVerdict};
    use uuid::Uuid;

    fn sample_intent() -> UpgradeIntent {
        UpgradeIntent::new(
            Uuid::new_v4(),
            "v1.1.0",
            "v1.0.0",
            UpgradeKind::Patch,
            "carrier-a",
            "fix",
        )
    }

    /// round10-01 测试用 sample manifest (Sandbox 默认接受).
    fn sample_manifest() -> UpgradeManifest {
        ManifestBuilder::new("v1.1.0", UpgradeKind::Patch)
            .with_description("round10-01 sample")
            .with_content_hash("hash")
            .build()
    }

    fn all_approve_opinions() -> Vec<CouncilOpinion> {
        CouncilSeat::ALL
            .iter()
            .map(|s| CouncilOpinion::new(*s, CouncilStance::Approve, 0.9, "ok"))
            .collect()
    }

    fn healthy_monitor_report() -> MonitorReport {
        let mut d = MonitorDashboard::new();
        d.record(MonitorMetric::new("a", 0.01, Some(0.05), None));
        d.record(MonitorMetric::new("b", 100.0, Some(500.0), None));
        d.report()
    }

    /// 公共 helper: 跑完 IntentDraft -> Sandbox 流程 (默认接受 sandbox).
    fn drive_to_sandbox(p: &mut OtaPipeline, intent: &UpgradeIntent) {
        let council = CouncilReviewer::new().review(intent, all_approve_opinions());
        p.enter_council_review(council).unwrap();
        let hash = intent_payload_hash(intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut col = MultiSigCollector::new(cfg, hash.clone());
        for i in 0..5 {
            col.submit(PhysicalSignature::new(
                format!("signer-{i}"),
                hash.clone(),
                100 + i64::from(i),
                format!("sig{i}"),
            ))
            .unwrap();
        }
        p.enter_multisig(col.evaluate(200)).unwrap();
        let sandbox = DefaultSandbox;
        p.enter_sandbox(
            intent.id,
            "blue".into(),
            "green".into(),
            &sample_manifest(),
            &sandbox,
        )
        .unwrap();
    }

    #[test]
    fn seven_stages_constant_has_seven() {
        assert_eq!(OtaStage::SEVEN_STAGES.len(), 7);
    }

    #[test]
    fn stage_phase_number_and_active() {
        assert_eq!(OtaStage::Idle.phase_number(), 0);
        assert_eq!(OtaStage::IntentDraft.phase_number(), 1);
        assert_eq!(OtaStage::CouncilReview.phase_number(), 2);
        assert_eq!(OtaStage::MultiSig.phase_number(), 3);
        assert_eq!(OtaStage::Sandbox.phase_number(), 4);
        assert_eq!(OtaStage::Switchover.phase_number(), 5);
        assert_eq!(OtaStage::Monitor.phase_number(), 6);
        assert_eq!(OtaStage::Done.phase_number(), 7);
        assert_eq!(OtaStage::Rollback.phase_number(), 7);
        assert!(!OtaStage::Idle.is_active());
        assert!(OtaStage::IntentDraft.is_active());
        assert!(OtaStage::Monitor.is_active());
        assert!(!OtaStage::Done.is_active());
        assert!(OtaStage::Done.is_terminal());
        assert!(OtaStage::Rollback.is_terminal());
    }

    #[test]
    fn pipeline_starts_idle() {
        let p = OtaPipeline::new(OtaStage::Idle);
        assert_eq!(p.stage(), OtaStage::Idle);
        assert!(!p.state().is_terminal());
    }

    #[test]
    fn pipeline_full_happy_path() {
        let intent = sample_intent();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        assert_eq!(p.stage(), OtaStage::IntentDraft);

        drive_to_sandbox(&mut p, &intent);
        assert_eq!(p.stage(), OtaStage::Sandbox);

        p.enter_switchover().unwrap();
        assert_eq!(p.stage(), OtaStage::Switchover);

        let report = healthy_monitor_report();
        p.enter_monitor(report).unwrap();
        assert_eq!(p.stage(), OtaStage::Monitor);

        let report2 = healthy_monitor_report();
        let term = p.finalize(report2).unwrap();
        assert_eq!(term, OtaStage::Done);
        assert!(p.state().is_success());
    }

    #[test]
    fn pipeline_council_hold_triggers_rollback() {
        let intent = sample_intent();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();

        let mut ops = all_approve_opinions();
        ops[0] = CouncilOpinion::new(
            CouncilSeat::Principle,
            CouncilStance::StrongDisapprove,
            0.95,
            "x",
        );
        let report = CouncilReviewer::new().review(&intent, ops);
        assert!(matches!(report.hold, HoldAction::TriggerHold { .. }));
        p.enter_council_review(report).unwrap();
        assert_eq!(p.stage(), OtaStage::Rollback);
        assert!(p.state().is_rollback());
    }

    #[test]
    fn pipeline_multisig_timeout_triggers_rollback() {
        let intent = sample_intent();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        let council = CouncilReviewer::new().review(&intent, all_approve_opinions());
        p.enter_council_review(council).unwrap();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::new(2, vec!["a".into(), "b".into()]).with_deadline(100);
        let col = MultiSigCollector::new(cfg, hash.clone());
        let outcome = col.evaluate(500);
        p.enter_multisig(outcome).unwrap();
        assert_eq!(p.stage(), OtaStage::Rollback);
    }

    #[test]
    fn pipeline_monitor_failed_triggers_rollback() {
        let intent = sample_intent();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        drive_to_sandbox(&mut p, &intent);
        p.enter_switchover().unwrap();
        let _ = p.enter_monitor(healthy_monitor_report());

        let mut d = MonitorDashboard::new();
        d.record(MonitorMetric::new("err", 0.99, Some(0.05), None));
        let bad = d.report();
        let term = p.finalize(bad).unwrap();
        assert_eq!(term, OtaStage::Rollback);
    }

    #[test]
    fn pipeline_explicit_rollback_at_any_stage() {
        let intent = sample_intent();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        p.rollback("manual abort").unwrap();
        assert_eq!(p.stage(), OtaStage::Rollback);
    }

    #[test]
    fn pipeline_illegal_intent_restart() {
        let intent = sample_intent();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        let err = p.start_intent(intent).unwrap_err();
        match err {
            UpgradeError::IllegalTransition(from, to) => {
                assert_eq!(from, OtaStage::IntentDraft);
                assert_eq!(to, OtaStage::IntentDraft);
            }
            _ => panic!("expected IllegalTransition"),
        }
    }

    #[test]
    fn pipeline_terminal_blocks_rollback() {
        let intent = sample_intent();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        p.rollback("x").unwrap();
        let err = p.rollback("y").unwrap_err();
        assert!(matches!(
            err,
            UpgradeError::IllegalTransition(OtaStage::Rollback, OtaStage::Rollback)
        ));
    }

    #[test]
    fn ota_state_is_terminal_for_done_and_rollback() {
        let report = healthy_monitor_report();
        assert!(OtaState::Done(report.clone()).is_terminal());
        assert!(OtaState::Done(report).is_success());
        let rb = OtaState::Rollback {
            reason: "x".into(),
            from_stage: OtaStage::Monitor,
        };
        assert!(rb.is_terminal());
        assert!(rb.is_rollback());
        let intent = sample_intent();
        let mut sm = IntentStateMachine::wrap(intent);
        let _ = sm.submit();
        assert!(!OtaState::IntentDrafting(sm.intent().clone()).is_terminal());
    }

    #[test]
    fn intent_state_machine_full_flow() {
        let intent = sample_intent();
        let mut sm = IntentStateMachine::wrap(intent);
        assert!(sm.submit().is_ok());
        assert!(sm.approve().is_ok());
        assert_eq!(sm.status(), crate::intent::IntentStatus::Approved);
    }

    #[test]
    fn manifest_through_manifest_builder() {
        let m = ManifestBuilder::new("v1.0.0", UpgradeKind::Patch)
            .with_content_hash("h")
            .build();
        assert_eq!(m.version, "v1.0.0");
    }

    #[test]
    fn transition_self_loop_is_idempotent() {
        let intent = sample_intent();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent).unwrap();
        // Same -> Same 幂等
        assert!(p.transition(OtaStage::IntentDraft).is_ok());
    }

    #[test]
    fn transition_skip_is_illegal() {
        let intent = sample_intent();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent).unwrap();
        // 跳到 Sandbox 不允许
        let err = p.transition(OtaStage::Sandbox).unwrap_err();
        assert!(matches!(err, UpgradeError::IllegalTransition(_, _)));
    }

    #[test]
    fn state_stage_returns_correct_for_done() {
        let report = healthy_monitor_report();
        let s = OtaState::Done(report);
        assert_eq!(s.stage(), OtaStage::Done);
        assert!(s.is_terminal());
        assert!(s.is_success());
    }

    #[test]
    fn state_stage_returns_correct_for_rollback() {
        let s = OtaState::Rollback {
            reason: "x".into(),
            from_stage: OtaStage::Monitor,
        };
        assert_eq!(s.stage(), OtaStage::Rollback);
        assert!(s.is_terminal());
        assert!(s.is_rollback());
    }

    // ========================= round10-01 新增单元测试 =========================
    // 覆盖 Sandbox 阶段 + 反向状态机 + 拒绝路径 + 边界

    #[test]
    fn r10_sandbox_replaces_download_in_seven_stages() {
        // 验证 SEVEN_STAGES 数组现在是 7 项, 第 4 项是 Sandbox.
        let s = OtaStage::SEVEN_STAGES;
        assert_eq!(s.len(), 7);
        assert_eq!(s[3], OtaStage::Sandbox);
        // SEVEN_STAGES 不应含 Idle (起态) 或 Rollback (替代终态).
        assert!(!s.contains(&OtaStage::Idle));
        assert!(!s.contains(&OtaStage::Rollback));
    }

    #[test]
    fn r10_reverse_stages_constant_order() {
        // 反向回溯顺序: Monitor -> Switchover -> Sandbox -> MultiSig -> CouncilReview -> IntentDraft -> Idle.
        let r = OtaStage::REVERSE_STAGES;
        assert_eq!(r.len(), 7);
        assert_eq!(r[0], OtaStage::Monitor);
        assert_eq!(r[1], OtaStage::Switchover);
        assert_eq!(r[2], OtaStage::Sandbox);
        assert_eq!(r[3], OtaStage::MultiSig);
        assert_eq!(r[4], OtaStage::CouncilReview);
        assert_eq!(r[5], OtaStage::IntentDraft);
        assert_eq!(r[6], OtaStage::Idle);
    }

    #[test]
    fn r10_rollback_reverse_path_from_monitor() {
        // Monitor 阶段触发回滚 → 反向路径应是 [Monitor, Switchover, Sandbox, MultiSig, CouncilReview, IntentDraft, Idle].
        let s = OtaState::Rollback {
            reason: "monitor failed".into(),
            from_stage: OtaStage::Monitor,
        };
        let path = s.rollback_reverse_path();
        assert_eq!(
            path,
            vec![
                OtaStage::Monitor,
                OtaStage::Switchover,
                OtaStage::Sandbox,
                OtaStage::MultiSig,
                OtaStage::CouncilReview,
                OtaStage::IntentDraft,
                OtaStage::Idle,
            ]
        );
    }

    #[test]
    fn r10_rollback_reverse_path_from_sandbox() {
        // Sandbox 阶段触发回滚 → 反向路径应包含 Sandbox 之后的所有阶段.
        let s = OtaState::Rollback {
            reason: "sandbox reject".into(),
            from_stage: OtaStage::Sandbox,
        };
        let path = s.rollback_reverse_path();
        assert_eq!(
            path,
            vec![
                OtaStage::Sandbox,
                OtaStage::MultiSig,
                OtaStage::CouncilReview,
                OtaStage::IntentDraft,
                OtaStage::Idle,
            ]
        );
    }

    #[test]
    fn r10_rollback_reverse_path_from_intent_draft() {
        // IntentDraft 触发回滚 → 反向路径只有 [IntentDraft, Idle].
        let s = OtaState::Rollback {
            reason: "intent rejected".into(),
            from_stage: OtaStage::IntentDraft,
        };
        let path = s.rollback_reverse_path();
        assert_eq!(path, vec![OtaStage::IntentDraft, OtaStage::Idle]);
    }

    #[test]
    fn r10_rollback_reverse_path_non_rollback_state_empty() {
        // 非 Rollback 状态调用 rollback_reverse_path 应返回空数组.
        let s = OtaState::Idle;
        assert!(s.rollback_reverse_path().is_empty());
        let intent = sample_intent();
        let s = OtaState::IntentDrafting(intent);
        assert!(s.rollback_reverse_path().is_empty());
        let s = OtaState::Done(healthy_monitor_report());
        assert!(s.rollback_reverse_path().is_empty());
    }

    #[test]
    fn r10_enter_sandbox_accepts_valid_manifest() {
        // SandboxValidator 返回 Accept → 进入 Sandboxed 状态 + 保留 verdict.
        let intent = sample_intent();
        let manifest = sample_manifest();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        let council = CouncilReviewer::new().review(&intent, all_approve_opinions());
        p.enter_council_review(council).unwrap();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut col = MultiSigCollector::new(cfg, hash.clone());
        for i in 0..5 {
            col.submit(PhysicalSignature::new(
                format!("signer-{i}"),
                hash.clone(),
                100,
                format!("sig{i}"),
            ))
            .unwrap();
        }
        p.enter_multisig(col.evaluate(200)).unwrap();

        let sandbox = DefaultSandbox;
        p.enter_sandbox(
            intent.id,
            "blue".into(),
            "green".into(),
            &manifest,
            &sandbox,
        )
        .unwrap();
        assert_eq!(p.stage(), OtaStage::Sandbox);
        match p.state() {
            OtaState::Sandboxed {
                blue_carrier,
                green_carrier,
                verdict,
                ..
            } => {
                assert_eq!(blue_carrier, "blue");
                assert_eq!(green_carrier, "green");
                assert!(matches!(verdict, SandboxVerdict::Accept));
            }
            _ => panic!("expected Sandboxed"),
        }
    }

    #[test]
    fn r10_enter_sandbox_rejects_e_layer_manifest_triggers_rollback() {
        // SandboxValidator 返回 Reject → 直接触发 Rollback.
        let intent = sample_intent();
        let e_layer_manifest = ManifestBuilder::new("v2.0.0", UpgradeKind::ELayerMutation)
            .with_content_hash("e")
            .build();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        let council = CouncilReviewer::new().review(&intent, all_approve_opinions());
        p.enter_council_review(council).unwrap();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut col = MultiSigCollector::new(cfg, hash.clone());
        for i in 0..5 {
            col.submit(PhysicalSignature::new(
                format!("signer-{i}"),
                hash.clone(),
                100,
                format!("sig{i}"),
            ))
            .unwrap();
        }
        p.enter_multisig(col.evaluate(200)).unwrap();

        let sandbox = DefaultSandbox;
        p.enter_sandbox(
            intent.id,
            "blue".into(),
            "green".into(),
            &e_layer_manifest,
            &sandbox,
        )
        .unwrap();
        // Reject → 应进入 Rollback 终态, from_stage = Sandbox.
        assert_eq!(p.stage(), OtaStage::Rollback);
        match p.state() {
            OtaState::Rollback { from_stage, .. } => {
                assert_eq!(*from_stage, OtaStage::Sandbox);
            }
            _ => panic!("expected Rollback from Sandbox"),
        }
    }

    #[test]
    fn r10_enter_sandbox_illegal_from_idle() {
        // 从 Idle 直接进入 Sandbox 不允许.
        let manifest = sample_manifest();
        let sandbox = DefaultSandbox;
        let mut p = OtaPipeline::new(OtaStage::Idle);
        let err = p
            .enter_sandbox(
                uuid::Uuid::new_v4(),
                "b".into(),
                "g".into(),
                &manifest,
                &sandbox,
            )
            .unwrap_err();
        match err {
            UpgradeError::IllegalTransition(from, to) => {
                assert_eq!(from, OtaStage::Idle);
                assert_eq!(to, OtaStage::Sandbox);
            }
            _ => panic!("expected IllegalTransition"),
        }
    }

    #[test]
    fn r10_enter_sandbox_illegal_from_intent() {
        // IntentDraft → Sandbox 跳过 Council/MultiSig 不允许.
        let intent = sample_intent();
        let manifest = sample_manifest();
        let sandbox = DefaultSandbox;
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent).unwrap();
        let err = p
            .enter_sandbox(
                uuid::Uuid::new_v4(),
                "b".into(),
                "g".into(),
                &manifest,
                &sandbox,
            )
            .unwrap_err();
        assert!(matches!(err, UpgradeError::IllegalTransition(_, _)));
    }

    #[test]
    fn r10_enter_switchover_illegal_from_intent_stage() {
        // 进入 Switchover 前 stage 必须是 Sandbox, 否则 IllegalTransition.
        let intent = sample_intent();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent).unwrap();
        let err = p.enter_switchover().unwrap_err();
        match err {
            UpgradeError::IllegalTransition(from, to) => {
                assert_eq!(from, OtaStage::IntentDraft);
                assert_eq!(to, OtaStage::Switchover);
            }
            _ => panic!("expected IllegalTransition"),
        }
    }

    /// round10-01 测试用: 总是 Reject 的 sandbox validator.
    struct AlwaysRejectSandbox;
    impl SandboxValidator for AlwaysRejectSandbox {
        fn validate(&self, _manifest: &UpgradeManifest) -> SandboxVerdict {
            SandboxVerdict::Reject("always reject".into())
        }
    }

    #[test]
    fn r10_custom_sandbox_rejects_everything() {
        // 自定义 sandbox 总是 reject → 进入 Rollback.
        let intent = sample_intent();
        let manifest = sample_manifest();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        let council = CouncilReviewer::new().review(&intent, all_approve_opinions());
        p.enter_council_review(council).unwrap();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut col = MultiSigCollector::new(cfg, hash.clone());
        for i in 0..5 {
            col.submit(PhysicalSignature::new(
                format!("signer-{i}"),
                hash.clone(),
                100,
                format!("sig{i}"),
            ))
            .unwrap();
        }
        p.enter_multisig(col.evaluate(200)).unwrap();

        let sandbox = AlwaysRejectSandbox;
        p.enter_sandbox(
            intent.id,
            "blue".into(),
            "green".into(),
            &manifest,
            &sandbox,
        )
        .unwrap();
        assert_eq!(p.stage(), OtaStage::Rollback);
        let path = p.state().rollback_reverse_path();
        // from_stage = Sandbox, 反向路径应以 Sandbox 开头, 以 Idle 结尾.
        assert_eq!(path[0], OtaStage::Sandbox);
        assert_eq!(path[path.len() - 1], OtaStage::Idle);
    }

    #[test]
    fn r10_rollback_at_sandbox_stage_records_from_sandbox() {
        // 在 Sandbox 阶段手动 rollback, from_stage 必须是 Sandbox.
        let intent = sample_intent();
        let manifest = sample_manifest();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        let council = CouncilReviewer::new().review(&intent, all_approve_opinions());
        p.enter_council_review(council).unwrap();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut col = MultiSigCollector::new(cfg, hash.clone());
        for i in 0..5 {
            col.submit(PhysicalSignature::new(
                format!("signer-{i}"),
                hash.clone(),
                100,
                format!("sig{i}"),
            ))
            .unwrap();
        }
        p.enter_multisig(col.evaluate(200)).unwrap();
        let sandbox = DefaultSandbox;
        p.enter_sandbox(
            intent.id,
            "blue".into(),
            "green".into(),
            &manifest,
            &sandbox,
        )
        .unwrap();
        assert_eq!(p.stage(), OtaStage::Sandbox);

        p.rollback("manual abort at sandbox").unwrap();
        match p.state() {
            OtaState::Rollback { from_stage, reason } => {
                assert_eq!(*from_stage, OtaStage::Sandbox);
                assert_eq!(reason, "manual abort at sandbox");
            }
            _ => panic!("expected Rollback"),
        }
        let path = p.state().rollback_reverse_path();
        assert_eq!(
            path,
            vec![
                OtaStage::Sandbox,
                OtaStage::MultiSig,
                OtaStage::CouncilReview,
                OtaStage::IntentDraft,
                OtaStage::Idle,
            ]
        );
    }

    #[test]
    fn r10_terminal_done_blocks_sandbox_transition() {
        // Done 终态后调用 enter_sandbox 应失败 (IllegalTransition).
        let intent = sample_intent();
        let manifest = sample_manifest();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        drive_to_sandbox(&mut p, &intent);
        p.enter_switchover().unwrap();
        p.enter_monitor(healthy_monitor_report()).unwrap();
        p.finalize(healthy_monitor_report()).unwrap();
        assert_eq!(p.stage(), OtaStage::Done);

        let sandbox = DefaultSandbox;
        let err = p
            .enter_sandbox(
                uuid::Uuid::new_v4(),
                "b".into(),
                "g".into(),
                &manifest,
                &sandbox,
            )
            .unwrap_err();
        match err {
            UpgradeError::IllegalTransition(from, to) => {
                assert_eq!(from, OtaStage::Done);
                assert_eq!(to, OtaStage::Sandbox);
            }
            _ => panic!("expected IllegalTransition"),
        }
    }

    #[test]
    fn r10_seven_stages_contain_sandbox_and_no_idle() {
        // 防御性测试: SEVEN_STAGES 包含 Sandbox 且不含 Idle/Rollback.
        let stages = OtaStage::SEVEN_STAGES;
        assert!(stages.contains(&OtaStage::Sandbox));
        assert!(!stages.contains(&OtaStage::Idle));
        assert!(!stages.contains(&OtaStage::Rollback));
    }

    #[test]
    fn r10_sandbox_state_carries_verdict_through_pipeline() {
        // Sandboxed 状态保留 SandboxVerdict 供后续审计.
        let intent = sample_intent();
        let manifest = sample_manifest();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        let council = CouncilReviewer::new().review(&intent, all_approve_opinions());
        p.enter_council_review(council).unwrap();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut col = MultiSigCollector::new(cfg, hash.clone());
        for i in 0..5 {
            col.submit(PhysicalSignature::new(
                format!("signer-{i}"),
                hash.clone(),
                100,
                format!("sig{i}"),
            ))
            .unwrap();
        }
        p.enter_multisig(col.evaluate(200)).unwrap();
        let sandbox = DefaultSandbox;
        p.enter_sandbox(
            intent.id,
            "blue".into(),
            "green".into(),
            &manifest,
            &sandbox,
        )
        .unwrap();

        match p.state() {
            OtaState::Sandboxed { verdict, .. } => {
                assert!(matches!(verdict, SandboxVerdict::Accept));
            }
            _ => panic!("expected Sandboxed"),
        }
        // 进入 Switchover 后 carriers 应保留.
        p.enter_switchover().unwrap();
        match p.state() {
            OtaState::SwitchedOver {
                blue_carrier,
                green_carrier,
                ..
            } => {
                assert_eq!(blue_carrier, "blue");
                assert_eq!(green_carrier, "green");
            }
            _ => panic!("expected SwitchedOver"),
        }
    }

    #[test]
    fn r10_reverse_path_after_council_hold_rollback() {
        // Council 阶段按住机制触发 Rollback → from_stage = CouncilReview.
        let intent = sample_intent();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        let mut ops = all_approve_opinions();
        ops[0] = CouncilOpinion::new(
            CouncilSeat::Principle,
            CouncilStance::StrongDisapprove,
            0.95,
            "no",
        );
        let report = CouncilReviewer::new().review(&intent, ops);
        p.enter_council_review(report).unwrap();
        assert_eq!(p.stage(), OtaStage::Rollback);
        let path = p.state().rollback_reverse_path();
        assert_eq!(
            path,
            vec![
                OtaStage::CouncilReview,
                OtaStage::IntentDraft,
                OtaStage::Idle,
            ]
        );
    }

    #[test]
    fn r10_rollback_from_monitor_records_from_stage() {
        // 验证 finalize() 触发 Rollback 时 from_stage = Monitor.
        let intent = sample_intent();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        drive_to_sandbox(&mut p, &intent);
        p.enter_switchover().unwrap();
        p.enter_monitor(healthy_monitor_report()).unwrap();
        let mut d = MonitorDashboard::new();
        d.record(MonitorMetric::new("err", 0.99, Some(0.05), None));
        p.finalize(d.report()).unwrap();
        match p.state() {
            OtaState::Rollback { from_stage, .. } => {
                assert_eq!(*from_stage, OtaStage::Monitor);
            }
            _ => panic!("expected Rollback from Monitor"),
        }
    }

    #[test]
    fn r10_sandbox_phase_number_is_four() {
        // 阶段编号锁定: Sandbox = 4 (替换原 Download).
        assert_eq!(OtaStage::Sandbox.phase_number(), 4);
        assert!(OtaStage::Sandbox.is_active());
        assert!(!OtaStage::Sandbox.is_terminal());
        assert!(!OtaStage::Sandbox.is_success());
        assert!(!OtaStage::Sandbox.is_rollback());
    }

    #[test]
    fn r10_intent_state_machine_unaffected_by_ota_change() {
        // 验证 Intent 状态机内部结构未被破坏.
        let intent = sample_intent();
        let mut sm = IntentStateMachine::wrap(intent);
        assert!(sm.submit().is_ok());
        assert!(sm.approve().is_ok());
        assert_eq!(sm.status(), crate::intent::IntentStatus::Approved);
    }

    #[test]
    fn r10_rollback_path_for_sandbox_then_full_reverse() {
        // Sandbox reject → rollback → 验证 reverse path 包含所有前置阶段直到 Idle.
        let intent = sample_intent();
        let e_layer = ManifestBuilder::new("v2.0.0", UpgradeKind::ELayerMutation)
            .with_content_hash("e")
            .build();
        let mut p = OtaPipeline::new(OtaStage::Idle);
        p.start_intent(intent.clone()).unwrap();
        let council = CouncilReviewer::new().review(&intent, all_approve_opinions());
        p.enter_council_review(council).unwrap();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut col = MultiSigCollector::new(cfg, hash.clone());
        for i in 0..5 {
            col.submit(PhysicalSignature::new(
                format!("signer-{i}"),
                hash.clone(),
                100,
                format!("sig{i}"),
            ))
            .unwrap();
        }
        p.enter_multisig(col.evaluate(200)).unwrap();
        let sandbox = DefaultSandbox;
        p.enter_sandbox(intent.id, "blue".into(), "green".into(), &e_layer, &sandbox)
            .unwrap();
        // Sandbox reject → Rollback, from_stage = Sandbox.
        let path = p.state().rollback_reverse_path();
        // 验证包含所有需要回退的阶段.
        assert!(path.contains(&OtaStage::Sandbox));
        assert!(path.contains(&OtaStage::MultiSig));
        assert!(path.contains(&OtaStage::CouncilReview));
        assert!(path.contains(&OtaStage::IntentDraft));
        assert!(path.contains(&OtaStage::Idle));
        // 验证顺序: Sandbox 应早于 MultiSig, MultiSig 早于 CouncilReview, 等等.
        let idx_s = path.iter().position(|s| *s == OtaStage::Sandbox).unwrap();
        let idx_m = path.iter().position(|s| *s == OtaStage::MultiSig).unwrap();
        let idx_c = path
            .iter()
            .position(|s| *s == OtaStage::CouncilReview)
            .unwrap();
        let idx_i = path
            .iter()
            .position(|s| *s == OtaStage::IntentDraft)
            .unwrap();
        let idx_idle = path.iter().position(|s| *s == OtaStage::Idle).unwrap();
        assert!(idx_s < idx_m);
        assert!(idx_m < idx_c);
        assert!(idx_c < idx_i);
        assert!(idx_i < idx_idle);
    }
}
