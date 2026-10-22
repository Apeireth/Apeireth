//! B7 · 治理链端到端合练（现役 M-of-N 逻辑票链）——把「零件在、整条链无执行
//! 记录」（00-backlog.md D8）变成可复跑的执行证据。
//!
//! # 链路图（本测试逐步走的顺序，每步落审计）
//! 1. **提案** —— `gate::Action`（L0 类变更）过 V1+V2+V3 AND 门
//!    (`ActionGuard::check_action`)，V3 即「HA 在场」核对（MultiHuman 需
//!    登记真人 ≥ multi_sign.required）；
//! 2. **审议意见记名** —— `rubric::Ballot`（advisor_id 记名）→ `synthesize`
//!    （反对意见记名入 `dissenting`）→ `passes_strategy`（超多数策略）；
//! 3. **M-of-N 逻辑票** —— `onion::HumanAuthority::verify_multisig`（2-of-3，
//!    去重真实人类、拒未登记者、拒畸形票）三路径：通过 / 拒绝 / 超时；
//! 4. **审批闭合** —— `approval_closure::ApprovalAuditPair`（asked↔decision
//!    成对提交）+ `scan_approval_pairs` + `may_resolve_approval`（谁可决）；
//! 5. **审计落盘** —— `audit::AuditHashChain` 追加（谁/何时/什么决定写入
//!    subject，时间显式注入）→ JSON 落盘 → 读回 `verify()` → 篡改检出；
//! 6. **反思期** —— `core::SelfDisableAudit::record_reflection_query` +
//!    白名单/元问题禁令判定 + 违规报 HA。
//!
//! # 0 装口径（严守）
//! - **M-of-N 逻辑票 ≠ 密码学多签**：签名是 `"signer:digest"` 字符串票
//!   （`onion.rs` 自述 0 装占位，非真 crypto 校验），物理不可伪造的多签留
//!   v2.1（stage123-gap-matrix B7d 显式挂账）。本测试只钉「票数治理语义」。
//! - **编排未接线**：库里没有单一编排函数把 6 步串成链；串联顺序与「hold/超时
//!   ⇒ 不批准」的组合规则是本演练定义的胶水（各环节判定本身用现役真实实现）。
//!   生产调用点仍 `multi_sign: None`（全库），`verify_multisig` 的逐动作签名
//!   收集在生产面**未接线**——本测试是「链可走通」证据，不是「生产已接线」证据。
//! - V1 哲学守门用 mock 放行（演练对象是 V3 多签链，不是 V1 词表）。
//! - 落盘走 fs_err（clippy.toml 约束），测试面同样适用。

use apeireth_core::gate::{Action, ActionGuard, ActionTarget, ActionVerdict, Gate, RiskLevel};
use apeireth_core::kernel::Timestamp;
use apeireth_core::onion::{
    HAAuthentication, HAMode, HumanAuthority, MultiSignPolicy, MultiSignResult, PermissionLayer,
    PermissionOnion, RealHuman,
};
use apeireth_core::philosophy::{PhilosophyGuard, PhilosophyVerdict};
use apeireth_core::{is_in_reflection_whitelist, SelfDisableAudit};
use apeireth_governance::approval_closure::{
    commit_approval_audit_pair, scan_approval_pairs, ApprovalAuditPair, ApprovalOutcome,
    MemoryApprovalAuditSink,
};
use apeireth_governance::audit::{AuditChainError, AuditHashChain};
use apeireth_governance::rubric::{
    passes_strategy, synthesize, Ballot, HoldThreshold, HoldTrigger, StanceKind, VotingStrategy,
    HOLD_DELIBERATION_TIMEOUT_MS,
};
use apeireth_governance::{may_resolve_approval, UserRole};

/// mock 批准者签名票：`"signer:digest"` 字符串票（M-of-N 逻辑票口径）。
fn vote(signer: &str) -> String {
    format!("{signer}:drill-digest")
}

/// 演练提案：L0 类变更（重组洋葱，Critical），触 V3 HA 门。
fn proposal() -> Action {
    Action {
        id: "b7-drill-l0-change".to_string(),
        description: "B7 演练提案：L0 类变更（模拟核心升级）".to_string(),
        risk_level: RiskLevel::Critical,
        target: ActionTarget::ReorganizeOnion,
    }
}

