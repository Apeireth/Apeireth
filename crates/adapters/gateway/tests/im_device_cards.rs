//! 设备审批卡→IM 推送链 (K 轴 L3/L4 收尾缝) 集成测试: 卡出站 (同链同信封) /
//! 按钮批准即执行 + 落账署名 / money 档必须 IM 卡 / 推送幂等 (plan_id 键) /
//! 失败留账重试 (凭账本载荷重建) / 一计划一决定 / 未接线照实报 / 工具审批卡
//! 路由零碰撞。外部端点一律本地 mock, 0 真实网络。

mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::SessionId;
use apeireth_gateway::{
    assemble_im_channels, ImApprovalCloser, ImApprovalNotice, ImApprovalResolution,
    ImApprovalResolver, ImBridge, ImChannelConfig, ImChannelSpec, ImDeviceApprovalFace,
    ImInboundResult, ImSessionMapStore, ImTurnHandler, ImTurnOutcome, MemoryApprovalAudit,
    DEVICE_APPROVAL_CARD_SUBJECT, DEVICE_CARD_NO_DEADLINE_MS,
};
use apeireth_sdk::im::{
    ImButtonPayload, ImChannelKind, ImHttpSender, ImReconnectPolicy, ImSegmentPolicy,
    IM_BUTTON_APPROVE, IM_BUTTON_REJECT,
};
use apeireth_tools_canonical::device::approval::{
    DeviceActionDecision, DeviceApprovalDecisionLedger, DeviceApprovalPushLedger,
    DeviceApprovalPushOutcome,
};
use apeireth_tools_canonical::device::device_control::{
    ApprovalEvidence, DeviceActionLedger, DeviceApprovalCard, DeviceControlConfig,
    DeviceControlRunner, DeviceEntry, DeviceRegistry, DeviceState, MockDeviceAdapter,
};
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

/// 无工具审批的解析器 (设备审批卡走计划卡路由, 不进审批四态; 本夹具只测设备链)。
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

/// 回声对话链 (回合文本可预测)。
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

struct DeviceHarness {
    server: MockServer,
    bridge: ImBridge,
    runner: Arc<DeviceControlRunner>,
    data_dir: PathBuf,
}

impl DeviceHarness {
    /// 出站到 mock IM 端点的全部 body。
    async fn posted(&self) -> Vec<String> {
        posted_bodies(&self.server).await
    }

    /// 走真三拍出一张审批卡 (预览拍产卡载荷)。
    fn preview_card(
        &self,
        device: &str,
        action: &str,
        params: &serde_json::Value,
    ) -> (DeviceApprovalCard, String) {
        let preview = self
            .runner
            .preview(device, action, params, None)
            .expect("preview 拍");
        let card = DeviceApprovalCard::from_preview(&preview);
        (card, preview.plan_id)
    }
}

fn money_params(item: &str) -> serde_json::Value {
    serde_json::json!({"item": item, "amount": 12.5, "currency": "CNY"})
}

fn place_order_params() -> serde_json::Value {
    serde_json::json!({"item": "纸巾", "quantity": 2, "amount": 30.0, "currency": "CNY"})
}

