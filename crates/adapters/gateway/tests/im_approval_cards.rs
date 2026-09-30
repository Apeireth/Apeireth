//! 件三 (审批卡片闭合) 集成测试: 卡片生成 / 按钮回调 / 四态闭合 /
//! 审计配对原子 / 超时语义 / 一次性执行 / 未配置零回归。
//! 外部端点一律本地 mock, 0 真实网络。

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::clock::{Clock, VirtualClock};
use apeireth_core::kernel::{SessionId, Timestamp};
use apeireth_gateway::{
    assemble_im_channels, CanonicalChainHandler, ImApprovalAuditCommit, ImApprovalCloser,
    ImApprovalNotice, ImApprovalResolution, ImApprovalResolver, ImBridge, ImChannelConfig,
    ImChannelSpec, ImInboundResult, ImSessionMapStore, MemoryApprovalAudit,
};
use apeireth_governance::approval_closure::{
    detect_unclosed_approval_pairs, scan_approval_pairs, ApprovalAuditSlot, ApprovalOutcome,
};
use apeireth_runtime::canonical::{Runtime, TurnOutcome};
use apeireth_sdk::im::{
    render_approval_card, ImButtonPayload, ImChannelKind, ImHttpSender, ImReconnectPolicy,
    ImSegmentPolicy, IM_BUTTON_APPROVE, IM_BUTTON_CANCEL, IM_BUTTON_REJECT,
};
use async_trait::async_trait;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use common::{
    approval_policy, build_runtime, fixed_session, test_clock, wecom_button_event,
    wecom_text_event, FakeProvider, CONVERSATION, FINAL_TEXT, TOOL,
};

// ---------------------------------------------------------------------------
// 夹具
// ---------------------------------------------------------------------------

async fn mock_endpoint() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "message_id": "m-out"
        })))
        .mount(&server)
        .await;
    server
}

async fn posted_bodies(server: &MockServer) -> Vec<String> {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|request| String::from_utf8_lossy(&request.body).to_string())
        .collect()
}

struct ApprovalHarness {
    server: MockServer,
    bridge: ImBridge,
    provider: Arc<FakeProvider>,
    tool_calls: Arc<AtomicUsize>,
    audit: Arc<MemoryApprovalAudit>,
    runtime: Arc<Runtime>,
}

impl ApprovalHarness {
    /// 出站到 mock IM 端点的全部 body。
    async fn posted(&self) -> Vec<String> {
        posted_bodies(&self.server).await
    }
}

/// 一座「需审批」的桥: 真实 canonical 运行时 + 真实对话链 + mock IM 端点。
///
/// `runtime_clock` 与 `bridge_clock` 可分开推进 (本地超时与卡片超时可独立验证)。
async fn approval_harness(
    runtime_clock: Arc<VirtualClock>,
    bridge_clock: Arc<VirtualClock>,
    ttl_ms: u64,
) -> ApprovalHarness {
    let server = mock_endpoint().await;
    let provider = FakeProvider::new();
    let tool_calls = Arc::new(AtomicUsize::new(0));
    let mut policy = apeireth_governance::PermissionPolicy::new();
    policy.grant(apeireth_governance::Permission::ExecuteTool(
        common::CAPABILITY.into(),
    ));
    policy.require_approval_for(common::CAPABILITY);
    let runtime = Arc::new(
        Runtime::builder()
            .with_clock(runtime_clock as Arc<dyn Clock>)
            .with_governance(Arc::new(
                apeireth_governance::GovernancePipeline::new().with(Arc::new(
                    apeireth_governance::PermissionGovernanceHook::new(policy),
                )),
            ))
            .with_plugin(common::TestPlugin::provider(provider.clone()))
            .with_plugin(common::TestPlugin::calculator(tool_calls.clone()))
            .with_default_model(common::MODEL)
            .with_approval_ttl(ttl_ms)
            .build()
            .await
            .unwrap(),
    );
    let handler = Arc::new(CanonicalChainHandler::new(runtime.clone()));
    let audit = Arc::new(MemoryApprovalAudit::new());
    let approvals = Arc::new(ImApprovalCloser::new(
        handler.clone() as Arc<dyn ImApprovalResolver>,
        audit.clone() as Arc<dyn apeireth_gateway::ImApprovalAuditCommit>,
    ));
    let spec = ImChannelSpec {
        id: "primary".to_string(),
        kind: ImChannelKind::ImWecom,
        webhook_or_endpoint: server.uri(),
        secret: None,
        enabled: true,
    };
    let assembly = assemble_im_channels(
        &ImChannelConfig::new(vec![spec]),
        ImReconnectPolicy::default(),
    )
    .unwrap();
    let bridge = ImBridge::new(
        &assembly,
        Arc::new(ImHttpSender::new(ImReconnectPolicy {
            max_attempts: 2,
            backoff_base_ms: 1,
            backoff_cap_ms: 2,
        })),
        Arc::new(Mutex::new(ImSessionMapStore::in_memory())),
        handler,
        approvals,
    )
    .with_clock(bridge_clock)
    .with_segment_policy(ImSegmentPolicy::default());
    ApprovalHarness {
        server,
        bridge,
        provider,
        tool_calls,
        audit,
        runtime,
    }
}