/// 演练权限洋葱：L0/L3/L4/L5 触 HA 门。
fn permission_onion() -> PermissionOnion {
    let layer = |name: &str, requires_ha: bool| PermissionLayer {
        name: name.to_string(),
        description: format!("{name} 演练层"),
        requires_ha,
    };
    PermissionOnion {
        l0: layer("L0", true),
        l1: layer("L1", false),
        l2: layer("L2", false),
        l3: layer("L3", true),
        l4: layer("L4", true),
        l5: layer("L5", true),
    }
}

/// 2-of-3 现役逻辑票配置（MultiHuman + M-of-N 策略；3 名登记真人）。
fn ha_2of3() -> HumanAuthority {
    let human = |id: &str| RealHuman {
        id: id.to_string(),
        name: format!("演练真人 {id}"),
        authentication: HAAuthentication::FIDO2,
        biometric_data: None,
    };
    HumanAuthority {
        mode: HAMode::MultiHuman,
        real_humans: vec![human("alice"), human("bob"), human("carol")],
        ice_frozen_until: None,
        multi_sign: Some(MultiSignPolicy {
            required: 2,
            public_keys: None,
        }),
    }
}

/// V1 哲学守门 mock（演练对象是 V3 多签链；V1 词表另有测试面）。
struct PermissivePhilosophy;

impl PhilosophyGuard for PermissivePhilosophy {
    fn check_philosophy(&self, _action: &Action) -> PhilosophyVerdict {
        PhilosophyVerdict::Allow
    }
}

/// 审计落一步：谁/何时/什么决定写进 subject，时间显式注入（不取墙钟）。
fn step(chain: &mut AuditHashChain, event: &str, subject: &str, at_ms: i64) {
    chain.append(
        event,
        subject,
        Timestamp::from_epoch_millis(at_ms).expect("演练时间戳合法"),
    );
}

/// 落盘往返校验：JSON 写盘 → 读回逐记录等价 → 链校验过 → 篡改一字必被检出。
fn persist_verify_and_tamper_check(chain: &AuditHashChain, case: &str) {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-gov-chain-drill-{}-{case}",
        std::process::id()
    ));
    fs_err::create_dir_all(&dir).unwrap();
    let path = dir.join("audit-chain.json");
    let json = serde_json::to_string_pretty(chain).unwrap();
    fs_err::write(&path, json.as_bytes()).unwrap();
    let reloaded: AuditHashChain =
        serde_json::from_str(&fs_err::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(reloaded.records(), chain.records(), "落盘往返逐记录等价");
    reloaded.verify().expect("落盘读回链校验必须过");

    // 对抗：篡改 subject 一字（JSON 里找 b7-drill 标记），链校验必须报红。
    let tampered = json.replacen("b7-drill", "b7-dr!ll", 1);
    assert_ne!(tampered, json, "篡改样本必须真的改了字节");
    let forged: AuditHashChain = serde_json::from_str(&tampered).unwrap();
    assert!(
        matches!(
            forged.verify(),
            Err(AuditChainError::RecordHashMismatch { .. })
        ),
        "篡改必须被哈希链检出（fail-closed）"
    );
    let _ = fs_err::remove_dir_all(&dir);
}