/// 一座接了设备审批面的桥: 真执行核心 (白名单 = 灯/门/商店三件显式登记启用,
/// mock 适配器零真实设备) + 真两本账落盘 + mock IM 端点 (同一套出站链与信封)。
async fn device_harness(server: MockServer, data_dir: &Path) -> DeviceHarness {
    let registry = DeviceRegistry::new();
    for entry in [
        DeviceEntry {
            id: "lamp".into(),
            name: "灯".into(),
            kind: "appliance".into(),
            enabled: true,
            actions: vec!["set_brightness".into()],
        },
        DeviceEntry {
            id: "door".into(),
            name: "门".into(),
            kind: "other".into(),
            enabled: true,
            actions: vec!["unlock_door".into()],
        },
        DeviceEntry {
            id: "shop".into(),
            name: "商店".into(),
            kind: "service".into(),
            enabled: true,
            actions: vec!["purchase".into(), "place_order".into()],
        },
    ] {
        registry.register(entry).expect("登记设备");
    }
    let adapter = MockDeviceAdapter::new();
    adapter.ensure_device("lamp", DeviceState::new());
    adapter.ensure_device("door", DeviceState::new());
    adapter.ensure_device("shop", DeviceState::new());
    let runner = Arc::new(DeviceControlRunner::new(
        DeviceControlConfig::new(data_dir),
        registry,
        adapter,
    ));
    let face = Arc::new(ImDeviceApprovalFace::new(data_dir, Arc::clone(&runner)));
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
    .with_device_approval_face(face);
    DeviceHarness {
        server,
        bridge,
        runner,
        data_dir: data_dir.to_path_buf(),
    }
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-im-device-{tag}-{}", std::process::id()));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("create test dir");
    dir
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
fn assert_envelopes_isomorphic(approval_body: &str, device_body: &str) {
    let approval: serde_json::Value = serde_json::from_str(approval_body).unwrap();
    let device: serde_json::Value = serde_json::from_str(device_body).unwrap();
    assert_eq!(
        envelope_shape(&approval),
        envelope_shape(&device),
        "设备审批卡出站必须与 IM 审批卡同信封 (逐层键集同构)\n审批体: {approval_body}\n设备体: {device_body}"
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

fn action_rows(
    data_dir: &Path,
) -> Vec<apeireth_tools_canonical::device::device_control::DeviceActionRecord> {
    DeviceActionLedger::under_data_dir(data_dir)
        .scan()
        .unwrap()
        .records
        .into_iter()
        .map(|record| record.body)
        .collect()
}

fn decision_rows(
    data_dir: &Path,
) -> Vec<apeireth_tools_canonical::device::approval::DeviceApprovalDecisionRecord> {
    DeviceApprovalDecisionLedger::under_data_dir(data_dir)
        .scan()
        .unwrap()
        .records
        .into_iter()
        .map(|record| record.body)
        .collect()
}

// ---------------------------------------------------------------------------
// 1. 卡出站信封同构 (同链同信封 + 按钮闭合身份 = plan_id)
// ---------------------------------------------------------------------------

/// 1. 卡出站 (信封同构断言): 设备审批卡经同一条出站链、同一个 ImApprovalCard
///    信封到手机; 三拍纪律 / money 档 IM 卡标注 / 分级风险档随卡; 按钮载荷 =
///    合法闭合身份 (pair_id = plan_id = 推送幂等键)。
#[tokio::test]
async fn device_cards_reach_the_phone_through_the_same_envelope_as_approval_cards() {
    let server = ok_endpoint().await;
    let dir = temp_dir("envelope");
    let harness = device_harness(server, &dir).await;
    let (card, plan_id) = harness.preview_card("shop", "purchase", &money_params("牛奶"));

    let pushed = harness
        .bridge
        .push_device_approval_cards("primary", CONVERSATION, &[card])
        .await
        .unwrap();
    assert_eq!(pushed.len(), 1);
    assert_eq!(pushed[0].plan_id, plan_id);
    assert_eq!(pushed[0].outcome, DeviceApprovalPushOutcome::Pushed);

    let bodies = harness.posted().await;
    assert_eq!(bodies.len(), 1, "恰好一张设备审批卡出站");
    let device_body = bodies[0].clone();
    for line in ["将发生什么 (先模拟后执行)", "IM 审批卡", "计划: "] {
        assert!(device_body.contains(line), "「{line}」必须随卡到手机");
    }
    assert!(device_body.contains(&plan_id), "卡面带计划 id");
    assert!(device_body.contains("high"), "money 档风险档 = 高");

    // 同信封: 工具审批卡走同一条链出站 → 两个体逐层键集同构 (0 第二套卡信封)。
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
    assert_envelopes_isomorphic(all.last().unwrap(), &device_body);

    // 按钮往返载荷: 两按钮都是合法 ImButtonPayload, 闭合身份 = plan_id。
    for action in [IM_BUTTON_APPROVE, IM_BUTTON_REJECT] {
        let payload: ImButtonPayload =
            serde_json::from_value(button_payload(&device_body, action)).unwrap();
        payload.validate().unwrap();
        assert_eq!(payload.approval_ref, plan_id);
        assert_eq!(payload.pair_id, plan_id, "一计划一决定的闭合身份 = plan_id");
        assert_eq!(payload.subject, DEVICE_APPROVAL_CARD_SUBJECT);
    }
    let _ = fs_err::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// 2-3. 批准即执行 + 落账署名 / money 档必须 IM 卡
// ---------------------------------------------------------------------------

/// 2. 批准按钮 = 批准即执行 + 落账署名: 执行落设备动作状态账, `approved_by` =
///    IM 用户名 + 渠道标注 (如实), `im_card_id` = 送达回执; 决定账同落一板。
#[tokio::test]
async fn approve_click_executes_the_action_signed_by_the_im_user_and_channel() {
    let server = ok_endpoint().await;
    let dir = temp_dir("approve");
    let harness = device_harness(server, &dir).await;
    let (card, plan_id) = harness.preview_card("shop", "purchase", &money_params("牛奶"));
    harness
        .bridge
        .push_device_approval_cards("primary", CONVERSATION, &[card])
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
        ImInboundResult::DeviceDecided {
            closure,
            reply_segments,
        } => {
            assert_eq!(reply_segments, 1);
            closure
        }
        other => panic!("expected a device decision, got {other:?}"),
    };
    assert_eq!(closure.plan_id, plan_id);
    assert_eq!(closure.decision, DeviceActionDecision::Approved);
    assert_eq!(
        closure.decided_by, "im:u_demo@primary",
        "decided_by = IM 用户名 + 渠道标注"
    );
    assert!(closure.executed, "批准即执行");
    assert_eq!(
        closure.im_card_id.as_deref(),
        Some("m-out"),
        "money 档 IM 卡 = 送达回执"
    );
    assert!(
        closure.reply_text.contains("im:u_demo@primary"),
        "回执带署名"
    );

    // 落账真值: 设备动作状态账一行 (谁批如实署名 + IM 卡回执) + 决定账一板。
    let rows = action_rows(&dir);
    assert_eq!(rows.len(), 1, "批准即执行: 状态账恰好一行");
    assert_eq!(rows[0].plan_id, plan_id);
    assert_eq!(rows[0].approved_by, "im:u_demo@primary", "按钮落账署名");
    assert_eq!(rows[0].im_card_id.as_deref(), Some("m-out"));
    assert_eq!(rows[0].grade, "money");
    assert_eq!(
        rows[0].after.get("charged_amount"),
        Some(&serde_json::json!(12.5)),
        "真执行: mock 状态真变更"
    );
    let decisions = decision_rows(&dir);
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].decision, DeviceActionDecision::Approved);
    assert_eq!(decisions[0].decided_by, "im:u_demo@primary");
    assert_eq!(
        DeviceApprovalDecisionLedger::under_data_dir(&dir)
            .decision_for(&plan_id)
            .unwrap(),
        Some(DeviceActionDecision::Approved)
    );
    let _ = fs_err::remove_dir_all(&dir);
}

/// 3. money 档必须 IM 卡 (三面断言): ① 面板侧执行缺 im_card_id 拒绝即帧
///    (零副作用); ② IM 批准落地的账行必须带送达回执作 im_card_id; ③ 查无
///    送达记录的点击不可信 (无回执 = 无 IM 卡), 拒绝即帧且零执行。
#[tokio::test]
async fn money_grade_execute_demands_an_im_card() {
    let server = ok_endpoint().await;
    let dir = temp_dir("money");
    let harness = device_harness(server, &dir).await;

    // ① 面板侧 (无 IM 卡): money 档永远逐次人批 + IM 卡, 缺卡拒绝即帧。
    harness
        .runner
        .preview("shop", "place_order", &place_order_params(), None)
        .expect("preview");
    let refused = harness
        .runner
        .execute(
            "shop",
            "place_order",
            &place_order_params(),
            None,
            &ApprovalEvidence {
                approved_by: Some("主人".to_string()),
                im_card_id: None,
            },
        )
        .expect_err("money 档缺 IM 卡必须拒");
    assert!(
        refused.message().contains("IM 审批卡"),
        "拒绝真因带 money 档口径: {}",
        refused.message()
    );
    assert!(action_rows(&dir).is_empty(), "缺卡拒绝 = 零副作用 (不落账)");

    // ② IM 批准: 账行必须带送达回执作 im_card_id。
    let (card, plan_id) = harness.preview_card("shop", "purchase", &money_params("牛奶"));
    harness
        .bridge
        .push_device_approval_cards("primary", CONVERSATION, &[card])
        .await
        .unwrap();
    let body = harness.posted().await.last().unwrap().clone();
    let payload = button_payload(&body, IM_BUTTON_APPROVE);
    harness
        .bridge
        .handle_inbound("primary", &wecom_button_event(CONVERSATION, &payload), None)
        .await
        .unwrap();
    let rows = action_rows(&dir);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].im_card_id.as_deref(),
        Some("m-out"),
        "money 账行必须带 IM 卡回执"
    );

    // ③ 无送达记录的点击 (伪造/未送达): 无回执 = 无 IM 卡, 拒绝即帧零执行。
    let forged: ImButtonPayload = serde_json::from_value(serde_json::json!({
        "approval_ref": plan_id,
        "pair_id": plan_id,
        "round": 0,
        "subject": DEVICE_APPROVAL_CARD_SUBJECT,
        "expires_at_ms": DEVICE_CARD_NO_DEADLINE_MS,
        "action": IM_BUTTON_APPROVE,
    }))
    .unwrap();
    // 另一个从没推送过的计划 (查无送达记录)。
    let (_card2, plan2) = harness.preview_card("shop", "purchase", &money_params("面包"));
    let forged: ImButtonPayload = ImButtonPayload {
        approval_ref: plan2.clone(),
        pair_id: plan2,
        ..forged
    };
    let result = harness
        .bridge
        .handle_inbound(
            "primary",
            &wecom_button_event(CONVERSATION, &serde_json::to_value(&forged).unwrap()),
            None,
        )
        .await
        .unwrap();
    let reason = match result {
        ImInboundResult::DeviceRefused { reason, .. } => reason,
        other => panic!("expected a refusal frame, got {other:?}"),
    };
    assert!(reason.contains("查无计划"), "{reason}");
    assert_eq!(
        action_rows(&dir).len(),
        1,
        "无送达记录的点击零执行 (账面不动)"
    );
    let _ = fs_err::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// 4-5. 推送幂等 (plan_id 键) / 失败留账重试
