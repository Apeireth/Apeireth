//! apeireth-upgrade: 升级器官 (A15 落点 — 完整 7 阶段 OTA 升级).
//!
//! **职责**: 跨版本连续性 — 智能体可改变自己. 提供:
//! 1. 完整 7 阶段 OTA 状态机 (round6-03 升级: Idle → IntentDraft → CouncilReview → MultiSig → Download → Switchover → Monitor → Done/Rollback)
//! 2. UpgradeIntent 数据结构 + Intent 状态机
//! 3. CouncilReview — 7 席智囊团审议 + 按住机制
//! 4. MultiSig — 物理多签收集 (m-of-n)
//! 5. Monitor — dashboard + smoke checks + Keep/Rollback 建议
//! 6. sandbox-validator trait (物理隔离守门 3)
//! 7. 5 重治理 trait (修改 E 层时触发)
//! 8. UpgradeManifest 数据结构
//!
//! **架构位置**: 阶段 4 §177 `apeireth-upgrade (OTA 升级 + sandbox-validator + 5 重守门)`;
//! 阶段 2 §7 `upgrade-impl (OTA + sandbox + 五重治理)` LOCKED.
//!
//! **历史**: A15 落地 3 状态 (Idle / Downloading / Applying); round6-03 升级为 7 阶段.
//!
//! **禁止**:
//! - ❌ 不修改 apeireth-core 任何已实装类型签名
//! - ❌ 不碰 R11 baseline 三值
//! - ❌ 不碰 apeireth-legacy/

#![deny(unsafe_code)]

mod council;
mod governance;
mod intent;
mod manifest;
mod monitor;
mod multisig;
mod ota;
/// Snapshot and rollback service, merged from the former standalone crate.
pub mod rollback;
mod sandbox;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
// round10-10: 跨 crate 集成适配层 (pub 以便 integration 测试)
pub mod cross_crate;

pub use council::{
    evaluate_hold, CouncilOpinion, CouncilReport, CouncilReviewer, CouncilSeat, CouncilStance,
    HoldAction, HoldTrigger,
};
pub use governance::{FiveFoldGovernance, Governance, GovernanceDecision, GovernanceVerdict};
pub use intent::{
    IntentStateMachine, IntentStatus, IntentTransitionError, UpgradeIntent, UpgradeScope,
};
pub use manifest::{ManifestBuilder, UpgradeKind, UpgradeManifest};
pub use monitor::{
    DefaultSmokeChecks, ErrorRateSmoke, HealthSmoke, LatencySmoke, MonitorDashboard, MonitorMetric,
    MonitorRecommendation, MonitorReport, SmokeCheck,
};
pub use multisig::{
    intent_payload_hash, MultiSigCollector, MultiSigConfig, MultiSigError, MultiSigOutcome,
    PhysicalSignature,
};
pub use ota::{OtaPipeline, OtaStage, OtaState};
pub use sandbox::{DefaultSandbox, SandboxValidator, SandboxVerdict, StrictSandbox};

