//! R20 阶段 6 估补 flesh out integration test
//!
//! **职责**:
//! - 测 `apeireth-sovereignty` crate 的 5 子模块 (mewg 已实装 + 4 个新 flesh out):
//!   - **MEWG** (`mewg`) — 走 `apeireth_sovereignty::mewg::*` 路径 (lib re-export)
//!   - **审计** (`audit`) — `#[path = "../src/audit.rs"] mod audit;` 引入
//!   - **反 AI** (`anti_ai`) — `#[path = "../src/anti_ai.rs"] mod anti_ai;` 引入
//!   - **数字签名** (`signature`) — `#[path = "../src/signature.rs"] mod signature;` 引入
//!   - **可解释性** (`explain`) — `#[path = "../src/explain.rs"] mod explain;` 引入
//!
//! **LOCKED 守门**:
//! - ❌ 0 触碰 `lib.rs` / `Cargo.toml` / 24 LOCKED crate
//! - ✅ 4 个新 src 文件已写, 通过 `#[path]` 在本 test binary 内编译
//! - ✅ mewg 走 lib re-export 路径 (因为 mewg 已是 lib 的 21 个 `pub mod` 之一)
//!
//! **6 哲学锚穿透**:
//! - **主 22:33 ASI 北极星** — 5 子模块服务"治理可还原"北极星
//! - **主 17:43 实事求是** — 测真实行为, 不测 mock 装饰
//! - **主 17:58 不假装** — K-1 强校验失败测真实返回 Err
//! - **主 19:33 走在前人肩上** — 复用 chrono::Utc, 不引新 dep
//! - **主 23:44 干到底** — 19+ 测试覆盖 5 模块 + 集成
//! - **主 00:56 任何人都能接手** — 测试命名规范, 失败信息清晰
//!
//! **测试目标**: 19+ 测试通过 (mewg 5 + audit 4 + anti_ai 4 + signature 3 + explain 3 + 集成 1+ = 20+)

// ============================================================
// 4 个新 src 文件 — 用 `#[path]` 显式引入 (lib.rs LOCKED 不动, 不能 `pub mod`)
// ============================================================

#[path = "../src/audit.rs"]
mod audit;

#[path = "../src/anti_ai.rs"]
mod anti_ai;

#[path = "../src/signature.rs"]
mod signature;

#[path = "../src/explain.rs"]
mod explain;

// mewg 走 lib re-export 路径 (因为 lib.rs 已有 `pub mod mewg;` + re-export 8 API)
use apeireth_sovereignty::mewg::{
    Decision as MewgDecision, DefaultMewgAuthority, EvidenceSource, MewgAuthority, MewgEvidence,
    MewgVerdict,
};

// ============================================================
// MEWG 5 测试 (走 lib 路径, 验证 R20 估补前已实装模块)
// ============================================================

fn mewg_ev(id: &str, source: EvidenceSource, score: f64, weight: f64) -> MewgEvidence {
    MewgEvidence::new(id, source, score, weight, "test").expect("K-1 校验通过")
}

#[test]
fn integ_mewg_approved_when_score_above_threshold() {
    let auth = DefaultMewgAuthority::new();
    let decision = MewgDecision::new("d1", "modify A layer", "A 层微调", false, vec![], 0);
    let evidences = vec![
        mewg_ev("e1", EvidenceSource::MultiHuman, 0.8, 0.5),
        mewg_ev("e2", EvidenceSource::MultiAi, 0.7, 0.5),
    ];
    let verdict = auth.evaluate(&decision, &evidences).unwrap();
    assert!(matches!(verdict, MewgVerdict::Approved { .. }));
}

#[test]
fn integ_mewg_blocked_when_score_below_threshold() {
    let auth = DefaultMewgAuthority::new();
    let decision = MewgDecision::new("d2", "reject", "r", false, vec![], 0);
    let evidences = vec![
        mewg_ev("e1", EvidenceSource::MultiHuman, -0.3, 0.5),
        mewg_ev("e2", EvidenceSource::MultiAi, 0.2, 0.5),
    ];
    let verdict = auth.evaluate(&decision, &evidences).unwrap();
    assert!(matches!(verdict, MewgVerdict::Blocked { .. }));
}