// ---------------------------------------------------------------------------

/// 4. 推送幂等 (同计划单推): 推送幂等键 = 动作 plan_id, 重复推送 0 二次出站;
///    换座桥 (重启后同账) 仍被账本幂等键挡住。
#[tokio::test]
async fn the_same_plan_is_never_pushed_twice() {
    let server = ok_endpoint().await;
    let dir = temp_dir("idempotent");
    let harness = device_harness(server, &dir).await;
    let (card, plan_id) = harness.preview_card("shop", "purchase", &money_params("牛奶"));

    let first = harness
        .bridge
        .push_device_approval_cards("primary", CONVERSATION, &[card.clone()])
        .await
        .unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].plan_id, plan_id, "幂等键 = 动作 plan_id");
    let second = harness
        .bridge
        .push_device_approval_cards("primary", CONVERSATION, &[card.clone()])
        .await
        .unwrap();
    assert!(second.is_empty(), "幂等: 同计划不重复推送");

    // 换座桥 (重启后同账): 幂等键在账本, 不在内存。
    let retry_harness = device_harness(ok_endpoint().await, &dir).await;
    let third = retry_harness
        .bridge
        .push_device_approval_cards("primary", CONVERSATION, &[card])
        .await
        .unwrap();
    assert!(third.is_empty(), "账本幂等键跨桥生效");

    let bodies = harness.posted().await;
    let card_bodies: Vec<&String> = bodies
        .iter()
        .filter(|body| body.contains("将发生什么 (先模拟后执行)"))
        .collect();
    assert_eq!(card_bodies.len(), 1, "同计划单推: 恰好一张设备审批卡出站");
    assert_eq!(
        DeviceApprovalPushLedger::under_data_dir(&dir)
            .pushed_plan_ids()
            .unwrap()
            .len(),
        1
    );
    assert!(retry_harness.posted().await.is_empty(), "跨桥 0 二次出站");
    let _ = fs_err::remove_dir_all(&dir);
}