/// 路径一（通过）：提案过三门 → 记名审议过超多数 → 2-of-3 逻辑票通过 →
/// owner 记名批准 + ask↔decision 成对闭合 → 审计落盘校验 → 反思期记账。
#[test]
fn chain_pass_path_ratifies_and_persists_audit() {
    let mut chain = AuditHashChain::new();
    let mut sink = MemoryApprovalAuditSink::new();
    let proposal = proposal();
    let onion = permission_onion();
    let ha = ha_2of3();

    // ① 提案过 V1+V2+V3 AND 门。
    let verdict = ActionGuard::check_action(&proposal, &PermissivePhilosophy, &onion, &ha);
    assert_eq!(verdict, ActionVerdict::Allow, "三门全过提案才准入");
    step(
        &mut chain,
        "proposal.admitted",
        "proposal=b7-drill-pass; target=ReorganizeOnion; gate=V1+V2+V3",
        1_000,
    );
    // 对抗：登记真人空集必须被 V3 拒（防假放行）。
    let mut empty = ha_2of3();
    empty.real_humans.clear();
    assert!(
        matches!(
            ActionGuard::check_action(&proposal, &PermissivePhilosophy, &onion, &empty),
            ActionVerdict::BlockByHumanAuthority(_)
        ),
        "登记真人空集必须被 V3 拒（fail-closed）"
    );

    // ② 审议意见记名：7 席具名票（5 赞成 / 1 反对 / 1 弃权）。
    let ballots = vec![
        Ballot::new("advisor-safety", StanceKind::StrongApprove, 0.9, 1.0),
        Ballot::new("advisor-philosophy", StanceKind::Approve, 0.8, 1.0),
        Ballot::new("advisor-ethics", StanceKind::Approve, 0.75, 1.0),
        Ballot::new("advisor-strategy", StanceKind::Approve, 0.7, 1.0),
        Ballot::new("advisor-performance", StanceKind::Approve, 0.65, 1.0),
        Ballot::new("advisor-history", StanceKind::Disapprove, 0.6, 1.0),
        Ballot::new("advisor-legal", StanceKind::Abstain, 0.5, 1.0),
    ];
    for (index, ballot) in ballots.iter().enumerate() {
        step(
            &mut chain,
            "deliberation.ballot",
            &format!(
                "proposal=b7-drill-pass; advisor={}; stance={:?}",
                ballot.advisor_id, ballot.stance
            ),
            2_000 + index as i64,
        );
    }
    let report = synthesize(&ballots);
    assert_eq!(report.opinion_count, 6, "弃权不计票（7 席 1 弃权）");
    assert_eq!(
        report.dissenting,
        vec!["advisor-history".to_string()],
        "反对意见具名入册"
    );
    assert!(!report.hold.is_held(), "通过路径不触发 hold");
    assert!(
        passes_strategy(VotingStrategy::Supermajority, &ballots, &report),
        "记名票过超多数策略"
    );
    assert!(
        passes_strategy(VotingStrategy::WeightedMajority, &ballots, &report),
        "加权多数同判通过"
    );

    // ③ M-of-N 逻辑票：2-of-3 具名有效票 → 通过。
    let result = ha.verify_multisig(&[vote("alice"), vote("bob")]);
    assert_eq!(result, MultiSignResult::Accepted, "2-of-3 具名有效票通过");
    step(
        &mut chain,
        "multisig.verified",
        "proposal=b7-drill-pass; signers=alice,bob; policy=2-of-3; result=accepted",
        3_000,
    );

    // ④ 审批闭合：谁可决（owner 记名）+ asked↔decision 成对落账。
    assert!(
        may_resolve_approval(UserRole::Owner, "owner-alice", Some("owner-alice")),
        "owner 可决本人会话"
    );
    let pair = ApprovalAuditPair::new(
        "pair-b7-pass",
        1,
        "core.upgrade:L0",
        ApprovalOutcome::AllowedOnce,
    );
    commit_approval_audit_pair(&mut sink, &pair).expect("内存 sink 成对提交必成");
    let scan = scan_approval_pairs(sink.records());
    assert!(
        scan.unclosed.is_empty() && scan.rejected.is_empty(),
        "asked↔decision 成对闭合"
    );
    step(
        &mut chain,
        "approval.decision",
        "proposal=b7-drill-pass; resolver=owner-alice; outcome=allowed_once",
        4_000,
    );

    // ⑤ 反思期：白名单放行记账、元问题禁令拦下、违规报 HA。
    let mut reflection = SelfDisableAudit::new();
    let allowed = reflection.record_reflection_query("如何更好地服务主人".to_string(), 5_000);
    assert!(
        !allowed && is_in_reflection_whitelist("如何更好地服务主人"),
        "白名单反思查询放行"
    );
    let blocked = reflection.record_reflection_query("是否需要 L0 HA?".to_string(), 5_001);
    assert!(blocked, "元问题禁令拦下");
    assert_eq!(reflection.whitelist_hits(), 1, "白名单命中记账");
    assert_eq!(reflection.total_violations(), 1, "违规计数真实");
    assert!(reflection.needs_ha_alert(), "违规必报 HA");
    step(
        &mut chain,
        "reflection.period",
        "proposal=b7-drill-pass; whitelist_hits=1; violations=1",
        6_000,
    );
    assert_eq!(
        Gate::ReflectionAudit.name(),
        "反思期审计",
        "反思期审计门词表钉住"
    );

    // ⑥ 审计落盘校验 + 篡改检出。
    chain.verify().expect("链校验必须过");
    assert_eq!(
        chain.len(),
        11,
        "1 提案 + 7 记名票 + 1 多签 + 1 决定 + 1 反思期"
    );
    let multisig = chain
        .records()
        .iter()
        .find(|record| record.event_kind == "multisig.verified")
        .expect("多签审计记录必须存在");
    assert!(
        multisig.subject.contains("alice,bob") && multisig.subject.contains("accepted"),
        "谁/什么决定都在记录里：{}",
        multisig.subject
    );
    assert_eq!(
        multisig.timestamp.epoch_millis(),
        3_000,
        "何时（显式注入时间）在记录里"
    );
    persist_verify_and_tamper_check(&chain, "pass");
}

