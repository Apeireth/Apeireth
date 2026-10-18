//! 自主度提案面接线 (件四) 集成测试: 提案卡出站 (同链同信封) / IM 按钮回调
//! 落账 (`proposal accept/reject`, decided_by = IM 用户 + 渠道标注) / 一提案一
//! 决定 (重复点击拒绝即帧) / 推送幂等 (同 id 单推) / 失败留账 (IM 失败不丢提案,
//! 重试可送达) / 自述提案节推送列。外部端点一律本地 mock, 0 真实网络。

mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::SessionId;
use apeireth_gateway::{
    assemble_im_channels, ImApprovalCloser, ImApprovalNotice, ImApprovalResolution,
    ImApprovalResolver, ImBridge, ImChannelConfig, ImChannelSpec, ImInboundResult, ImProposalFace,
    ImSessionMapStore, ImTurnHandler, ImTurnOutcome, MemoryApprovalAudit,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
use apeireth_sdk::im::{
    ImButtonPayload, ImChannelKind, ImHttpSender, ImReconnectPolicy, ImSegmentPolicy,
    IM_BUTTON_APPROVE, IM_BUTTON_REJECT,
};
use apeireth_tools_canonical::proposal::{
    proposal_capability, ProposalConfig, ProposalDecision, ProposalLedger, ProposalLedgerEntry,
    ProposalState,
};
use apeireth_tools_canonical::proposal_push::{ProposalPushLedger, ProposalPushOutcome};
use async_trait::async_trait;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use common::{fixed_session, test_clock, wecom_button_event, wecom_text_event, CONVERSATION};

// ---------------------------------------------------------------------------
// 夹具
// ---------------------------------------------------------------------------

async fn ok_endpoint() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "message_id": "m-out"
        })))
        .mount(&server)
        .await;
    server
}

async fn failing_endpoint() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
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

/// 无工具审批的解析器 (提案面不走审批四态; 本夹具只测提案链)。
struct NoApprovals;

#[async_trait]
impl ImApprovalResolver for NoApprovals {
    async fn resolve(
        &self,
        _session: SessionId,
        _approval_ref: &str,
        _decision: &str,
    ) -> ImApprovalResolution {
        ImApprovalResolution::NotFound
    }
}

/// 回声对话链 (回合文本可预测, 提案卡推送发生在回合收尾)。
struct EchoHandler;

#[async_trait]
impl ImTurnHandler for EchoHandler {
    async fn handle_text(&self, _session: SessionId, text: &str) -> ImTurnOutcome {
        ImTurnOutcome {
            text: format!("echo: {text}"),
            approval: None,
        }
    }
}

struct ProposalHarness {
    server: MockServer,
    bridge: ImBridge,
    face: Arc<ImProposalFace>,
    tool: Arc<dyn ToolCapability>,
    data_dir: PathBuf,
}

impl ProposalHarness {
    /// 出站到 mock IM 端点的全部 body。
    async fn posted(&self) -> Vec<String> {
        posted_bodies(&self.server).await
    }

    /// 经注册工具面起草一份提案 (提案入账), 返回提案 id。
    async fn draft_one(&self) -> String {
        draft_one(&self.tool).await
    }
}

/// 一座接了提案面的桥: 真实 `tool.proposal_draft` 工具面 + 真实推送账落盘 +
/// mock IM 端点 (同一套出站链与信封)。
async fn proposal_harness(server: MockServer, data_dir: &Path) -> ProposalHarness {
    let tool = proposal_capability(ProposalConfig::new(data_dir));
    let face = Arc::new(ImProposalFace::new(data_dir, tool.clone()));
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
    let approvals = Arc::new(ImApprovalCloser::new(
        Arc::new(NoApprovals),
        Arc::new(MemoryApprovalAudit::new()),
    ));
    let bridge = ImBridge::new(
        &assembly,
        Arc::new(ImHttpSender::new(ImReconnectPolicy {
            max_attempts: 1,
            backoff_base_ms: 1,
            backoff_cap_ms: 2,
        })),
        Arc::new(Mutex::new(ImSessionMapStore::in_memory())),
        Arc::new(EchoHandler),
        approvals,
    )
    .with_clock(test_clock())
    .with_segment_policy(ImSegmentPolicy::default())
    .with_proposal_face(face.clone());
    ProposalHarness {
        server,
        bridge,
        face,
        tool,
        data_dir: data_dir.to_path_buf(),
    }
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("apeireth-im-proposal-{tag}-{}", std::process::id()));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("create test dir");
    dir
}