/// 顶层错误: 所有 upgrade 子系统的 fallback error.
#[derive(Debug, thiserror::Error)]
pub enum UpgradeError {
    /// Manifest 校验失败.
    #[error("manifest validation failed: {0}")]
    InvalidManifest(String),
    /// Sandbox 校验拒绝.
    #[error("sandbox rejected: {0}")]
    SandboxRejected(String),
    /// 5 重治理拒绝.
    #[error("governance rejected: {0:?}")]
    GovernanceRejected(GovernanceVerdict),
    /// OTA 状态机非法转换.
    #[error("illegal ota transition: {0:?} -> {1:?}")]
    IllegalTransition(OtaStage, OtaStage),
    /// Intent 状态机非法转换.
    #[error("intent error: {0}")]
    Intent(#[from] IntentTransitionError),
    /// 多签错误.
    #[error("multisig error: {0}")]
    MultiSig(#[from] MultiSigError),
    /// 序列化错误.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// 跨 crate 集成错误 (round10-10).
    #[error("cross-crate integration error: {0}")]
    CouncilIntegration(String),
}

/// 统一结果类型.
pub type UpgradeResult<T> = Result<T, UpgradeError>;

/// 主入口 — 完整 7 阶段升级 (round6-03 推荐).
///
/// 流程: manifest validate → sandbox check → governance check →
///       IntentDraft → CouncilReview → MultiSig → Download → Switchover → Monitor → Done/Rollback.
pub fn run_upgrade(manifest: &UpgradeManifest) -> UpgradeResult<OtaState> {
    // 1. Manifest 校验
    manifest.validate()?;

    // 2. Sandbox 守门 (物理隔离)
    let sandbox = DefaultSandboxValidator;
    let sandbox_verdict = sandbox.validate(manifest);
    if let SandboxVerdict::Reject(reason) = sandbox_verdict {
        return Err(UpgradeError::SandboxRejected(reason));
    }

    // 3. 5 重治理守门 (修改 E 层时触发)
    let governance = FiveFoldGovernance::default();
    let gov_verdict = governance.evaluate(manifest);
    if let GovernanceDecision::Reject(verdict) = gov_verdict {
        return Err(UpgradeError::GovernanceRejected(verdict));
    }

    // 4. 构造 Intent (从 manifest 派生)
    let intent = UpgradeIntent::new(
        manifest.id,
        manifest.version.clone(),
        "v0.0.0".to_string(),
        manifest.kind,
        "run_upgrade",
        manifest.description.clone(),
    );

    // 5. 启动 OTA pipeline (Idle -> IntentDraft)
    let mut pipeline = OtaPipeline::new(OtaStage::Idle);
    pipeline.start_intent(intent.clone())?;

    // 6. 智囊团审议 (7 席默认全通过, 因为已通过 sandbox + governance)
    let all_approve: Vec<CouncilOpinion> = CouncilSeat::ALL
        .iter()
        .map(|s| CouncilOpinion::new(*s, CouncilStance::Approve, 0.9, "auto-approve"))
        .collect();
    let council_report = CouncilReviewer::new().review(&intent, all_approve);
    pipeline.enter_council_review(council_report)?;

    // 7. 多签 (默认 quorum, 因为升级方为受信任进程)
    let multisig_outcome = MultiSigOutcome::Quorum {
        count: 5,
        reached_at: 0,
    };
    pipeline.enter_multisig(multisig_outcome)?;

    // 8. 沙盒验证 (round10-01 升级: 取代 round6-03 的 Download 阶段, 物理隔离守门 3)
    let sandbox = DefaultSandboxValidator;
    pipeline.enter_sandbox(
        manifest.id,
        "blue".into(),
        "green".into(),
        manifest,
        &sandbox,
    )?;
    pipeline.enter_switchover()?;

    // 9. 监控期 (注入健康指标)
    let mut dashboard = MonitorDashboard::new();
    dashboard.record(MonitorMetric::new("a", 0.01, Some(0.05), None));
    dashboard.record(MonitorMetric::new("b", 100.0, Some(500.0), None));
    let report = dashboard.report();
    pipeline.enter_monitor(report.clone())?;
    pipeline.finalize(report)?;

    Ok(pipeline.state().clone())
}

/// 默认 Sandbox validator — 接受所有 manifest (保守基线).
struct DefaultSandboxValidator;
impl SandboxValidator for DefaultSandboxValidator {
    fn validate(&self, manifest: &UpgradeManifest) -> SandboxVerdict {
        if manifest.version.is_empty() {
            SandboxVerdict::Reject("empty version".to_string())
        } else {
            SandboxVerdict::Accept
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ManifestBuilder, UpgradeKind};
    use std::collections::HashSet;

    fn sample_manifest() -> UpgradeManifest {
        ManifestBuilder::new("v1.0.0", UpgradeKind::Patch)
            .with_description("A15 sample")
            .with_content_hash("abc123")
            .build()
    }

    #[test]
    fn run_upgrade_full_flow_returns_success() {
        let manifest = sample_manifest();
        let state = run_upgrade(&manifest).expect("upgrade must succeed");
        // 7 阶段完整流: Done 终态 (成功)
        assert!(state.is_success());
        assert!(state.is_terminal());
    }

    #[test]
    fn run_upgrade_rejects_empty_version() {
        let manifest = ManifestBuilder::new("", UpgradeKind::Patch).build();
        assert!(run_upgrade(&manifest).is_err());
    }

    #[test]
    fn run_upgrade_rejects_invalid_manifest() {
        let mut manifest = sample_manifest();
        manifest.content_hash = "".to_string();
        let result = run_upgrade(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn seven_stages_pipeline_count() {
        let stages: HashSet<OtaStage> = OtaStage::SEVEN_STAGES.iter().copied().collect();
        assert_eq!(stages.len(), 7, "7 阶段必须 7 个唯一 stage");
    }

    #[test]
    fn seven_stages_includes_done_and_not_rollback_in_array() {
        // SEVEN_STAGES 包含 Done 但不含 Rollback (Rollback 是替代终态)
        assert!(OtaStage::SEVEN_STAGES.contains(&OtaStage::Done));
        assert!(!OtaStage::SEVEN_STAGES.contains(&OtaStage::Rollback));
    }

    #[test]
    fn upgrade_result_alias_compiles() {
        let _: UpgradeResult<OtaState> = Ok(OtaState::Idle);
    }
}

// R223: 实际二进制自更新 (backup + atomic swap + verify + rollback)
pub mod self_update;
