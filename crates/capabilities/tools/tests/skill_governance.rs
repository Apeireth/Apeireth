//! 治理档回归 (I1): `tool.skill_distill` = 写文件面 (file_write 总闸族 +
//! 审批档), `tool.skill_search` = 只读档零审批。
//!
//! 覆盖: 审批档 (每次沉淀都人批, 无自动放行档, 理由载荷 = 技能卡摘要) /
//! 风险映射 (未授权冻结在 require-approval 档) / 单调边界 Guard (坏卡与来源
//! 越界拒绝即帧) / 只读档 (无 ask 档钩子) / 授权播种语义。

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernanceRequest, Permission, PermissionPolicy,
};
use apeireth_plugin::ToolCapability;
use apeireth_tools_canonical::skill::{SkillCardDraft, SkillLibrary, SkillSource, SkillSourceKind};
use apeireth_tools_canonical::{
    authorized_skill_distill_policy, skill_search_pipeline, PreExecuteHook, PreExecuteRequest,
    PreVerdict, SkillDistillApprovalHook, SkillDistillBoundaryGuard, SkillDistillParams,
    SkillDistillRiskMappingHook, SkillSearchBoundaryGuard, SkillSearchTool, ToolGuard,
    ToolGuardRequest, SKILL_DISTILL_CAPABILITY_ID, SKILL_SEARCH_CAPABILITY_ID,
};

fn good_params() -> SkillDistillParams {
    SkillDistillParams {
        card: SkillCardDraft {
            name: "换元法".into(),
            summary: "换元后微分元同步替换".into(),
            triggers: vec!["换元".into()],
            body: "步骤一二三".into(),
            source: SkillSource {
                kind: SkillSourceKind::Book,
                title: "讲义".into(),
            },
        },
        text: Some("来源正文".into()),
        path: None,
        update: false,
    }
}

fn dispatch_request<'a>(
    capability: &'a CapabilityId,
    arguments: &'a serde_json::Value,
) -> GovernanceRequest<'a> {
    GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability,
            arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    )
}

/// 审批档: **每次沉淀都停人工审批** (无自动放行档), 理由载荷 = 技能卡摘要。
#[tokio::test]
async fn every_distill_call_waits_for_a_human_decision_with_the_card_summary() {
    let hook = SkillDistillApprovalHook::new();
    let capability = CapabilityId::new(SKILL_DISTILL_CAPABILITY_ID).unwrap();
    let args = serde_json::to_value(good_params()).unwrap();

    let decision = hook.evaluate(&dispatch_request(&capability, &args)).await;
    let Decision::RequireApproval { reason } = decision else {
        panic!("每次沉淀都必须停人工审批: {decision:?}");
    };
    assert!(
        reason.contains("技能卡沉淀"),
        "理由载荷 = 技能卡摘要: {reason}"
    );
    assert!(reason.contains("换元法"), "{reason}");

    // 其它能力不归本钩子 (授权仍由第一钩子裁决)。
    let other = CapabilityId::new(SKILL_SEARCH_CAPABILITY_ID).unwrap();
    let decision = hook.evaluate(&dispatch_request(&other, &args)).await;
    assert!(matches!(decision, Decision::Allow), "{decision:?}");
}

/// 错误即帧: 坏请求 (坏卡 / 缺来源) 不进审批, 直接拒绝。
#[tokio::test]
async fn bad_distill_requests_are_denied_outside_the_approval_flow() {
    let hook = SkillDistillApprovalHook::new();
    let capability = CapabilityId::new(SKILL_DISTILL_CAPABILITY_ID).unwrap();
    let mut bad = good_params();
    bad.card.triggers = vec!["   ".into()];
    bad.text = None;
    let args = serde_json::to_value(bad).unwrap();

    let decision = hook.evaluate(&dispatch_request(&capability, &args)).await;
    let Decision::Deny { reason } = decision else {
        panic!("坏请求必须拒绝即帧, 不进审批: {decision:?}");
    };
    assert!(reason.starts_with("拒绝"), "{reason}");
}

/// 风险映射: 未显式授权的沉淀调用冻结在 require-approval 档 (不放行、不落盘)。
#[test]
fn risk_mapping_freezes_unauthorized_calls_at_require_approval() {
    let capability = CapabilityId::new(SKILL_DISTILL_CAPABILITY_ID).unwrap();
    let call = apeireth_protocol::canonical::ToolCall {
        id: "call_1".into(),
        name: "skill_distill".into(),
        arguments: serde_json::to_value(good_params()).unwrap(),
    };
    let request = PreExecuteRequest::new(&call, &capability);

    // 无授权: Ask (冻结待批)。
    let empty = Arc::new(std::sync::Mutex::new(PermissionPolicy::new()));
    let verdict = SkillDistillRiskMappingHook::new(empty).pre_verdict(&request);
    assert!(
        matches!(verdict, PreVerdict::Ask { .. }),
        "未授权必须停在 require-approval 档: {verdict:?}"
    );

    // 生产授权播种 (已注册即有人拍板): Allow。
    let verdict =
        SkillDistillRiskMappingHook::new(authorized_skill_distill_policy()).pre_verdict(&request);
    assert!(
        matches!(verdict, PreVerdict::Allow),
        "授权后由审批档钩子承载人批: {verdict:?}"
    );
}

