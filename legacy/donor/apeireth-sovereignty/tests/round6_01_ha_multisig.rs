//! round6-01 integration tests — HA M-of-N 多签字段补全
//!
//! 覆盖:
//! - HumanAuthority 三模式 (Single / Multi / Dynamic) 真实可用
//! - process_owner_request_with_authority 真实计算 (不再 stub)
//! - applications audit trail + revoke
//! - 守 7 项不修改承诺: 本文件不修改 LOCKED 文档 (reflection / governance / owner.rs 等)
//!
//! 用法: `cargo test -p apeireth-sovereignty --test round6_01_ha_multisig`

use apeireth_sovereignty::{
    AuthorityMode, AuthorityMultisigOutcome, HAAuthentication, HumanApproval, HumanAuthority,
    MultiSigPolicy, OwnerAction, OwnerRequest, OwnerToken, Signatory,
};

fn sigs(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_string()).collect()
}

#[test]
fn integration_single_mode_full_pipeline() {
    // 场景: 单人模式下 Master 请求 audit query, 1 个签名批准即可通过
    let policy = MultiSigPolicy {
        required: 1,
        signatories: vec![Signatory::new(
            "h-1",
            "Alice",
            HAAuthentication::WindowsHello,
        )],
    };
    let ha = HumanAuthority::single("h-1", "Alice");
    assert_eq!(ha.mode, AuthorityMode::Single);
    assert_eq!(ha.required_approvals, 1);
    assert_eq!(ha.threshold, 100);

    let mut ha_mut = ha.clone();
    ha_mut.record_approval(HumanApproval::new(
        "ap-1",
        "h-1",
        "Alice",
        1000,
        "audit_query",
    ));
    assert!(ha_mut.meets_authority(2000));

    let request = OwnerRequest::new(
        "req-1",
        OwnerToken::Master,
        OwnerAction::AuditQuery,
        "alice",
        "routine audit",
    );
    let collected = sigs(&["h-1"]);
    let outcome = policy.process_owner_request_with_authority(&request, &collected, &ha_mut, 2000);
    match outcome {
        AuthorityMultisigOutcome::Approved {
            signature_count,
            required,
            threshold,
            ..
        } => {
            assert_eq!(signature_count, 1);
            assert_eq!(required, 1);
            assert_eq!(threshold, 100);
        }
        _ => panic!("Single mode Master should be Approved, got {:?}", outcome),
    }
}

#[test]
fn integration_multi_mode_2_of_3_real_computation() {
    // 场景: 多人模式 2-of-3, Master 请求 core-rule 修改 — 必须 2 个签名
    let policy = MultiSigPolicy::default_2_of_3();
    let ha = HumanAuthority::multi("ha-board", "governance-board", 2, 3).unwrap();
    assert_eq!(ha.mode, AuthorityMode::Multi);
    assert_eq!(ha.required_approvals, 2);
    assert_eq!(ha.total_signatories, 3);
    assert_eq!(ha.threshold, 66); // round(2/3*100)

    // Case 1: 1 个签名 → InsufficientSignatures
    let request1 = OwnerRequest::new(
        "req-2a",
        OwnerToken::Master,
        OwnerAction::ModifyL0Threshold,
        "alice",
        "tune threshold",
    );
    let outcome1 =
        policy.process_owner_request_with_authority(&request1, &sigs(&["h-1"]), &ha, 1000);
    assert!(matches!(
        outcome1,
        AuthorityMultisigOutcome::InsufficientSignatures {
            collected: 1,
            required: 2,
            ..
        }
    ));

    // Case 2: 2 个签名 → Approved (66% == threshold)
    let outcome2 =
        policy.process_owner_request_with_authority(&request1, &sigs(&["h-1", "h-2"]), &ha, 1000);
    match outcome2 {
        AuthorityMultisigOutcome::Approved {
            signature_count,
            required,
            threshold,
            ..
        } => {
            assert_eq!(signature_count, 2);
            assert_eq!(required, 2);
            assert_eq!(threshold, 66);
        }
        _ => panic!("2-of-3 with 2 sigs should be Approved, got {:?}", outcome2),
    }

    // Case 3: 3 个签名 → Approved (100% > threshold)
    let outcome3 = policy.process_owner_request_with_authority(
        &request1,
        &sigs(&["h-1", "h-2", "h-3"]),
        &ha,
        1000,
    );
    assert!(matches!(
        outcome3,
        AuthorityMultisigOutcome::Approved { .. }
    ));
}