/// 经注册工具面起草一份提案 (提案入账), 返回提案 id。
async fn draft_one(tool: &Arc<dyn ToolCapability>) -> String {
    let result = tool
        .invoke(&ToolCall {
            id: "call_draft".into(),
            name: "proposal_draft".into(),
            arguments: serde_json::json!({
                "action": "draft",
                "kind": "tidy",
                "title": "边角料: 三处临时注释待清理",
                "detail": "这三行注释是调试时留下的。",
                "evidence": ["src/a.rs:12", "src/b.rs:40"],
                "draft_payload": {"files": ["src/a.rs", "src/b.rs"]},
            }),
        })
        .await;
    assert!(result.is_ok(), "起草必须成功: {}", result.render());
    match result.outcome {
        ToolOutcome::Ok { value } => value["proposal"]["id"]
            .as_str()
            .expect("提案 id 在场")
            .to_string(),
        other => panic!("起草帧必须是 ok: {other:?}"),
    }
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

/// 信封同构断言: 两个出站体的形状 (键集 + 标量类型) 逐层一致 —— 同一个信封。
fn assert_envelopes_isomorphic(approval_body: &str, proposal_body: &str) {
    let approval: serde_json::Value = serde_json::from_str(approval_body).unwrap();
    let proposal: serde_json::Value = serde_json::from_str(proposal_body).unwrap();
    assert_eq!(
        envelope_shape(&approval),
        envelope_shape(&proposal),
        "提案卡出站必须与 IM 审批卡同信封 (逐层键集同构)\n审批体: {approval_body}\n提案体: {proposal_body}"
    );
}

fn envelope_shape(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, item) in map {
                out.insert(key.clone(), envelope_shape(item));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(envelope_shape).collect())
        }
        serde_json::Value::String(_) => serde_json::json!("<string>"),
        serde_json::Value::Number(_) => serde_json::json!("<number>"),
        serde_json::Value::Bool(_) => serde_json::json!("<bool>"),
        serde_json::Value::Null => serde_json::json!("<null>"),
    }
}