/// 5. 失败留账重试: 推送失败 → `failed` 行 (真因 + 卡载荷留痕) 待重试; 换座桥
///    凭**账本行载荷**重建出站送达, 重试成功后幂等键生效不再重推。
#[tokio::test]
async fn a_failed_push_stays_on_the_books_and_the_retry_rebuilds_from_the_ledger() {
    let failing = failing_endpoint().await;
    let dir = temp_dir("failure");
    let harness = device_harness(failing, &dir).await;
    let (card, plan_id) = harness.preview_card("shop", "purchase", &money_params("牛奶"));

    let results = harness
        .bridge
        .push_device_approval_cards("primary", CONVERSATION, &[card])
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].outcome, DeviceApprovalPushOutcome::Failed);
    assert!(
        results[0].detail.contains("delivery failed"),
        "{}",
        results[0].detail
    );

    // 失败留账 (真因 + 卡载荷): 重试只凭账本即可重建。
    let pushes = DeviceApprovalPushLedger::under_data_dir(&dir);
    let scan = pushes.scan().unwrap();
    assert_eq!(scan.records.len(), 1);
    assert_eq!(
        scan.records[0].body.outcome,
        DeviceApprovalPushOutcome::Failed
    );
    assert_eq!(scan.records[0].body.plan_id(), plan_id);
    assert_eq!(
        scan.records[0].body.card.params["amount"],
        serde_json::json!(12.5),
        "卡载荷随行留痕 (含执行所需真参数)"
    );
    assert!(
        scan.records[0]
            .body
            .reason
            .as_deref()
            .unwrap()
            .contains("delivery failed"),
        "失败真因如实"
    );
    assert!(
        pushes.pushed_plan_ids().unwrap().is_empty(),
        "失败不算已送达"
    );
    assert_eq!(
        pushes.pending_retry().unwrap().len(),
        1,
        "失败留账 = 待重试"
    );

    // 重试 (换座桥 = 重启后同账): 凭账本载荷重建出站送达。
    let retry_harness = device_harness(ok_endpoint().await, &dir).await;
    let retried = retry_harness
        .bridge
        .retry_device_approval_pushes()
        .await
        .unwrap();
    assert_eq!(retried.len(), 1);
    assert_eq!(retried[0].plan_id, plan_id);
    assert_eq!(retried[0].outcome, DeviceApprovalPushOutcome::Pushed);
    let after = retry_harness
        .bridge
        .retry_device_approval_pushes()
        .await
        .unwrap();
    assert!(after.is_empty(), "重试成功后不再重推 (幂等)");
    let pushes = DeviceApprovalPushLedger::under_data_dir(&dir);
    assert_eq!(
        pushes.scan().unwrap().records.len(),
        2,
        "失败留账 + 成功留痕都在"
    );
    assert_eq!(pushes.pushed_plan_ids().unwrap().len(), 1);
    let _ = fs_err::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// 6-8. 拒绝关账 (一计划一决定) / 未接线照实报 / 工具审批卡路由零碰撞