#[test]
fn integration_dynamic_mode_adaptive_threshold() {
    // 场景: 动态模式 — threshold 由调用方上下文决定, 此函数只看计数
    let policy = MultiSigPolicy::three_of_five();
    let ha = HumanAuthority::dynamic("ha-dynamic", "ctx-aware", 3, 50, 5);
    assert_eq!(ha.mode, AuthorityMode::Dynamic);

    let request = OwnerRequest::new(
        "req-3",
        OwnerToken::Master,
        OwnerAction::ModifyL0HumanAuthority,
        "alice",
        "rotate board",
    );

    // 2 个签名 < required=3 → InsufficientSignatures
    let outcome_insufficient =
        policy.process_owner_request_with_authority(&request, &sigs(&["h-0", "h-1"]), &ha, 1000);
    assert!(matches!(
        outcome_insufficient,
        AuthorityMultisigOutcome::InsufficientSignatures { .. }
    ));

    // 3 个签名 == required=3 → Approved (Dynamic 模式)
    let outcome_ok = policy.process_owner_request_with_authority(
        &request,
        &sigs(&["h-0", "h-1", "h-2"]),
        &ha,
        1000,
    );
    match outcome_ok {
        AuthorityMultisigOutcome::Approved {
            signature_count,
            required,
            threshold,
            ..
        } => {
            assert_eq!(signature_count, 3);
            assert_eq!(required, 3);
            assert_eq!(threshold, 50); // 用户定义
        }
        _ => panic!("Dynamic with 3/5 sigs should be Approved"),
    }

    // meets_authority: Dynamic 模式只看 required
    let mut ha = ha.clone();
    ha.record_approval(HumanApproval::new("ap-1", "h-0", "S0", 1000, "rotate"));
    ha.record_approval(HumanApproval::new("ap-2", "h-1", "S1", 1000, "rotate"));
    assert!(!ha.meets_authority(2000)); // 只有 2 < 3
    ha.record_approval(HumanApproval::new("ap-3", "h-2", "S2", 1000, "rotate"));
    assert!(ha.meets_authority(2000));
}

#[test]
fn integration_unknown_signatory_and_revoke_flow() {
    // 场景: 收集的签名有 1 个不在注册表, 必须 UnknownSignatory 拒绝
    let policy = MultiSigPolicy::default_2_of_3();
    let ha = HumanAuthority::multi("ha-board", "board", 2, 3).unwrap();
    let request = OwnerRequest::new(
        "req-4",
        OwnerToken::Master,
        OwnerAction::AuditQuery,
        "alice",
        "audit",
    );

    let collected = sigs(&["h-1", "h-evil"]);
    let outcome = policy.process_owner_request_with_authority(&request, &collected, &ha, 1000);
    match outcome {
        AuthorityMultisigOutcome::UnknownSignatory(s) => assert_eq!(s, "h-evil"),
        _ => panic!("Unknown signatory should be rejected, got {:?}", outcome),
    }

    // Audit trail + revoke
    let mut ha_log = HumanAuthority::multi("ha-board", "board", 2, 3).unwrap();
    ha_log.record_approval(HumanApproval::new("ap-1", "h-1", "Alice", 1000, "audit"));
    ha_log.record_approval(HumanApproval::new("ap-2", "h-2", "Bob", 1000, "audit"));
    assert_eq!(ha_log.valid_approval_count(2000), 2);
    assert!(ha_log.revoke_approval("ap-1"));
    assert_eq!(ha_log.valid_approval_count(2000), 1);
    assert!(!ha_log.revoke_approval("nonexistent"));
    assert_eq!(ha_log.valid_approval_percentage(2000), 33); // 1/3*100

    // Expired
    ha_log.record_approval(
        HumanApproval::new("ap-3", "h-3", "Carol", 1000, "audit").with_expiry(1500),
    );
    assert_eq!(ha_log.valid_approval_count(2000), 1); // ap-3 已过期
    assert_eq!(ha_log.valid_approval_count(1200), 2); // 过期前 ap-3 仍有效
}