fn review_rows(data_dir: &Path) -> Vec<apeireth_tools_canonical::proposal::ProposalReview> {
    ProposalLedger::under_data_dir(data_dir)
        .scan()
        .unwrap()
        .records
        .iter()
        .filter_map(|record| match &record.body {
            ProposalLedgerEntry::Review(review) => Some(review.clone()),
            ProposalLedgerEntry::Draft(_) => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 1-2. 提案卡出站: 同链同信封 + 「建议你亲自确认」行随卡到手机 + 提案入账→推送
// ---------------------------------------------------------------------------

/// 1. 卡出站 (信封同构断言): 提案卡经同一条出站链、同一个 ImApprovalCard 信封
///    到手机; 「建议你亲自确认」行 + 逐条「请亲自过目」+ 温柔语气行随卡;
///    按钮载荷 = 合法闭合身份 (pair_id = 提案 id = 推送幂等键)。
#[tokio::test]
async fn proposal_cards_reach_the_phone_through_the_same_envelope_as_approval_cards() {
    let server = ok_endpoint().await;
    let dir = temp_dir("envelope");
    let harness = proposal_harness(server, &dir).await;
    let id = harness.draft_one().await;

    let pushed = harness
        .bridge
        .push_proposal_cards("primary", CONVERSATION)
        .await
        .unwrap();
    assert_eq!(pushed.len(), 1);
    assert_eq!(pushed[0].proposal_id, id);
    assert_eq!(pushed[0].outcome, ProposalPushOutcome::Pushed);

    let bodies = harness.posted().await;
    assert_eq!(bodies.len(), 1, "恰好一张提案卡出站");
    let proposal_body = bodies[0].clone();
    for line in ["建议你亲自确认", "【请亲自过目】", "你最好亲自看一遍"] {
        assert!(proposal_body.contains(line), "「{line}」必须随卡到手机");
    }
    assert!(proposal_body.contains(&id), "卡面带提案 id");
    assert!(proposal_body.contains("low"), "提案面风险档 = 低");

    // 同信封: 审批卡走同一条链出站 → 两个体逐层键集同构 (0 第二套卡信封)。
    let notice = ImApprovalNotice {
        approval_ref: "apr_1".to_string(),
        session: fixed_session(),
        pair_id: "pair-1".to_string(),
        round: 1,
        subject: "tool.calculator".to_string(),
        command_text: "calculator: 1 + 1".to_string(),
        arguments_summary: "执行命令 1 + 1".to_string(),
        authorization_families: Vec::new(),
        action_binding_digest: String::new(),
        governance_hook: "permission".to_string(),
        governance_reason: "requires human approval".to_string(),
        created_at_ms: 1_700_000_000_000,
        expires_at_ms: 4_000_000_000_000,
    };
    harness
        .bridge
        .notify_approval("primary", CONVERSATION, &notice)
        .await
        .unwrap();
    let all = harness.posted().await;
    assert_envelopes_isomorphic(all.last().unwrap(), &proposal_body);

    // 按钮往返载荷: 两按钮都是合法 ImButtonPayload, 闭合身份 = 提案 id。
    for action in [IM_BUTTON_APPROVE, IM_BUTTON_REJECT] {
        let payload: ImButtonPayload =
            serde_json::from_value(button_payload(&proposal_body, action)).unwrap();
        payload.validate().unwrap();
        assert_eq!(payload.approval_ref, id);
        assert_eq!(payload.pair_id, id, "一提案一决定的闭合身份 = 提案 id");
        assert_eq!(payload.subject, "tool.proposal_draft");
    }
    let _ = fs_err::remove_dir_all(&dir);
}

/// 2. 提案入账 → 推送接线: 回合收尾把落地的提案草稿推到同一 IM 会话
///    (推送账 delivered 留痕)。
#[tokio::test]
async fn a_landed_proposal_rides_the_next_im_turn_to_the_same_conversation() {
    let server = ok_endpoint().await;
    let dir = temp_dir("turn-push");
    let harness = proposal_harness(server, &dir).await;
    let id = harness.draft_one().await;

    let result = harness
        .bridge
        .handle_inbound("primary", &wecom_text_event(CONVERSATION, "在吗"), None)
        .await
        .unwrap();
    assert!(
        matches!(result, ImInboundResult::Replied { .. }),
        "{result:?}"
    );

    let bodies = harness.posted().await;
    assert_eq!(bodies.len(), 2, "回执文本 + 提案卡各一次出站");
    assert!(bodies[0].contains("echo: 在吗"), "{:?}", bodies[0]);
    assert!(bodies[1].contains("建议你亲自确认"), "{:?}", bodies[1]);
    assert!(bodies[1].contains(&id), "{:?}", bodies[1]);

    let pushes = ProposalPushLedger::under_data_dir(&dir).scan().unwrap();
    assert_eq!(pushes.records.len(), 1, "推送账留痕");
    assert_eq!(pushes.records[0].body.outcome, ProposalPushOutcome::Pushed);
    assert_eq!(pushes.records[0].body.proposal_id, id);
    assert_eq!(pushes.records[0].body.channel_id, "primary");
    assert_eq!(
        pushes.records[0].body.message_ref.as_deref(),
        Some("m-out"),
        "对端回执如实入账"
    );
    let _ = fs_err::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// 3-5. 推送幂等 / 按钮落账署名 / 一提案一决定
// ---------------------------------------------------------------------------

/// 3. 推送幂等 (同 id 单推): 推送幂等键 = 提案 id, 重复推送 0 二次出站,
///    自述推送列照实报 1。
#[tokio::test]
async fn the_same_proposal_is_never_pushed_twice() {
    let server = ok_endpoint().await;
    let dir = temp_dir("idempotent");
    let harness = proposal_harness(server, &dir).await;
    let id = harness.draft_one().await;

    let first = harness
        .bridge
        .push_proposal_cards("primary", CONVERSATION)
        .await
        .unwrap();
    assert_eq!(first.len(), 1);
    let second = harness
        .bridge
        .push_proposal_cards("primary", CONVERSATION)
        .await
        .unwrap();
    assert!(second.is_empty(), "幂等: 已推送的不再推");
    // 回合触发的自动推送同样被幂等键挡住。
    harness
        .bridge
        .handle_inbound("primary", &wecom_text_event(CONVERSATION, "再看一眼"), None)
        .await
        .unwrap();

    let bodies = harness.posted().await;
    let card_bodies: Vec<&String> = bodies
        .iter()
        .filter(|body| body.contains("建议你亲自确认"))
        .collect();
    assert_eq!(card_bodies.len(), 1, "同 id 单推: 恰好一张提案卡出站");

    let pushes = ProposalPushLedger::under_data_dir(&dir);
    assert_eq!(pushes.pushed_ids().unwrap().len(), 1);
    let status = ProposalLedger::under_data_dir(&dir).status().unwrap();
    assert_eq!(status.pushed, 1, "自述推送列 = 1");
    assert_eq!(id, status.recent_pushes[0].proposal_id);
    assert_eq!(status.recent_pushes[0].outcome, "pushed");
    let _ = fs_err::remove_dir_all(&dir);
}

/// 4. 按钮回调落账 (decided_by 署名): 批准按钮 → `proposal accept` 落账,
///    decided_by = IM 用户名 + 渠道标注; 点头只是账面标记 (恒不执行)。
#[tokio::test]
async fn approve_click_lands_the_decision_signed_by_the_im_user_and_channel() {
    let server = ok_endpoint().await;
    let dir = temp_dir("approve");
    let harness = proposal_harness(server, &dir).await;
    let id = harness.draft_one().await;
    harness
        .bridge
        .push_proposal_cards("primary", CONVERSATION)
        .await
        .unwrap();

    let body = harness.posted().await.last().unwrap().clone();
    let payload = button_payload(&body, IM_BUTTON_APPROVE);
    let result = harness
        .bridge
        .handle_inbound("primary", &wecom_button_event(CONVERSATION, &payload), None)
        .await
        .unwrap();
    let closure = match result {
        ImInboundResult::ProposalDecided {
            closure,
            reply_segments,
        } => {
            assert_eq!(reply_segments, 1);
            closure
        }
        other => panic!("expected a proposal decision, got {other:?}"),
    };
    assert_eq!(closure.proposal_id, id);
    assert_eq!(closure.decision, ProposalDecision::Accepted);
    assert_eq!(
        closure.decided_by, "im:u_demo@primary",
        "decided_by = IM 用户名 + 渠道标注"
    );
    assert!(!closure.executed, "点头只是账面标记, 恒不执行");
    assert!(
        closure.reply_text.contains("im:u_demo@primary"),
        "回执带署名"
    );

    // 落账真值: 提案账决定行 (与工具面同一本账)。
    let rows = review_rows(&dir);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].proposal_id, id);
    assert_eq!(rows[0].decision, ProposalDecision::Accepted);
    assert_eq!(rows[0].decided_by, "im:u_demo@primary");
    assert_eq!(
        ProposalLedger::under_data_dir(&dir).state_of(&id).unwrap(),
        Some(ProposalState::Accepted)
    );
    let _ = fs_err::remove_dir_all(&dir);
}

/// 5. 一提案一决定 (重复点击拒绝即帧): 第一次点击落账, 重复点击走拒绝即帧
///    (真因如实: 一提案一决定, append-only 不改史), 账面恰好一行决定。
#[tokio::test]
async fn one_proposal_gets_exactly_one_decision_and_the_second_click_is_refused() {
    let server = ok_endpoint().await;
    let dir = temp_dir("one-decision");
    let harness = proposal_harness(server, &dir).await;
    let id = harness.draft_one().await;
    harness
        .bridge
        .push_proposal_cards("primary", CONVERSATION)
        .await
        .unwrap();
    let body = harness.posted().await.last().unwrap().clone();

    let reject = button_payload(&body, IM_BUTTON_REJECT);
    let first = harness
        .bridge
        .handle_inbound("primary", &wecom_button_event(CONVERSATION, &reject), None)
        .await
        .unwrap();
    match first {
        ImInboundResult::ProposalDecided { closure, .. } => {
            assert_eq!(closure.decision, ProposalDecision::Rejected);
            assert!(!closure.executed);
        }
        other => panic!("expected a proposal decision, got {other:?}"),
    }

    // 重复点击 (这次点批准) → 拒绝即帧, 真因如实, 不翻案。
    let approve = button_payload(&body, IM_BUTTON_APPROVE);
    let again = harness
        .bridge
        .handle_inbound("primary", &wecom_button_event(CONVERSATION, &approve), None)
        .await
        .unwrap();
    let reason = match again {
        ImInboundResult::ProposalRefused {
            reason,
            reply_segments,
        } => {
            assert_eq!(reply_segments, 1, "拒绝即帧也回执到会话");
            reason
        }
        other => panic!("expected a refusal frame, got {other:?}"),
    };
    assert!(reason.contains("一提案一决定"), "{reason}");
    assert!(reason.contains(&id), "{reason}");

    let rows = review_rows(&dir);
    assert_eq!(rows.len(), 1, "一提案一决定: 账面恰好一行决定");
    assert_eq!(rows[0].decision, ProposalDecision::Rejected);
    assert_eq!(
        ProposalLedger::under_data_dir(&dir).state_of(&id).unwrap(),
        Some(ProposalState::Rejected),
        "重复点击不改史"
    );
    let _ = fs_err::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// 6-7. 失败留账 (IM 失败不丢提案) / 自述推送列
// ---------------------------------------------------------------------------

/// 6. 失败留账: 推送失败 → `failed` 行 + 真因入推送账, 提案本体不丢 (仍在
///    提案账待审); 换座桥 (重试) 成功送达后幂等键生效, 不再重推。
#[tokio::test]
async fn a_failed_push_stays_on_the_books_and_the_proposal_is_never_lost() {
    let failing = failing_endpoint().await;
    let dir = temp_dir("failure");
    let harness = proposal_harness(failing, &dir).await;
    let id = harness.draft_one().await;

    let results = harness
        .bridge
        .push_proposal_cards("primary", CONVERSATION)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].outcome, ProposalPushOutcome::Failed);
    assert!(
        results[0].detail.contains("delivery failed"),
        "{}",
        results[0].detail
    );

    // 失败留账 (真因) + 提案不丢 (提案账仍在, 待审)。
    let pushes = ProposalPushLedger::under_data_dir(&dir).scan().unwrap();
    assert_eq!(pushes.records.len(), 1);
    assert_eq!(pushes.records[0].body.outcome, ProposalPushOutcome::Failed);
    assert_eq!(
        pushes.records[0]
            .body
            .reason
            .as_deref()
            .unwrap()
            .contains("delivery failed"),
        true
    );
    assert!(
        ProposalPushLedger::under_data_dir(&dir)
            .pushed_ids()
            .unwrap()
            .is_empty(),
        "失败不算已推送 (要重试)"
    );
    assert_eq!(
        ProposalLedger::under_data_dir(&dir).state_of(&id).unwrap(),
        Some(ProposalState::Pending),
        "IM 失败不丢提案"
    );

    // 重试 (换座桥 = 重启后同账重试): 成功送达 → 幂等键生效。
    let recovered = ok_endpoint().await;
    let retry_harness = proposal_harness(recovered, &dir).await;
    let retried = retry_harness
        .bridge
        .push_proposal_cards("primary", CONVERSATION)
        .await
        .unwrap();
    assert_eq!(retried.len(), 1);
    assert_eq!(retried[0].outcome, ProposalPushOutcome::Pushed);
    let after = retry_harness
        .bridge
        .push_proposal_cards("primary", CONVERSATION)
        .await
        .unwrap();
    assert!(after.is_empty(), "重试成功后不再重推 (幂等)");

    let pushes = ProposalPushLedger::under_data_dir(&dir).scan().unwrap();
    assert_eq!(pushes.records.len(), 2, "失败留账 + 成功留痕, 两行都在");
    let status = ProposalLedger::under_data_dir(&dir).status().unwrap();
    assert_eq!(status.pushed, 1);
    let _ = fs_err::remove_dir_all(&dir);
}

/// 7. self_status 提案节扩: 已推送数 / 最近推送与推送账同源 (成功 + 失败照实)。
#[tokio::test]
async fn self_status_reports_pushed_count_and_recent_pushes() {
    let server = ok_endpoint().await;
    let dir = temp_dir("status");
    let harness = proposal_harness(server, &dir).await;
    let pushed_id = harness.draft_one().await;
    harness
        .bridge
        .push_proposal_cards("primary", CONVERSATION)
        .await
        .unwrap();

    // 第二份提案走失败通道 (推送失败留账)。
    let failing = failing_endpoint().await;
    let fail_harness = proposal_harness(failing, &dir).await;
    let failed_id = fail_harness.draft_one().await;
    fail_harness
        .bridge
        .push_proposal_cards("primary", CONVERSATION)
        .await
        .unwrap();

    let status = ProposalLedger::under_data_dir(&dir).status().unwrap();
    assert_eq!(status.pushed, 1, "已推送数只计成功送达");
    assert_eq!(status.recent_pushes.len(), 2, "最近推送含失败留账行 (如实)");
    assert_eq!(status.recent_pushes[0].proposal_id, failed_id, "新在前");
    assert_eq!(status.recent_pushes[0].outcome, "failed");
    assert_eq!(status.recent_pushes[1].proposal_id, pushed_id);
    assert_eq!(status.recent_pushes[1].outcome, "pushed");
    assert_eq!(status.recent_pushes[0].channel_id, "primary");
    assert_eq!(
        status.pending, 2,
        "两份提案都还在待审 (推送状态不改提案状态)"
    );
    let _ = fs_err::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// 8-9. 诚实署名 (缺人不落账) / 未接线零回归
// ---------------------------------------------------------------------------

/// 8. 按钮回调缺 IM 用户标识 = 拒绝即帧 (decided_by 必须如实署名), 账面零写。
#[tokio::test]
async fn a_click_without_an_im_operator_is_refused_with_a_real_frame() {
    let server = ok_endpoint().await;
    let dir = temp_dir("unsigned");
    let harness = proposal_harness(server, &dir).await;
    let _id = harness.draft_one().await;
    harness
        .bridge
        .push_proposal_cards("primary", CONVERSATION)
        .await
        .unwrap();
    let body = harness.posted().await.last().unwrap().clone();
    let payload = button_payload(&body, IM_BUTTON_APPROVE);

    let event = serde_json::json!({
        "action": "callback",
        "action_value": payload.to_string(),
        "chatid": CONVERSATION,
        "msgid": "msg_button",
        "operator": "",
    })
    .to_string();
    let result = harness
        .bridge
        .handle_inbound("primary", &event, None)
        .await
        .unwrap();
    let reason = match result {
        ImInboundResult::ProposalRefused { reason, .. } => reason,
        other => panic!("expected a refusal frame, got {other:?}"),
    };
    assert!(reason.contains("缺 IM 用户标识"), "{reason}");
    assert!(review_rows(&dir).is_empty(), "缺署名 = 账面零写");
    let _ = fs_err::remove_dir_all(&dir);
}

/// 9. 未接提案面 = 照实报「未接线」+ 出站 inert (0 假旋钮, 既有链零回归)。
#[tokio::test]
async fn without_the_proposal_face_the_bridge_reports_unwired_honestly() {
    let server = ok_endpoint().await;
    let dir = temp_dir("unwired");
    let tool = proposal_capability(ProposalConfig::new(&dir));
    let _id = draft_one(&tool).await;

    // 不带提案面的桥 (既有链原样)。
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
            max_attempts: 1,
            backoff_base_ms: 1,
            backoff_cap_ms: 2,
        })),
        Arc::new(Mutex::new(ImSessionMapStore::in_memory())),
        Arc::new(EchoHandler),
        Arc::new(ImApprovalCloser::new(
            Arc::new(NoApprovals),
            Arc::new(MemoryApprovalAudit::new()),
        )),
    )
    .with_clock(test_clock());

    // 出站 inert: 无提案卡推送 (0 出站)。
    let pushed = bridge
        .push_proposal_cards("primary", CONVERSATION)
        .await
        .unwrap();
    assert!(pushed.is_empty(), "未接线 = inert");
    assert!(posted_bodies(&server).await.is_empty(), "0 出站");

    // 提案按钮回调照实报「未接线」(不假装能落账)。
    let payload = serde_json::json!({
        "approval_ref": "prop-x",
        "pair_id": "prop-x",
        "round": 0,
        "subject": "tool.proposal_draft",
        "expires_at_ms": 9_000_000_000_000_i64,
        "action": IM_BUTTON_APPROVE,
    });
    let result = bridge
        .handle_card_action(
            &ImChannelSpec {
                id: "primary".to_string(),
                kind: ImChannelKind::ImWecom,
                webhook_or_endpoint: server.uri(),
                secret: None,
                enabled: true,
            }
            .target()
            .unwrap(),
            ImChannelKind::ImWecom,
            apeireth_sdk::im::ImCardAction {
                conversation_id: CONVERSATION.to_string(),
                message_id: "msg_button".to_string(),
                operator_id: "u_demo".to_string(),
                payload: serde_json::from_value(payload).unwrap(),
            },
        )
        .await
        .unwrap();
    let reason = match result {
        ImInboundResult::ProposalRefused { reason, .. } => reason,
        other => panic!("expected a refusal frame, got {other:?}"),
    };
    assert!(reason.contains("未接线"), "{reason}");
    let _ = fs_err::remove_dir_all(&dir);
}