// ---------------------------------------------------------------------------

/// 6. 拒绝按钮 = 不执行 + 关账: 落决定账 (decided_by 如实署名), 设备零执行;
///    一计划一决定 —— 再点批准拒绝即帧 (真因如实), 账面恰好一板。
#[tokio::test]
async fn reject_click_lands_a_signed_decision_and_closes_the_plan() {
    let server = ok_endpoint().await;
    let dir = temp_dir("reject");
    let harness = device_harness(server, &dir).await;
    let (card, plan_id) = harness.preview_card("door", "unlock_door", &serde_json::json!({}));
    harness
        .bridge
        .push_device_approval_cards("primary", CONVERSATION, &[card])
        .await
        .unwrap();
    let body = harness.posted().await.last().unwrap().clone();

    let reject = button_payload(&body, IM_BUTTON_REJECT);
    let first = harness
        .bridge
        .handle_inbound("primary", &wecom_button_event(CONVERSATION, &reject), None)
        .await
        .unwrap();
    let closure = match first {
        ImInboundResult::DeviceDecided { closure, .. } => closure,
        other => panic!("expected a device decision, got {other:?}"),
    };
    assert_eq!(closure.decision, DeviceActionDecision::Rejected);
    assert!(!closure.executed, "拒绝即不执行");
    assert_eq!(closure.decided_by, "im:u_demo@primary", "拒绝也要如实署名");
    assert!(action_rows(&dir).is_empty(), "拒绝 = 设备零执行");

    // 一计划一决定: 再点批准拒绝即帧, 不翻案 (不可逆动作真的不会发生)。
    let approve = button_payload(&body, IM_BUTTON_APPROVE);
    let again = harness
        .bridge
        .handle_inbound("primary", &wecom_button_event(CONVERSATION, &approve), None)
        .await
        .unwrap();
    let reason = match again {
        ImInboundResult::DeviceRefused { reason, .. } => reason,
        other => panic!("expected a refusal frame, got {other:?}"),
    };
    assert!(reason.contains("一计划一决定"), "{reason}");
    assert!(reason.contains(&plan_id), "{reason}");
    assert!(action_rows(&dir).is_empty(), "重复点击不执行");
    let decisions = decision_rows(&dir);
    assert_eq!(decisions.len(), 1, "一计划一决定: 账面恰好一板");
    assert_eq!(decisions[0].decision, DeviceActionDecision::Rejected);
    let _ = fs_err::remove_dir_all(&dir);
}