/// 默认夹具: 同一固定时钟 (运行时与桥同口径), 5 分钟审批 TTL。
async fn default_harness(clock: Arc<VirtualClock>) -> ApprovalHarness {
    approval_harness(clock.clone(), clock, 300_000).await
}

/// 从出站卡片体里捡出按钮载荷 (平台按钮往返值)。
fn button_payload(posted_card_body: &str, action: &str) -> serde_json::Value {
    let value: serde_json::Value = serde_json::from_str(posted_card_body).unwrap();
    let mut found = None;
    collect_payloads(&value, &mut |candidate| {
        if candidate.get("pair_id").is_some()
            && candidate.get("approval_ref").is_some()
            && candidate.get("action").and_then(|a| a.as_str()) == Some(action)
        {
            found = Some(candidate.clone());
        }
    });
    found.unwrap_or_else(|| panic!("no {action} button payload in {posted_card_body}"))
}

fn collect_payloads(value: &serde_json::Value, visit: &mut impl FnMut(&serde_json::Value)) {
    visit(value);
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                collect_payloads(item, visit);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values() {
                collect_payloads(item, visit);
            }
        }
        _ => {}
    }
}

/// 脚本闭合器: 分辨率可编程, 记录调用次数。
struct ScriptedResolver {
    mode: Mutex<String>,
    calls: AtomicUsize,
}