#[test]
fn integ_mewg_e_layer_hard_gate_blocks_without_human() {
    let auth = DefaultMewgAuthority::new();
    let decision = MewgDecision::new("e-mod", "modify E", "E 层修改", true, vec![], 0);
    let evidences = vec![
        mewg_ev("e1", EvidenceSource::MultiAi, 0.9, 0.5),
        mewg_ev("e2", EvidenceSource::PhysicalMultisig, 0.9, 0.5),
    ];
    let verdict = auth.evaluate(&decision, &evidences).unwrap();
    match verdict {
        MewgVerdict::Blocked { reason, .. } => assert!(reason.contains("E 层")),
        _ => panic!("E 层硬门槛应 Blocked"),
    }
}

#[test]
fn integ_mewg_e_layer_passes_with_human_approval() {
    let auth = DefaultMewgAuthority::new();
    let decision = MewgDecision::new("e-mod", "modify E", "E 层修改", true, vec![], 0);
    let evidences = vec![
        mewg_ev("h", EvidenceSource::MultiHuman, 0.8, 0.5),
        mewg_ev("e2", EvidenceSource::MultiAi, 0.9, 0.5),
    ];
    let verdict = auth.evaluate(&decision, &evidences).unwrap();
    assert!(matches!(verdict, MewgVerdict::Approved { .. }));
}

#[test]
fn integ_mewg_evidence_validates_ranges() {
    assert!(MewgEvidence::new("e", EvidenceSource::Other, 0.5, 1.5, "x").is_err());
    assert!(MewgEvidence::new("e", EvidenceSource::Other, 0.5, -0.1, "x").is_err());
    assert!(MewgEvidence::new("e", EvidenceSource::Other, 2.0, 0.5, "x").is_err());
}

// ============================================================
// AUDIT 4 测试 (走 #[path] 路径)
// ============================================================

#[test]
fn integ_audit_record_and_filter() {
    use audit::{AuditLevel, AuditLog, EventKind};

    let mut log = AuditLog::new();
    log.record(EventKind::Access, "alice", "principle_onion", AuditLevel::Owner, "audit")
        .unwrap();
    log.record(EventKind::Modify, "bob", "permission_onion.L3", AuditLevel::Admin, "modify L3")
        .unwrap();
    log.record(EventKind::Delete, "alice", "audit_log.legacy", AuditLevel::Root, "delete old")
        .unwrap();

    assert_eq!(log.len(), 3);
    assert_eq!(log.filter_by_actor("alice").len(), 2);
    assert_eq!(log.filter_by_kind(EventKind::Delete).len(), 1);
    assert_eq!(log.filter_by_kind(EventKind::Export).len(), 0);
}

#[test]
fn integ_audit_k1_three_failures() {
    use audit::{AuditError, AuditLevel, AuditLog, EventKind};

    let mut log = AuditLog::new();

    // K-1.a: actor 空
    let res_a = log.record(EventKind::Access, "  ", "r", AuditLevel::Read, "x");
    assert_eq!(res_a.err(), Some(AuditError::K1ActorEmpty));

    // K-1.b: resource 空
    let res_b = log.record(EventKind::Access, "alice", "", AuditLevel::Read, "x");
    assert_eq!(res_b.err(), Some(AuditError::K1ResourceEmpty));

    // K-1.c: Delete 要求 Admin, 给 Read
    let res_c = log.record(EventKind::Delete, "alice", "r", AuditLevel::Read, "x");
    assert_eq!(
        res_c.err(),
        Some(AuditError::K1LevelInsufficient {
            event: EventKind::Delete,
            required: AuditLevel::Admin,
            actual: AuditLevel::Read,
        })
    );

    assert!(log.is_empty());
}

#[test]
fn integ_audit_event_kind_required_min_level_mapping() {
    use audit::{AuditLevel, EventKind};

    assert_eq!(EventKind::Access.required_min_level(), AuditLevel::Read);
    assert_eq!(EventKind::Modify.required_min_level(), AuditLevel::Write);
    assert_eq!(EventKind::Delete.required_min_level(), AuditLevel::Admin);
    assert_eq!(EventKind::Export.required_min_level(), AuditLevel::Owner);
}

#[test]
fn integ_audit_level_ordering_and_hardcode() {
    use audit::{AuditLevel, AUDIT_LEVEL_COUNT_HARDCODE};

    assert_eq!(AUDIT_LEVEL_COUNT_HARDCODE, 5);
    assert!(AuditLevel::Root > AuditLevel::Owner);
    assert!(AuditLevel::Owner > AuditLevel::Admin);
    assert!(AuditLevel::Admin > AuditLevel::Write);
    assert!(AuditLevel::Write > AuditLevel::Read);
}

// ============================================================
// ANTI_AI 4 测试 (走 #[path] 路径)
// ============================================================