/// 7. 未接设备审批面 = 照实报「未接线」+ 出站 inert (0 假旋钮, 既有链零回归)。
#[tokio::test]
async fn without_the_device_face_the_bridge_reports_unwired_honestly() {
    let server = ok_endpoint().await;
    let dir = temp_dir("unwired");
    let spec = ImChannelSpec {
        id: "primary".to_string(),
        kind: ImChannelKind::ImWecom,
        webhook_or_endpoint: server.uri(),
        secret: None,
        enabled: true,
    };
    let assembly = assemble_im_channels(
        &ImChannelConfig::new(vec![spec.clone()]),
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

    // 出站 inert: 无设备审批卡推送 (0 出站)。
    let card = DeviceApprovalCard {
        plan_id: "0123456789abcdef".to_string(),
        device: "shop".to_string(),
        device_name: "商店".to_string(),
        device_action: "purchase".to_string(),
        grade: "money".to_string(),
        params: money_params("牛奶"),
        effects: vec!["charged_amount: - → 12.5".to_string()],
        requires_im_card: true,
        approval_note: "动钱动作: 永远逐次人批 + IM 审批卡 (无自动档, 底线②)。".to_string(),
    };
    let pushed = bridge
        .push_device_approval_cards("primary", CONVERSATION, &[card])
        .await
        .unwrap();
    assert!(pushed.is_empty(), "未接线 = inert");
    assert!(posted_bodies(&server).await.is_empty(), "0 出站");

    // 设备审批卡按钮回调照实报「未接线」(不假装能落账)。
    let payload = serde_json::json!({
        "approval_ref": "0123456789abcdef",
        "pair_id": "0123456789abcdef",
        "round": 0,
        "subject": DEVICE_APPROVAL_CARD_SUBJECT,
        "expires_at_ms": DEVICE_CARD_NO_DEADLINE_MS,
        "action": IM_BUTTON_APPROVE,
    });
    let result = bridge
        .handle_card_action(
            &spec.target().unwrap(),
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
        ImInboundResult::DeviceRefused { reason, .. } => reason,
        other => panic!("expected a refusal frame, got {other:?}"),
    };
    assert!(reason.contains("未接线"), "{reason}");
    let _ = fs_err::remove_dir_all(&dir);
}

/// 8. 路由零碰撞: `tool.device_control` 的**工具审批卡** (subject = 能力 id,
///    闭合身份 = approval id) 照旧走四态闭合链, 不进设备计划卡路由。
#[tokio::test]
async fn tool_approval_cards_for_device_control_still_close_through_the_four_state_chain() {
    let server = ok_endpoint().await;
    let dir = temp_dir("routing");
    let harness = device_harness(server, &dir).await;

    // 先来一条文本消息 (建立会话映射, 四态闭合需要同源会话)。
    harness
        .bridge
        .handle_inbound("primary", &wecom_text_event(CONVERSATION, "在吗"), None)
        .await
        .unwrap();

    let payload = serde_json::json!({
        "approval_ref": "apr_1",
        "pair_id": "pair-1",
        "round": 1,
        "subject": "tool.device_control",
        "expires_at_ms": 4_000_000_000_000_i64,
        "action": IM_BUTTON_APPROVE,
    });
    let result = harness
        .bridge
        .handle_inbound("primary", &wecom_button_event(CONVERSATION, &payload), None)
        .await
        .unwrap();
    match result {
        ImInboundResult::Closed { closure, .. } => {
            assert_eq!(closure.subject, "tool.device_control");
            assert_eq!(
                closure.resolution_label, "unavailable",
                "工具审批卡走四态闭合 (本夹具解析器查无, 如实 unavailable)"
            );
        }
        other => panic!("工具审批卡必须走四态闭合链, 不进设备计划卡路由: {other:?}"),
    }
    let _ = fs_err::remove_dir_all(&dir);
}