impl ScriptedResolver {
    fn new(mode: &str) -> Arc<Self> {
        Arc::new(Self {
            mode: Mutex::new(mode.to_string()),
            calls: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl ImApprovalResolver for ScriptedResolver {
    async fn resolve(
        &self,
        _session: SessionId,
        _approval_ref: &str,
        _decision: &str,
    ) -> ImApprovalResolution {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mode = self.mode.lock().unwrap().clone();
        match mode.as_str() {
            "approved" => ImApprovalResolution::Resumed {
                text: "done".to_string(),
                label: "approved".to_string(),
            },
            "rejected" => ImApprovalResolution::Resumed {
                text: String::new(),
                label: "rejected".to_string(),
            },
            "cancelled" => ImApprovalResolution::Resumed {
                text: String::new(),
                label: "cancelled".to_string(),
            },
            "expired" => ImApprovalResolution::Expired,
            "interrupted" => ImApprovalResolution::Interrupted,
            "not_found" => ImApprovalResolution::NotFound,
            "failed" => ImApprovalResolution::Failed {
                reason: "boom".to_string(),
            },
            other => panic!("unknown scripted mode {other}"),
        }
    }
}

fn button(action: &str, expires_at_ms: i64) -> ImButtonPayload {
    ImButtonPayload {
        approval_ref: "apr_scripted".to_string(),
        pair_id: format!("pair-{action}"),
        round: 7,
        subject: "tool.calculator".to_string(),
        expires_at_ms,
        action: action.to_string(),
    }
}

fn far_future() -> i64 {
    4_000_000_000_000
}

// ---------------------------------------------------------------------------
// 卡片生成
// ---------------------------------------------------------------------------

/// 1. 审批卡片带命令文本 / 风险级 / 批准·拒绝两个按钮 (三个 kind 同一载荷)。
#[tokio::test]
async fn approval_card_carries_command_text_risk_level_and_two_buttons() {
    let clock = test_clock();
    let harness = default_harness(clock.clone()).await;

    let result = harness
        .bridge
        .handle_inbound(
            "primary",
            &wecom_text_event(CONVERSATION, "calculate 1 + 1"),
            None,
        )
        .await
        .unwrap();
    let (pair_id, risk_level) = match result {
        ImInboundResult::ApprovalRequested {
            pair_id,
            risk_level,
            session,
        } => {
            assert_eq!(session, harness.bridge.session_entries()[0].session);
            (pair_id, risk_level)
        }
        other => panic!("expected a pending approval, got {other:?}"),
    };
    assert_eq!(risk_level, "high", "无显式风险标签时基线是 high");

    let bodies = harness.posted().await;
    let body = bodies.last().unwrap();
    assert!(body.contains(TOOL), "{body}");
    assert!(body.contains("approve"), "{body}");
    assert!(body.contains("reject"), "{body}");
    assert!(body.contains(&pair_id), "{body}");
    assert!(body.contains("high"), "{body}");

    let approve = button_payload(body, IM_BUTTON_APPROVE);
    let reject = button_payload(body, IM_BUTTON_REJECT);
    assert_eq!(approve["pair_id"], pair_id);
    assert_eq!(reject["pair_id"], pair_id);
    assert_eq!(approve["subject"], format!("tool.{TOOL}"));
    let parsed: ImButtonPayload = serde_json::from_value(approve).unwrap();
    parsed.validate().unwrap();

    // 三个 kind 的卡片渲染都带同一闭合载荷。
    let notice = ImApprovalNotice {
        approval_ref: "apr_1".to_string(),
        session: fixed_session(),
        pair_id: pair_id.clone(),
        round: 1,
        subject: format!("tool.{TOOL}"),
        command_text: format!("{TOOL}: 1 + 1"),
        arguments_summary: "执行命令 1 + 1".to_string(),
        governance_hook: "permission".to_string(),
        governance_reason: "requires human approval".to_string(),
        created_at_ms: 1_700_000_000_000,
        expires_at_ms: far_future(),
    };
    for kind in ImChannelKind::ALL {
        let card = notice.approval_card();
        let rendered = render_approval_card(kind, &card).unwrap().to_string();
        assert!(rendered.contains(&pair_id), "{kind}: {rendered}");
        assert!(rendered.contains("approve"), "{kind}: {rendered}");
        assert!(rendered.contains("reject"), "{kind}: {rendered}");
    }
}

/// 2. 批准按钮 → `allowed_once` 闭合 + 冻结工具恰好执行一次 + 审计配对原子;
///    重复点击不二次执行、不二次记账。
#[tokio::test]
async fn approve_button_closes_allowed_once_and_runs_the_frozen_tool_once() {
    let clock = test_clock();
    let harness = default_harness(clock.clone()).await;

    harness
        .bridge
        .handle_inbound(
            "primary",
            &wecom_text_event(CONVERSATION, "calculate 1 + 1"),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        harness.tool_calls.load(Ordering::SeqCst),
        0,
        "批准前 0 执行"
    );

    let body = harness.posted().await.last().unwrap().clone();
    let payload = button_payload(&body, IM_BUTTON_APPROVE);
    let result = harness
        .bridge
        .handle_inbound("primary", &wecom_button_event(CONVERSATION, &payload), None)
        .await
        .unwrap();
    let closure = match result {
        ImInboundResult::Closed { closure, .. } => closure,
        other => panic!("expected a closure, got {other:?}"),
    };
    assert_eq!(closure.outcome, ApprovalOutcome::AllowedOnce);
    assert!(closure.executed);
    assert_eq!(closure.resolution_label, "approved");
    assert!(closure.reply_text.contains(FINAL_TEXT), "{:?}", closure);
    assert_eq!(harness.tool_calls.load(Ordering::SeqCst), 1, "恰好一次");

    // 审计配对原子: asked ↔ decision 同一 pair/round/subject。
    let records = harness.audit.committed();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].slot(), ApprovalAuditSlot::Asked);
    assert_eq!(records[1].slot(), ApprovalAuditSlot::Decision);
    assert_eq!(records[0].pair_id(), records[1].pair_id());
    assert_eq!(records[0].round(), records[1].round());
    assert_eq!(records[0].subject(), records[1].subject());
    assert_eq!(records[1].outcome(), Some(ApprovalOutcome::AllowedOnce));
    assert!(detect_unclosed_approval_pairs(&records).is_empty());
    assert!(scan_approval_pairs(&records).unclosed.is_empty());

    // 重复点击: 已闭合轮次 0 二次执行、0 二次记账。
    let again = harness
        .bridge
        .handle_inbound("primary", &wecom_button_event(CONVERSATION, &payload), None)
        .await
        .unwrap();
    assert!(
        matches!(again, ImInboundResult::Ignored { .. }),
        "{again:?}"
    );
    assert_eq!(harness.tool_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.audit.committed().len(), 2);
}

/// 3. 拒绝按钮 → `rejected` 闭合 fail-closed (工具 0 执行), 配对仍原子。
#[tokio::test]
async fn reject_button_fails_closed_with_rejected_outcome() {
    let clock = test_clock();
    let harness = default_harness(clock.clone()).await;

    harness
        .bridge
        .handle_inbound(
            "primary",
            &wecom_text_event(CONVERSATION, "calculate 1 + 1"),
            None,
        )
        .await
        .unwrap();
    let body = harness.posted().await.last().unwrap().clone();
    let payload = button_payload(&body, IM_BUTTON_REJECT);
    let result = harness
        .bridge
        .handle_inbound("primary", &wecom_button_event(CONVERSATION, &payload), None)
        .await
        .unwrap();
    let closure = match result {
        ImInboundResult::Closed { closure, .. } => closure,
        other => panic!("expected a closure, got {other:?}"),
    };
    assert_eq!(closure.outcome, ApprovalOutcome::Rejected);
    assert!(!closure.executed);
    assert_eq!(
        harness.tool_calls.load(Ordering::SeqCst),
        0,
        "拒绝 = 不执行"
    );

    let records = harness.audit.committed();
    assert_eq!(records.len(), 2);
    assert_eq!(records[1].outcome(), Some(ApprovalOutcome::Rejected));
    assert!(detect_unclosed_approval_pairs(&records).is_empty());
}

// ---------------------------------------------------------------------------
// 四态闭合 + 超时语义
// ---------------------------------------------------------------------------

/// 4. 四态词表全可达: 批准/拒绝/取消/超时 各自映射到 allowed_once /
///    rejected / cancelled / unavailable, 且只有 allowed_once 执行。
#[tokio::test]
async fn closure_vocabulary_maps_every_button_onto_four_states() {
    let now = Timestamp::now().epoch_millis();
    for (mode, action, expected, executed) in [
        (
            "approved",
            IM_BUTTON_APPROVE,
            ApprovalOutcome::AllowedOnce,
            true,
        ),
        (
            "rejected",
            IM_BUTTON_REJECT,
            ApprovalOutcome::Rejected,
            false,
        ),
        (
            "cancelled",
            IM_BUTTON_CANCEL,
            ApprovalOutcome::Cancelled,
            false,
        ),
        (
            "expired",
            IM_BUTTON_APPROVE,
            ApprovalOutcome::Unavailable,
            false,
        ),
        (
            "interrupted",
            IM_BUTTON_APPROVE,
            ApprovalOutcome::Unavailable,
            false,
        ),
        (
            "not_found",
            IM_BUTTON_APPROVE,
            ApprovalOutcome::Unavailable,
            false,
        ),
        (
            "failed",
            IM_BUTTON_APPROVE,
            ApprovalOutcome::Unavailable,
            false,
        ),
    ] {
        let resolver = ScriptedResolver::new(mode);
        let audit = Arc::new(MemoryApprovalAudit::new());
        let closer = ImApprovalCloser::new(resolver.clone(), audit.clone());
        let payload = button(action, far_future());
        let result = closer
            .close(apeireth_gateway::ImApprovalRequest {
                session: fixed_session(),
                payload: payload.clone(),
                now_ms: now,
            })
            .await
            .unwrap();
        let closure = match result {
            apeireth_gateway::ImApprovalClosureResult::Closed(closure) => closure,
            other => panic!("{mode}: expected a closure, got {other:?}"),
        };
        assert_eq!(closure.outcome, expected, "{mode}");
        assert_eq!(closure.executed, executed, "{mode}");
        let records = audit.committed();
        assert_eq!(records.len(), 2, "{mode} 配对必须成对落地");
        assert_eq!(records[1].outcome(), Some(expected), "{mode}");
        assert!(
            detect_unclosed_approval_pairs(&records).is_empty(),
            "{mode}"
        );
        assert_eq!(resolver.calls.load(Ordering::SeqCst), 1, "{mode}");
    }
}

/// 5. 超时语义与本地一致: 到点按钮即 `unavailable` 闭合, 0 执行、0 调解析器。
#[tokio::test]
async fn expired_cards_close_unavailable_without_execution() {
    let resolver = ScriptedResolver::new("approved");
    let audit = Arc::new(MemoryApprovalAudit::new());
    let closer = ImApprovalCloser::new(resolver.clone(), audit.clone());

    let created = Timestamp::now().epoch_millis();
    let payload = button(IM_BUTTON_APPROVE, created + 1_000);
    let result = closer
        .close(apeireth_gateway::ImApprovalRequest {
            session: fixed_session(),
            payload: payload.clone(),
            now_ms: created + 60_000,
        })
        .await
        .unwrap();
    let closure = match result {
        apeireth_gateway::ImApprovalClosureResult::Closed(closure) => closure,
        other => panic!("expected a closure, got {other:?}"),
    };
    assert_eq!(closure.outcome, ApprovalOutcome::Unavailable);
    assert!(!closure.executed);
    assert_eq!(closure.resolution_label, "expired");
    assert_eq!(resolver.calls.load(Ordering::SeqCst), 0, "超时不进解析器");
    let records = audit.committed();
    assert_eq!(records.len(), 2);
    assert_eq!(records[1].outcome(), Some(ApprovalOutcome::Unavailable));
}

/// 6. 本地超时 → IM 侧同一四态: canonical `Expired` 分辨率映射 `unavailable`。
#[tokio::test]
async fn local_expiry_maps_to_the_same_unavailable_closure() {
    let runtime_clock = test_clock();
    let bridge_clock = test_clock();
    // 运行时 TTL 1ms, 桥时钟停在卡片创建时刻: 本地先超时, 按钮仍在卡片有效期内。
    let harness = approval_harness(runtime_clock.clone(), bridge_clock, 1).await;
    harness
        .bridge
        .handle_inbound(
            "primary",
            &wecom_text_event(CONVERSATION, "calculate 1 + 1"),
            None,
        )
        .await
        .unwrap();
    let body = harness.posted().await.last().unwrap().clone();
    let payload = button_payload(&body, IM_BUTTON_APPROVE);

    // 运行时侧时钟快进 → 本地超时 (桥时钟不动, 逼出解析器里的 `Expired`)。
    runtime_clock.set(
        Timestamp::from_epoch_millis(1_700_000_000_010)
            .unwrap()
            .as_datetime(),
    );
    let result = harness
        .bridge
        .handle_inbound("primary", &wecom_button_event(CONVERSATION, &payload), None)
        .await
        .unwrap();
    let closure = match result {
        ImInboundResult::Closed { closure, .. } => closure,
        other => panic!("expected a closure, got {other:?}"),
    };
    assert_eq!(closure.outcome, ApprovalOutcome::Unavailable);
    assert_eq!(closure.resolution_label, "expired");
    assert!(!closure.executed);
    assert_eq!(harness.tool_calls.load(Ordering::SeqCst), 0);
    let records = harness.audit.committed();
    assert_eq!(records[1].outcome(), Some(ApprovalOutcome::Unavailable));
}

// ---------------------------------------------------------------------------
// 未配置零回归
// ---------------------------------------------------------------------------

/// 7. 未配置 IM: 本地审批链路原样 (批准执行一次 / 拒绝不执行 / 会话可续),
///    IM 面 inert (装配无渠道)。
#[tokio::test]
async fn unconfigured_im_channels_leave_the_local_approval_flow_untouched() {
    let assembly =
        assemble_im_channels(&ImChannelConfig::default(), ImReconnectPolicy::default()).unwrap();
    assert!(!assembly.is_active(), "无渠道 = IM 面不挂载");

    let clock = test_clock();
    let provider = FakeProvider::new();
    let tool_calls = Arc::new(AtomicUsize::new(0));
    let (runtime, _store) = build_runtime(
        clock as Arc<dyn Clock>,
        Arc::new(approval_policy()),
        provider.clone(),
        tool_calls.clone(),
    )
    .await;

    // 本地既有对话链: 暂停 → 批准 → 恰好执行一次。
    let session = fixed_session();
    let outcome = apeireth_gateway::execute_chat(
        runtime.as_ref(),
        apeireth_gateway::CanonicalChatRequest {
            session: Some(session),
            input: "calculate 1 + 1".to_string(),
            model: None,
            system: None,
        },
    )
    .await
    .unwrap();
    let approval = match outcome {
        apeireth_gateway::CanonicalChatOutcome::PendingApproval(view) => {
            assert_eq!(view.tool_name, TOOL);
            view.approval_id
        }
        other => panic!("expected a pending approval, got {other:?}"),
    };
    assert_eq!(tool_calls.load(Ordering::SeqCst), 0);

    let resolution = runtime
        .resolve_approval(
            session,
            approval,
            apeireth_runtime::canonical::ApprovalDecision::Approve,
        )
        .await
        .unwrap();
    match resolution {
        apeireth_runtime::canonical::ApprovalResolution::Resumed(TurnOutcome::Completed(
            response,
        )) => assert_eq!(response.text, FINAL_TEXT),
        other => panic!("expected a resumed turn, got {other:?}"),
    }
    assert_eq!(tool_calls.load(Ordering::SeqCst), 1, "本地批准语义不变");

    // 同一会话继续对话 (本地会话仍可用)。
    let again = apeireth_gateway::execute_chat(
        runtime.as_ref(),
        apeireth_gateway::CanonicalChatRequest {
            session: Some(session),
            input: "continue".to_string(),
            model: None,
            system: None,
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        again,
        apeireth_gateway::CanonicalChatOutcome::Completed(_)
    ));
}

/// 8. 落盘审计: 闭合轮次成对持久, 重开仍配对完整 (审计配对原子)。
#[tokio::test]
async fn paired_audit_records_persist_and_reopen_as_complete_pairs() {
    let dir = std::env::temp_dir().join(format!("apeireth-im-audit-{}", std::process::id()));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).unwrap();
    let audit =
        apeireth_gateway::FileApprovalAudit::open(dir.join("approval-audit.jsonl")).unwrap();

    let resolver = ScriptedResolver::new("approved");
    let closer = ImApprovalCloser::new(resolver, Arc::new(audit.clone()));
    let now = Timestamp::now().epoch_millis();
    closer
        .close(apeireth_gateway::ImApprovalRequest {
            session: fixed_session(),
            payload: button(IM_BUTTON_APPROVE, far_future()),
            now_ms: now,
        })
        .await
        .unwrap();

    let records = audit.records();
    assert_eq!(records.len(), 2);
    assert!(detect_unclosed_approval_pairs(&records).is_empty());

    let reopened = apeireth_gateway::FileApprovalAudit::open(dir.join("approval-audit.jsonl"))
        .expect("audit reopens");
    let restored = reopened.records();
    assert_eq!(restored, records, "落盘记录重开一致");
    assert!(scan_approval_pairs(&restored).unclosed.is_empty());
    let _ = fs_err::remove_dir_all(&dir);
}

/// 9. 一次性: 同一 pair 的第二次闭合请求不记账 (闭合器层再验一次)。
#[tokio::test]
async fn the_closer_never_commits_a_second_pair_for_one_round() {
    let resolver = ScriptedResolver::new("approved");
    let audit = Arc::new(MemoryApprovalAudit::new());
    let closer = ImApprovalCloser::new(resolver.clone(), audit.clone());
    let payload = button(IM_BUTTON_APPROVE, far_future());
    let now = Timestamp::now().epoch_millis();

    let first = closer
        .close(apeireth_gateway::ImApprovalRequest {
            session: fixed_session(),
            payload: payload.clone(),
            now_ms: now,
        })
        .await
        .unwrap();
    assert!(matches!(
        first,
        apeireth_gateway::ImApprovalClosureResult::Closed(_)
    ));

    let second = closer
        .close(apeireth_gateway::ImApprovalRequest {
            session: fixed_session(),
            payload,
            now_ms: now,
        })
        .await
        .unwrap();
    assert!(matches!(
        second,
        apeireth_gateway::ImApprovalClosureResult::AlreadyClosed { .. }
    ));
    assert_eq!(audit.committed().len(), 2);
    assert_eq!(resolver.calls.load(Ordering::SeqCst), 1);
}