/// 单调边界 Guard: 坏卡 / 来源越界一律拒绝 (只拒不放)。
#[test]
fn boundary_guard_denies_bad_cards_and_source_escapes() {
    let guard = SkillDistillBoundaryGuard::new(".");
    let capability = CapabilityId::new(SKILL_DISTILL_CAPABILITY_ID).unwrap();

    let mut bad = good_params();
    bad.card.name = "  ".into();
    let args = serde_json::to_value(bad).unwrap();
    let denial = guard
        .deny(&ToolGuardRequest::new(&capability, "skill_distill", &args))
        .expect("坏卡必须拒");
    assert!(denial.contains("卡名"), "{denial}");

    let mut escape = good_params();
    escape.text = None;
    escape.path = Some("../outside.md".into());
    let args = serde_json::to_value(escape).unwrap();
    let denial = guard
        .deny(&ToolGuardRequest::new(&capability, "skill_distill", &args))
        .expect("来源越界必须拒");
    assert!(denial.contains("拒绝沉淀"), "{denial}");

    let args = serde_json::to_value(good_params()).unwrap();
    assert!(
        guard
            .deny(&ToolGuardRequest::new(&capability, "skill_distill", &args))
            .is_none(),
        "合法请求不归边界 Guard 拦"
    );
}

/// 只读档: `tool.skill_search` 的流水线没有 ask 档钩子 (零审批), 只有
/// 单调边界 Guard + 超时 + 输出归一合同。
#[test]
fn skill_search_sits_at_the_read_only_tier_without_an_approval_stage() {
    let pipeline = skill_search_pipeline();
    assert!(
        pipeline.pre().is_empty(),
        "只读检索不得有 pre 档钩子: {:?}",
        pipeline.pre().names()
    );
    assert!(
        pipeline
            .guards()
            .names()
            .iter()
            .any(|name| *name == "skill_search_boundary"),
        "单调边界 Guard 在位: {:?}",
        pipeline.guards().names()
    );
    assert!(pipeline.output_schema().is_some(), "输出归一合同在位");

    // 边界 Guard 只拒不放: 空查询拒。
    let guard = SkillSearchBoundaryGuard::new();
    let capability = CapabilityId::new(SKILL_SEARCH_CAPABILITY_ID).unwrap();
    let args = serde_json::json!({ "query": "  " });
    assert!(guard
        .deny(&ToolGuardRequest::new(&capability, "skill_search", &args))
        .unwrap()
        .contains("query"));
}

/// 授权播种语义: `authorized_skill_distill_policy` 只 grant 沉淀工具
/// (授权 = 「这次调用有人拍板」); 审批档不在策略里。
#[test]
fn authorization_seeding_grants_the_distill_tool_only() {
    let policy = authorized_skill_distill_policy();
    let locked = policy.lock().unwrap();
    assert!(
        locked
            .decision_for_capability(SKILL_DISTILL_CAPABILITY_ID)
            .is_allowed(),
        "注册即授权"
    );
    assert!(
        !locked
            .decision_for_capability(SKILL_SEARCH_CAPABILITY_ID)
            .is_allowed(),
        "只读检索的授权面由开关同源播种, 不进写策略"
    );
    drop(locked);

    // 需要审批时策略本身也能标记 (与审批档同链, 双保险不冲突)。
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool(
        SKILL_DISTILL_CAPABILITY_ID.to_string(),
    ));
    policy.require_approval_for(SKILL_DISTILL_CAPABILITY_ID);
    let decision = policy.decision_for_capability(SKILL_DISTILL_CAPABILITY_ID);
    assert!(
        matches!(decision, Decision::RequireApproval { .. }),
        "{decision:?}"
    );
}

/// 输出归一合同与工具声明一致 (模型面看到的形状被冻结)。
#[test]
fn tool_declarations_carry_their_contract() {
    let search = SkillSearchTool::with_default_budget(SkillLibrary::new("."));
    let declaration = search.declaration();
    assert_eq!(declaration.name, "skill_search");
    assert!(
        declaration
            .description
            .as_deref()
            .unwrap()
            .contains("read-only"),
        "只读档要在声明面写明"
    );
}