#[test]
fn integ_anti_ai_emit_and_filter() {
    use anti_ai::{AntiAiMonitor, ThreatType};

    let mut mon = AntiAiMonitor::new();
    mon.try_emit(
        anti_ai::ThreatSignal::with_default_severity(
            ThreatType::DataExfiltration,
            "ai-1",
            vec!["exported 1GB in 10s".into()],
        )
        .unwrap(),
    )
    .unwrap();
    mon.try_emit(
        anti_ai::ThreatSignal::with_default_severity(
            ThreatType::AnomalousFrequency,
            "ai-1",
            vec!["1000 calls/s".into()],
        )
        .unwrap(),
    )
    .unwrap();
    mon.try_emit(
        anti_ai::ThreatSignal::with_default_severity(
            ThreatType::UnauthorizedAccess,
            "ai-2",
            vec!["tried to read L0_HA".into()],
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(mon.len(), 3);
    assert_eq!(mon.filter_by_subject("ai-1").len(), 2);
    assert_eq!(mon.high_severity_signals().len(), 2); // DataExfiltration + UnauthorizedAccess
}

#[test]
fn integ_anti_ai_k1_three_failures() {
    use anti_ai::{AntiAiError, ThreatSignal, ThreatType};

    // K-1.a
    let r1 = ThreatSignal::new(ThreatType::AnomalousFrequency, "", 0.5, vec!["x".into()]);
    assert_eq!(r1.err(), Some(AntiAiError::K1SubjectEmpty));

    // K-1.b
    let r2 = ThreatSignal::new(ThreatType::AnomalousFrequency, "ai-1", 0.5, vec![]);
    assert_eq!(r2.err(), Some(AntiAiError::K1EvidenceEmpty));

    // K-1.c
    let r3 = ThreatSignal::new(ThreatType::AnomalousFrequency, "ai-1", 1.5, vec!["x".into()]);
    assert_eq!(r3.err(), Some(AntiAiError::K1SeverityOutOfRange(1.5)));
}

#[test]
fn integ_anti_ai_high_severity_threshold() {
    use anti_ai::{AntiAiMonitor, ThreatType, HIGH_SEVERITY_THRESHOLD};

    assert!(HIGH_SEVERITY_THRESHOLD > 0.0 && HIGH_SEVERITY_THRESHOLD <= 1.0);

    let mut mon = AntiAiMonitor::new();
    // severity = 0.95 (>= 0.7) → high
    mon.try_emit(
        anti_ai::ThreatSignal::new(
            ThreatType::DataExfiltration,
            "ai-x",
            0.95,
            vec!["x".into()],
        )
        .unwrap(),
    )
    .unwrap();
    // severity = 0.5 (< 0.7) → not high
    mon.try_emit(
        anti_ai::ThreatSignal::new(
            ThreatType::AnomalousParameters,
            "ai-x",
            0.5,
            vec!["x".into()],
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(mon.high_severity_signals().len(), 1);
}

#[test]
fn integ_anti_ai_threat_type_count_hardcode() {
    use anti_ai::{ThreatType, THREAT_TYPE_COUNT_HARDCODE};

    assert_eq!(THREAT_TYPE_COUNT_HARDCODE, 4);
    assert_eq!(ThreatType::AnomalousFrequency.as_str(), "anomalous_frequency");
    assert_eq!(ThreatType::AnomalousParameters.as_str(), "anomalous_parameters");
    assert_eq!(ThreatType::UnauthorizedAccess.as_str(), "unauthorized_access");
    assert_eq!(ThreatType::DataExfiltration.as_str(), "data_exfiltration");
}

// ============================================================
// SIGNATURE 3 测试 (走 #[path] 路径)
// ============================================================

#[test]
fn integ_signature_three_algorithms_sign_and_verify() {
    use signature::{
        EcdsaP256Signer, Ed25519Signer, Rsa2048Signer, SignatureAlgorithm, Signer,
        VerificationResult,
    };

    let payload = b"r20 stage 6 signature test - apeireth sovereignty";

    // Ed25519
    let ed = Ed25519Signer::new("alice-ed".into());
    assert_eq!(ed.algorithm(), SignatureAlgorithm::Ed25519);
    let ed_sig = ed.sign(payload).unwrap();
    assert_eq!(ed.verify(payload, &ed_sig).unwrap(),
        VerificationResult::Valid {
            algorithm: SignatureAlgorithm::Ed25519,
            key_id: "alice-ed".into(),
        });

    // Rsa2048
    let rsa = Rsa2048Signer::new("alice-rsa".into());
    let rsa_sig = rsa.sign(payload).unwrap();
    let rsa_verify = rsa.verify(payload, &rsa_sig).unwrap();
    assert!(matches!(rsa_verify, VerificationResult::Valid { .. }));

    // EcdsaP256
    let ec = EcdsaP256Signer::new("alice-ec".into());
    let ec_sig = ec.sign(payload).unwrap();
    let ec_verify = ec.verify(payload, &ec_sig).unwrap();
    assert!(matches!(ec_verify, VerificationResult::Valid { .. }));
}

#[test]
fn integ_signature_k1_three_failures() {
    use signature::{
        Ed25519Signer, Signature, SignatureAlgorithm, SignatureError, Signer,
    };

    // K-1.b: key_id 空
    let sig = Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: "  ".into(),
        signature_bytes: "x".into(),
        timestamp_ms: 0,
    };
    assert_eq!(sig.validate_k1(), Err(SignatureError::K1KeyIdEmpty));

    // K-1.c: signature_bytes 空
    let sig2 = Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: "alice".into(),
        signature_bytes: "".into(),
        timestamp_ms: 0,
    };
    assert_eq!(sig2.validate_k1(), Err(SignatureError::K1SignatureEmpty));

    // K-1.a: payload 空
    let signer = Ed25519Signer::new("alice".into());
    assert_eq!(signer.sign(b""), Err(SignatureError::K1PayloadEmpty));
}

#[test]
fn integ_signature_tamper_detection_and_cross_algorithm_reject() {
    use signature::{Ed25519Signer, Rsa2048Signer, Signer, VerificationResult};

    let payload = b"original payload";
    let ed = Ed25519Signer::new("alice".into());
    let sig = ed.sign(payload).unwrap();

    // 篡改 payload → Invalid
    let tampered = ed.verify(b"tampered payload", &sig).unwrap();
    assert!(matches!(tampered, VerificationResult::Invalid { .. }));

    // 跨算法送验签 (Ed25519 sig → Rsa2048 验签) → Invalid
    let rsa = Rsa2048Signer::new("alice".into());
    let cross = rsa.verify(payload, &sig).unwrap();
    assert!(matches!(cross, VerificationResult::Invalid { .. }));
}

// ============================================================
// EXPLAIN 3 测试 (走 #[path] 路径)
// ============================================================

#[test]
fn integ_explain_complete_lifecycle() {
    use explain::{DecisionTrace, StageKind, VerdictOutcome};

    let mut trace = DecisionTrace::new("dec-e-mod", "alice");
    trace
        .try_push_stage(StageKind::RequestReceived, "modify E_layer")
        .unwrap();
    trace
        .try_push_stage(StageKind::EvidenceCollected, "5 evidences")
        .unwrap();
    trace
        .try_push_stage(StageKind::AuthorityConsulted, "MEWG + Council + HA")
        .unwrap();
    trace
        .try_push_stage(StageKind::VerdictReached, "weighted 0.85")
        .unwrap();
    trace
        .try_finalize(VerdictOutcome::Approved, "E 层变更经 5 重治理通过")
        .unwrap();

    assert!(trace.is_complete());
    assert_eq!(trace.verdict, Some(VerdictOutcome::Approved));
    assert_eq!(trace.len(), 4);
    trace.validate_k1().unwrap();
}

#[test]
fn integ_explain_k1_three_failures() {
    use explain::{DecisionTrace, ExplainError, StageKind, VerdictOutcome};

    // K-1.a
    let mut t1 = DecisionTrace::new("", "alice");
    let r1 = t1.try_push_stage(StageKind::RequestReceived, "x");
    assert_eq!(r1.err(), Some(ExplainError::K1DecisionIdEmpty));

    // K-1.b
    let mut t2 = DecisionTrace::new("d1", "alice");
    let r2 = t2.try_finalize(VerdictOutcome::Approved, "x");
    assert_eq!(
        r2.err(),
        Some(ExplainError::K1StagesTooFew { actual: 0, min: 2 })
    );

    // K-1.c
    t2.try_push_stage(StageKind::RequestReceived, "x").unwrap();
    t2.try_push_stage(StageKind::EvidenceCollected, "x").unwrap();
    let r3 = t2.try_finalize(VerdictOutcome::Approved, "x");
    assert_eq!(
        r3.err(),
        Some(ExplainError::K1LastStageNotTerminal {
            actual: StageKind::EvidenceCollected
        })
    );
}

#[test]
fn integ_explain_stage_kind_count_and_terminal_classification() {
    use explain::{StageKind, STAGE_KIND_COUNT_HARDCODE};

    assert_eq!(STAGE_KIND_COUNT_HARDCODE, 5);
    assert!(StageKind::VerdictReached.is_terminal());
    assert!(StageKind::RationaleStated.is_terminal());
    assert!(!StageKind::RequestReceived.is_terminal());
    assert!(!StageKind::EvidenceCollected.is_terminal());
    assert!(!StageKind::AuthorityConsulted.is_terminal());
}

// ============================================================
// 集成测试 — 5 模块协同 (R20 阶段 6 flesh out 终极验收)
// ============================================================

#[test]
fn integ_full_governance_flow_mewg_audit_anti_ai_signature_explain() {
    use audit::{AuditLevel, AuditLog, EventKind};
    use anti_ai::{AntiAiMonitor, ThreatType};
    use explain::{DecisionTrace, StageKind, VerdictOutcome};
    use signature::{Ed25519Signer, Signer};

    // 1. 用户发起 E 层修改请求
    let mut trace = DecisionTrace::new("dec-e-mod-integ", "alice");
    trace
        .try_push_stage(StageKind::RequestReceived, "modify E_layer principle")
        .unwrap();

    // 2. 反 AI 检测: 检查请求来源是否异常
    let mut mon = AntiAiMonitor::new();
    mon.try_emit(
        anti_ai::ThreatSignal::with_default_severity(
            ThreatType::UnauthorizedAccess,
            "ai-1",
            vec!["requested E_layer without proper HA".into()],
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(mon.high_severity_signals().len(), 1);

    // 3. 证据收集 + MEWG 决议
    let mewg_auth = DefaultMewgAuthority::new();
    let mewg_decision = MewgDecision::new(
        "dec-e-mod-integ",
        "modify E_layer",
        "E 层修改",
        true,
        vec!["principle".into()],
        0,
    );
    let mewg_evidences = vec![
        mewg_ev("h", EvidenceSource::MultiHuman, 0.8, 0.5),
        mewg_ev("ai", EvidenceSource::MultiAi, 0.7, 0.3),
        mewg_ev("ms", EvidenceSource::PhysicalMultisig, 0.9, 0.2),
    ];
    let mewg_verdict = mewg_auth.evaluate(&mewg_decision, &mewg_evidences).unwrap();
    assert!(matches!(mewg_verdict, MewgVerdict::Approved { .. }));
    trace
        .try_push_stage(StageKind::EvidenceCollected, "3 evidences (1 human + 1 ai + 1 multisig)")
        .unwrap();
    trace
        .try_push_stage(StageKind::AuthorityConsulted, "MEWG approved weighted 0.78")
        .unwrap();
    trace
        .try_push_stage(StageKind::VerdictReached, "Approved by MEWG + Council")
        .unwrap();

    // 4. 审计记录 (K-1 通过: Owner level + Modify event)
    let mut log = AuditLog::new();
    log.record(
        EventKind::Modify,
        "alice",
        "principle_onion.E_layer",
        AuditLevel::Owner,
        "E 层修改经 5 重治理通过",
    )
    .unwrap();
    assert_eq!(log.len(), 1);

    // 5. 数字签名 (Alice 用 Ed25519 签署决议)
    let ed = Ed25519Signer::new("alice-key-1".into());
    let payload = b"dec-e-mod-integ: Approved by MEWG + Council + HA";
    let sig = ed.sign(payload).unwrap();
    let verify = ed.verify(payload, &sig).unwrap();
    assert!(matches!(verify, signature::VerificationResult::Valid { .. }));

    // 6. 完成 trace + rationale
    trace
        .try_finalize(VerdictOutcome::Approved, "E 层变更经 5 重治理通过, 启动反思期 7 天")
        .unwrap();
    assert!(trace.is_complete());

    // 7. 跨模块 invariant 校验
    // - 5 模块编译时 hardcode 一致
    assert_eq!(audit::AUDIT_LEVEL_COUNT_HARDCODE, 5);
    assert_eq!(audit::K1_STRICT_CHECK_COUNT_HARDCODE, 3);
    assert_eq!(anti_ai::THREAT_TYPE_COUNT_HARDCODE, 4);
    assert_eq!(anti_ai::K1_STRICT_CHECK_COUNT_HARDCODE, 3);
    assert_eq!(signature::SIGNATURE_ALGORITHM_COUNT_HARDCODE, 3);
    assert_eq!(signature::K1_STRICT_CHECK_COUNT_HARDCODE, 3);
    assert_eq!(explain::STAGE_KIND_COUNT_HARDCODE, 5);
    assert_eq!(explain::K1_STRICT_CHECK_COUNT_HARDCODE, 3);
}