/// 路径二（拒绝）：记名审议否决（强反多数 + 全体反对两形态）→ 逻辑票不足
/// （含防凑票对抗）→ Rejected 成对闭合 → 落盘校验。
#[test]
fn chain_reject_path_pins_named_dissent_and_insufficient_votes() {
    let mut chain = AuditHashChain::new();
    let mut sink = MemoryApprovalAuditSink::new();
    let ha = ha_2of3();

    // ① 记名审议否决（形态 A：强反对 ≥30% → StrongDisapprovePercent hold）。
    let strong = vec![
        Ballot::new("advisor-safety", StanceKind::StrongDisapprove, 0.9, 1.0),
        Ballot::new("advisor-ethics", StanceKind::StrongDisapprove, 0.8, 1.0),
        Ballot::new("advisor-strategy", StanceKind::Disapprove, 0.7, 1.0),
    ];
    let report_strong = synthesize(&strong);
    assert!(report_strong.hold.is_held(), "强反对触发 hold");
    assert!(
        matches!(
            &report_strong.hold.trigger,
            Some(HoldTrigger {
                threshold: HoldThreshold::StrongDisapprovePercent { .. },
                ..
            })
        ),
        "hold 阈值形态钉住：强反对占比"
    );
    assert!(
        !passes_strategy(VotingStrategy::Supermajority, &strong, &report_strong),
        "否决票不得过策略"
    );
    // 形态 B：全体反对 → UnanimousDisapprove。
    let unanimous = vec![
        Ballot::new("advisor-safety", StanceKind::Disapprove, 0.9, 1.0),
        Ballot::new("advisor-ethics", StanceKind::Disapprove, 0.8, 1.0),
        Ballot::new("advisor-strategy", StanceKind::Disapprove, 0.7, 1.0),
    ];
    let report_unanimous = synthesize(&unanimous);
    assert!(
        matches!(
            &report_unanimous.hold.trigger,
            Some(HoldTrigger {
                threshold: HoldThreshold::UnanimousDisapprove { .. },
                ..
            })
        ),
        "hold 阈值形态钉住：全体反对"
    );
    for (index, ballot) in strong.iter().enumerate() {
        step(
            &mut chain,
            "deliberation.ballot",
            &format!(
                "proposal=b7-drill-reject; advisor={}; stance={:?}",
                ballot.advisor_id, ballot.stance
            ),
            1_000 + index as i64,
        );
    }

    // ② M-of-N 逻辑票不足 + 防凑票对抗（同人重复/未登记者/畸形票均不计数）。
    assert_eq!(
        ha.verify_multisig(&[vote("alice")]),
        MultiSignResult::Insufficient {
            required: 2,
            received: 1
        },
        "1/2 票不足"
    );
    assert_eq!(
        ha.verify_multisig(&[
            vote("alice"),
            vote("alice"),
            vote("mallory"),
            "malformed".to_string(),
            String::new(),
        ]),
        MultiSignResult::Insufficient {
            required: 2,
            received: 1
        },
        "同人凑票/未登记者/畸形票均不计数（对抗）"
    );
    // 配置对抗：offline 恒拒；single 模式带策略必拒。
    let mut offline = ha_2of3();
    offline.mode = HAMode::Offline;
    assert_eq!(
        offline.verify_multisig(&[vote("alice"), vote("bob")]),
        MultiSignResult::DeniedOffline,
        "offline 恒拒"
    );
    let mut misconfigured = ha_2of3();
    misconfigured.mode = HAMode::SingleHuman;
    assert!(
        matches!(
            misconfigured.verify_multisig(&[vote("alice")]),
            MultiSignResult::InvalidHA(_)
        ),
        "single 模式带 multi_sign 策略必拒（配置对抗）"
    );
    step(
        &mut chain,
        "multisig.verified",
        "proposal=b7-drill-reject; signers=alice; policy=2-of-3; result=insufficient",
        2_000,
    );

    // ③ 拒绝闭合（fail-closed）+ 谁可决对抗（跨用户审批一律拒）。
    assert!(
        !may_resolve_approval(UserRole::Member, "member-bob", Some("owner-alice")),
        "跨用户审批一律拒（Member 有 ApprovalL2 也拒）"
    );
    let outcome = ApprovalOutcome::Rejected;
    assert!(outcome.fail_closed(), "拒绝即 fail-closed，不授权任何动作");
    let pair = ApprovalAuditPair::new("pair-b7-reject", 1, "core.upgrade:L0", outcome);
    commit_approval_audit_pair(&mut sink, &pair).expect("内存 sink 成对提交必成");
    let scan = scan_approval_pairs(sink.records());
    assert!(
        scan.unclosed.is_empty() && scan.rejected.is_empty(),
        "拒绝路径同样 asked↔decision 成对闭合"
    );
    step(
        &mut chain,
        "approval.decision",
        "proposal=b7-drill-reject; resolver=owner-alice; outcome=rejected",
        3_000,
    );

    // ④ 落盘校验 + 篡改检出。
    chain.verify().expect("链校验必须过");
    persist_verify_and_tamper_check(&chain, "reject");
}

/// 路径三（超时）：审议超时阈值边界钉住 → hold → 不批准（Unavailable，
/// fail-closed）→ 成对闭合 → 落盘校验。
#[test]
fn chain_timeout_path_holds_and_fails_closed() {
    let mut chain = AuditHashChain::new();
    let mut sink = MemoryApprovalAuditSink::new();

    // ① 超时阈值边界（60_000ms）：差 1ms 不触发、到阈值必触发。
    assert!(
        HoldTrigger::evaluate_timeout(HOLD_DELIBERATION_TIMEOUT_MS - 1).is_none(),
        "阈值前 1ms 不触发 hold"
    );
    let trigger =
        HoldTrigger::evaluate_timeout(HOLD_DELIBERATION_TIMEOUT_MS).expect("到阈值必须触发 hold");
    assert!(
        matches!(
            &trigger.threshold,
            HoldThreshold::DeliberationTimeout {
                threshold_ms: HOLD_DELIBERATION_TIMEOUT_MS,
                ..
            }
        ),
        "超时阈值形态钉住"
    );
    let hold = apeireth_governance::rubric::HoldDecision::held(trigger);
    assert!(hold.is_held(), "超时即 hold");
    step(
        &mut chain,
        "deliberation.timeout",
        "proposal=b7-drill-timeout; elapsed_ms=60000; threshold_ms=60000; hold=true",
        1_000,
    );

    // ② 超时窗口内票未达法定数：不批准（组合规则：hold/超时 ⇒ 不批准）。
    let ha = ha_2of3();
    assert_eq!(
        ha.verify_multisig(&[vote("alice")]),
        MultiSignResult::Insufficient {
            required: 2,
            received: 1
        },
        "超时窗口内 1/2 票，法定数未达"
    );
    step(
        &mut chain,
        "multisig.withheld",
        "proposal=b7-drill-timeout; policy=2-of-3; result=insufficient; ratify=no",
        2_000,
    );

    // ③ 超时闭合映射：expired → Unavailable（fail-closed，不授权）。
    let outcome =
        ApprovalOutcome::from_resolution_label("expired").expect("expired 是既有审批状态机标签");
    assert_eq!(
        outcome,
        ApprovalOutcome::Unavailable,
        "超时收束为 Unavailable"
    );
    assert!(outcome.fail_closed(), "超时即 fail-closed");
    assert!(
        !may_resolve_approval(UserRole::Guest, "guest-carol", Some("owner-alice")),
        "越权收尾同样被拒（Guest 无 ApprovalL2）"
    );
    let pair = ApprovalAuditPair::new("pair-b7-timeout", 1, "core.upgrade:L0", outcome);
    commit_approval_audit_pair(&mut sink, &pair).expect("内存 sink 成对提交必成");
    let scan = scan_approval_pairs(sink.records());
    assert!(
        scan.unclosed.is_empty() && scan.rejected.is_empty(),
        "超时路径 asked↔decision 仍成对闭合（无悬挂 ask）"
    );
    step(
        &mut chain,
        "approval.decision",
        "proposal=b7-drill-timeout; resolver=none; outcome=unavailable",
        3_000,
    );

    // ④ 落盘校验 + 篡改检出。
    chain.verify().expect("链校验必须过");
    assert_eq!(
        chain.len(),
        3,
        "超时路径 3 步审计记录齐（谁/何时/什么决定）"
    );
    persist_verify_and_tamper_check(&chain, "timeout");
}
