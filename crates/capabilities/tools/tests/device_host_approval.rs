//! 设备面「同族缺陷」回归 (2026-11-01 安全审计): **模型参数不得冒充授权人**。
//!
//! ① 敏感读面 (`tool.device_sense`): 摄像头类读的放行只认**宿主决定账**的批准
//!    记录 —— 模型自称的 `authorized_by` 一律不作数 (不采信、不落账), 审计账里
//!    的授权人 = **宿主署名**;
//! ② 审批证据面 (`tool.device_control`): 不可逆/动钱档 (以及 reversible 审批档)
//!    的「谁批」查**决定账 + 推送账** (照抄 IM 面判据) —— 模型自称的
//!    `approved_by` / `im_card_id` 一律不作数, 状态账里的授权人 = **宿主署名**。
//!
//! 两侧同族 ⇒ 两侧各钉一遍 (审计纪律: 同族缺陷必须钉在两处)。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::device::adapter::AdapterRegistry;
use apeireth_tools_canonical::device::approval::{
    DeviceActionDecision, DeviceApprovalDecisionLedger, DeviceApprovalDecisionRecord,
    DeviceApprovalPushLedger, DeviceApprovalPushOutcome, DeviceApprovalPushRecord,
};
use apeireth_tools_canonical::device::audit::DeviceSenseAuditLedger;
use apeireth_tools_canonical::device::device_control::{
    ActionGrade, DeviceActionLedger, DeviceApprovalCard, DeviceControlConfig, DeviceControlRunner,
    DeviceEntry, DeviceRegistry, DeviceState, MockDeviceAdapter, AUTO_PASS_APPROVER,
    DEVICE_CONTROL_CAPABILITY_ID,
};
use apeireth_tools_canonical::device::registry::{Device, DeviceKind, DevicesBody};
use apeireth_tools_canonical::device::sense::{
    sense_plan_id, DeviceSenseActivity, DeviceSenseApprovalHook, DeviceSenseConfig,
    DeviceSenseTool, DEVICE_SENSE_CAPABILITY_ID,
};
use serde_json::{json, Value};

/// 临时目录 (每测试一个)。
fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-device-host-approval-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试目录");
    dir
}

/// 感知工具调用一帧。
fn call(name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: name.into(),
        arguments,
    }
}

/// 感知面夹具: 设备表 + 感知工具 (同一数据目录)。
fn sense_tool(data_dir: &PathBuf, devices: Vec<Device>) -> DeviceSenseTool {
    let config = DeviceSenseConfig::new(data_dir);
    apeireth_tools_canonical::device::registry::DeviceRegistry::under_data_dir(data_dir)
        .save(&DevicesBody { devices })
        .expect("设备表落盘");
    DeviceSenseTool::new(
        config.registry(),
        AdapterRegistry::builtin(),
        config.audit(),
        config.decisions(),
        Arc::new(DeviceSenseActivity::new()),
    )
}

/// 执行面夹具: 白名单 + mock 适配器 + 执行核心 (同一数据目录)。
fn control_runner(
    data_dir: &PathBuf,
    entries: Vec<DeviceEntry>,
    auto_pass: bool,
) -> Arc<DeviceControlRunner> {
    let registry = DeviceRegistry::new();
    let mut device_ids = Vec::new();
    for entry in entries {
        device_ids.push(entry.id.clone());
        registry.register(entry).expect("登记设备");
    }
    let adapter = MockDeviceAdapter::new();
    for id in device_ids {
        adapter.ensure_device(&id, DeviceState::new());
    }
    Arc::new(DeviceControlRunner::new(
        DeviceControlConfig::new(data_dir).with_auto_pass_reversible(auto_pass),
        registry,
        adapter,
    ))
}

/// 一台 mock 感知设备行。
fn sense_device(id: &str, kind: DeviceKind) -> Device {
    Device {
        id: id.to_string(),
        kind,
        name: format!("设备 {id}"),
        adapter: "mock".to_string(),
        enabled: true,
    }
}

/// 宿主拍板 (决定账落一条批准记录): 设备面放行的唯一来源。
fn host_approves(data_dir: &PathBuf, plan_id: &str, who: &str) {
    DeviceApprovalDecisionLedger::under_data_dir(data_dir)
        .append(&DeviceApprovalDecisionRecord {
            plan_id: plan_id.to_string(),
            decision: DeviceActionDecision::Approved,
            decided_by: who.to_string(),
            im_card_id: None,
            at_ms: 1_700_000_000_000,
        })
        .expect("宿主批准落决定账");
}

/// 宿主批准被拒 (决定账落一条拒绝记录)。
fn host_rejects(data_dir: &PathBuf, plan_id: &str, who: &str) {
    DeviceApprovalDecisionLedger::under_data_dir(data_dir)
        .append(&DeviceApprovalDecisionRecord {
            plan_id: plan_id.to_string(),
            decision: DeviceActionDecision::Rejected,
            decided_by: who.to_string(),
            im_card_id: None,
            at_ms: 1_700_000_000_000,
        })
        .expect("宿主拒绝落决定账");
}

/// 宿主把审批卡推到手机并拿到送达回执 (推送账一行 `pushed`)。
fn host_pushes_card(data_dir: &PathBuf, plan_id: &str, message_ref: &str) {
    let card = DeviceApprovalCard {
        plan_id: plan_id.to_string(),
        device: "shop".to_string(),
        device_name: "商店".to_string(),
        device_action: "purchase".to_string(),
        grade: ActionGrade::Money.as_str().to_string(),
        params: json!({"item": "牛奶", "amount": 12.5}),
        effects: vec!["charged_amount: - → 12.5".to_string()],
        requires_im_card: true,
        approval_note: "动钱动作: 永远逐次人批 + IM 审批卡 (无自动档, 底线②)。".to_string(),
    };
    DeviceApprovalPushLedger::under_data_dir(data_dir)
        .append(&DeviceApprovalPushRecord {
            card,
            channel_id: "primary".to_string(),
            conversation_id: "oc_demo_chat".to_string(),
            outcome: DeviceApprovalPushOutcome::Pushed,
            message_ref: Some(message_ref.to_string()),
            reason: None,
            at_ms: 1_700_000_000_000,
        })
        .expect("宿主推送落推送账");
}

/// 治理请求 (能力派发形态)。
fn dispatch(capability_id: &str, arguments: Value) -> GovernanceRequest<'static> {
    let capability: &'static CapabilityId = Box::leak(Box::new(
        CapabilityId::new(capability_id).expect("capability id"),
    ));
    let arguments: &'static Value = Box::leak(Box::new(arguments));
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

// ---------------------------------------------------------------------------
// ① 敏感读面: 模型不得自称授权人
// ---------------------------------------------------------------------------

/// ①-a 模型带 `authorized_by` 但**没有宿主批准** ⇒ 拒绝即帧, 且审计账里的
/// 授权人是空的 (模型自称的字符串不作数)。
#[tokio::test]
async fn the_model_cannot_claim_the_sensitive_read_authorization() {
    let data_dir = temp_root("sense-no-host");
    let camera = sense_device("cam-1", DeviceKind::Camera);
    let tool = sense_tool(&data_dir, vec![camera.clone()]);
    let audit = DeviceSenseAuditLedger::under_data_dir(&data_dir);

    let result = tool
        .invoke(&call(
            "device_sense",
            json!({
                "action": "sense",
                "sensitive_read_authorized": true,
                "authorized_by": "模型自称的主人",
            }),
        ))
        .await;
    assert!(!result.is_ok(), "模型自称授权必须被拒: {}", result.render());

    let scan = audit.scan().expect("审计账可读");
    assert_eq!(scan.records.len(), 1, "每次尝试都入账");
    let row = &scan.records[0].body;
    assert_eq!(row.device_id, "cam-1");
    assert_eq!(row.outcome.as_str(), "denied");
    assert_eq!(row.authorized_by, None, "审计账里不许出现模型自称的授权人");
}

/// ①-b 宿主批准 ⇒ 放行, 且审计账里的授权人是**宿主**的值 (不是模型给的字符串)。
#[tokio::test]
async fn the_host_approval_releases_the_sensitive_read_and_names_the_host() {
    let data_dir = temp_root("sense-host-ok");
    let camera = sense_device("cam-1", DeviceKind::Camera);
    let tool = sense_tool(&data_dir, vec![camera.clone()]);
    let audit = DeviceSenseAuditLedger::under_data_dir(&data_dir);

    // 宿主拍板 (决定账) = 这一批敏感设备的这一次读。
    host_approves(&data_dir, &sense_plan_id(&[&camera]), "宿主:面板@local");

    let result = tool
        .invoke(&call(
            "device_sense",
            json!({
                "action": "sense",
                "sensitive_read_authorized": true,
                "authorized_by": "模型自称的主人",
            }),
        ))
        .await;
    assert!(result.is_ok(), "宿主批准后放行: {}", result.render());

    let scan = audit.scan().expect("审计账可读");
    assert_eq!(scan.records.len(), 1);
    let row = &scan.records[0].body;
    assert_eq!(row.outcome.as_str(), "granted");
    assert_eq!(
        row.authorized_by.as_deref(),
        Some("宿主:面板@local"),
        "审计账里的授权人 = 宿主署名, 不是模型给的字符串"
    );
}

/// ①-c 宿主审批门 (治理钩子): 摄像头类敏感读停宿主人批; 其余读面放行。
#[tokio::test]
async fn the_sensitive_read_stops_at_the_host_approval_gate() {
    let data_dir = temp_root("sense-hook");
    let camera = sense_device("cam-1", DeviceKind::Camera);
    let thermo = sense_device("thermo-1", DeviceKind::Sensor);
    let _tool = sense_tool(&data_dir, vec![camera, thermo]);
    let hook = DeviceSenseApprovalHook::new(
        apeireth_tools_canonical::device::registry::DeviceRegistry::under_data_dir(&data_dir),
    );

    // 摄像头类: 宿主人批 (无自动放行档)。
    let decision = hook
        .evaluate(&dispatch(
            DEVICE_SENSE_CAPABILITY_ID,
            json!({"action": "sense", "device_ids": ["cam-1"]}),
        ))
        .await;
    let Decision::RequireApproval { reason } = decision else {
        panic!("敏感读必须停宿主人批: {decision:?}");
    };
    assert!(reason.contains("敏感读面"), "{reason}");

    // 非敏感设备 / 白名单视图: 放行。
    for arguments in [
        json!({"action": "sense", "device_ids": ["thermo-1"]}),
        json!({"action": "list"}),
    ] {
        let decision = hook
            .evaluate(&dispatch(DEVICE_SENSE_CAPABILITY_ID, arguments))
            .await;
        assert_eq!(decision, Decision::Allow, "非敏感读面放行: {decision:?}");
    }

    // 词表外拍: fail-closed 拒绝即帧。
    let decision = hook
        .evaluate(&dispatch(
            DEVICE_SENSE_CAPABILITY_ID,
            json!({"action": "reboot"}),
        ))
        .await;
    assert!(matches!(decision, Decision::Deny { .. }), "{decision:?}");
}

// ---------------------------------------------------------------------------
// ② 审批证据面: 不可逆/动钱档的「谁批」查决定账 + 推送账
// ---------------------------------------------------------------------------

/// ②-a 模型带 `approved_by` 但**没有宿主批准** ⇒ 拒绝即帧 (零副作用); 宿主
/// 批准 ⇒ 放行, 且状态账里的授权人是**宿主**的值。
#[tokio::test]
async fn the_model_cannot_claim_the_approval_of_an_irreversible_action() {
    let data_dir = temp_root("control-no-host");
    let runner = control_runner(
        &data_dir,
        vec![DeviceEntry {
            id: "front_door".into(),
            name: "前门".into(),
            kind: "door".into(),
            enabled: true,
            actions: vec!["unlock_door".into()],
        }],
        false,
    );
    let tool =
        apeireth_tools_canonical::device::device_control::DeviceControlTool::new(runner.clone());
    let ledger = DeviceActionLedger::under_data_dir(&data_dir);

    // plan → preview (取回 plan_id)。
    let planned = tool
        .invoke(&call(
            "device_control",
            json!({"action": "plan", "device": "front_door", "device_action": "unlock_door"}),
        ))
        .await;
    assert!(planned.is_ok(), "{}", planned.render());
    let plan_id = serde_json::from_str::<Value>(&planned.render()).expect("plan 帧是 json")
        ["plan_id"]
        .as_str()
        .expect("plan_id 进帧")
        .to_string();
    let previewed = tool
        .invoke(&call(
            "device_control",
            json!({"action": "preview", "device": "front_door", "device_action": "unlock_door", "plan_id": plan_id}),
        ))
        .await;
    assert!(previewed.is_ok(), "{}", previewed.render());

    // 模型自称「主人」⇒ 宿主没拍板 ⇒ 拒绝即帧 (零副作用: 状态账不落行)。
    let refused = tool
        .invoke(&call(
            "device_control",
            json!({"action": "execute", "device": "front_door", "device_action": "unlock_door", "plan_id": plan_id, "approved_by": "模型自称的主人"}),
        ))
        .await;
    assert!(
        !refused.is_ok(),
        "模型自称批准必须被拒: {}",
        refused.render()
    );
    assert!(
        ledger.scan().expect("状态账可读").records.is_empty(),
        "拒绝即帧 = 零副作用 (状态账不落行)"
    );

    // 宿主拍板 ⇒ 放行, 且「谁批」= 宿主署名。
    host_approves(&data_dir, &plan_id, "宿主:面板@local");
    let executed = tool
        .invoke(&call(
            "device_control",
            json!({"action": "execute", "device": "front_door", "device_action": "unlock_door", "plan_id": plan_id, "approved_by": "模型自称的主人"}),
        ))
        .await;
    assert!(executed.is_ok(), "宿主批准后放行: {}", executed.render());
    let scan = ledger.scan().expect("状态账可读");
    assert_eq!(scan.records.len(), 1);
    assert_eq!(
        scan.records[0].body.approved_by, "宿主:面板@local",
        "状态账里的「谁批」= 宿主署名, 不是模型给的字符串"
    );
}

/// ②-b 宿主拒绝了 ⇒ 不翻案 (一计划一决定), 拒绝即帧。
#[tokio::test]
async fn a_host_rejection_is_not_overturned() {
    let data_dir = temp_root("control-host-reject");
    let runner = control_runner(
        &data_dir,
        vec![DeviceEntry {
            id: "front_door".into(),
            name: "前门".into(),
            kind: "door".into(),
            enabled: true,
            actions: vec!["unlock_door".into()],
        }],
        false,
    );
    let tool =
        apeireth_tools_canonical::device::device_control::DeviceControlTool::new(runner.clone());
    let plan_id = runner
        .plan("front_door", "unlock_door", &json!({}))
        .expect("plan")
        .plan_id;
    runner
        .preview("front_door", "unlock_door", &json!({}), Some(&plan_id))
        .expect("preview");
    host_rejects(&data_dir, &plan_id, "宿主:面板@local");

    let refused = tool
        .invoke(&call(
            "device_control",
            json!({"action": "execute", "device": "front_door", "device_action": "unlock_door", "plan_id": plan_id, "approved_by": "模型自称的主人"}),
        ))
        .await;
    assert!(!refused.is_ok(), "宿主已拒 ⇒ 不翻案: {}", refused.render());
    assert!(
        DeviceActionLedger::under_data_dir(&data_dir)
            .scan()
            .expect("状态账可读")
            .records
            .is_empty(),
        "拒绝即帧 = 零副作用"
    );
}

/// ②-c 动钱档: 「谁批」查决定账, IM 卡查**推送账**的送达回执 (卡没到手机就没有
/// 回执 ⇒ 拒绝即帧); 送达回执到账后才放行, 且卡号取自宿主推送账。
#[tokio::test]
async fn the_money_grade_needs_the_host_push_receipt() {
    let data_dir = temp_root("control-money");
    let runner = control_runner(
        &data_dir,
        vec![DeviceEntry {
            id: "shop".into(),
            name: "商店".into(),
            kind: "service".into(),
            enabled: true,
            actions: vec!["purchase".into()],
        }],
        false,
    );
    let tool =
        apeireth_tools_canonical::device::device_control::DeviceControlTool::new(runner.clone());
    let params = json!({"item": "牛奶", "amount": 12.5, "currency": "CNY"});
    let plan_id = runner
        .plan("shop", "purchase", &params)
        .expect("plan")
        .plan_id;
    runner
        .preview("shop", "purchase", &params, Some(&plan_id))
        .expect("preview");
    let ledger = DeviceActionLedger::under_data_dir(&data_dir);

    // 宿主拍板了, 但审批卡没送达 (无推送账回执) ⇒ 拒绝即帧。
    host_approves(&data_dir, &plan_id, "宿主:面板@local");
    let refused = tool
        .invoke(&call(
            "device_control",
            json!({"action": "execute", "device": "shop", "device_action": "purchase", "params": {"item": "牛奶", "amount": 12.5, "currency": "CNY"}, "plan_id": plan_id, "im_card_id": "模型自称的卡号"}),
        ))
        .await;
    assert!(
        !refused.is_ok(),
        "查无送达记录必须被拒: {}",
        refused.render()
    );
    assert!(
        ledger.scan().expect("状态账可读").records.is_empty(),
        "拒绝即帧 = 零副作用"
    );

    // 宿主把卡推到手机 (送达回执落推送账) ⇒ 放行, 卡号取宿主推送账。
    host_pushes_card(&data_dir, &plan_id, "m-out");
    let executed = tool
        .invoke(&call(
            "device_control",
            json!({"action": "execute", "device": "shop", "device_action": "purchase", "params": {"item": "牛奶", "amount": 12.5, "currency": "CNY"}, "plan_id": plan_id, "im_card_id": "模型自称的卡号"}),
        ))
        .await;
    assert!(
        executed.is_ok(),
        "宿主送达回执到账后放行: {}",
        executed.render()
    );
    let scan = ledger.scan().expect("状态账可读");
    assert_eq!(scan.records.len(), 1);
    assert_eq!(scan.records[0].body.approved_by, "宿主:面板@local");
    assert_eq!(
        scan.records[0].body.im_card_id.as_deref(),
        Some("m-out"),
        "IM 卡号取宿主推送账的送达回执, 不是模型给的字符串"
    );
}

/// ②-d 宿主自动档 (reversible 自动放行): 「谁批」= 宿主配置的如实标签, 不是
/// 模型自称的字符串。
#[tokio::test]
async fn the_host_auto_pass_names_the_host_not_the_model() {
    let data_dir = temp_root("control-auto-pass");
    let runner = control_runner(
        &data_dir,
        vec![DeviceEntry {
            id: "lamp".into(),
            name: "灯".into(),
            kind: "appliance".into(),
            enabled: true,
            actions: vec!["power_on".into()],
        }],
        true,
    );
    let tool =
        apeireth_tools_canonical::device::device_control::DeviceControlTool::new(runner.clone());
    let plan_id = runner
        .plan("lamp", "power_on", &json!({}))
        .expect("plan")
        .plan_id;
    runner
        .preview("lamp", "power_on", &json!({}), Some(&plan_id))
        .expect("preview");

    let executed = tool
        .invoke(&call(
            "device_control",
            json!({"action": "execute", "device": "lamp", "device_action": "power_on", "plan_id": plan_id, "approved_by": "模型自称的主人"}),
        ))
        .await;
    assert!(executed.is_ok(), "自动档放行: {}", executed.render());
    let scan = DeviceActionLedger::under_data_dir(&data_dir)
        .scan()
        .expect("状态账可读");
    assert_eq!(scan.records.len(), 1);
    assert_eq!(
        scan.records[0].body.approved_by, AUTO_PASS_APPROVER,
        "自动档的「谁批」= 宿主配置的标签, 不是模型给的字符串"
    );
}

/// ②-e 工具面三拍走通 (plan → preview → execute): 宿主拍板后执行, 且执行帧里
/// 的「谁批」= **宿主署名** (模型自称的字符串不作数)。
///
/// (本测试从 `device_control.rs` 的模块内测试搬来 —— 该文件的尺寸棘轮已到顶,
/// 覆盖面不变: 三拍语义 + 缺 plan_id 即拒 + 词表外拍即拒 + 输出归一字段。)
#[tokio::test]
async fn the_tool_face_walks_the_three_beats_with_the_host_approval() {
    use apeireth_protocol::canonical::ToolOutcome;

    fn value_of(result: &apeireth_protocol::canonical::ToolResult) -> Value {
        match &result.outcome {
            ToolOutcome::Ok { value } => value.clone(),
            other => panic!("必须成功帧: {other:?}"),
        }
    }

    let data_dir = temp_root("tool-three-beats");
    let runner = Arc::new(DeviceControlRunner::new(
        DeviceControlConfig::new(&data_dir),
        DeviceRegistry::with_mock_fixtures(),
        MockDeviceAdapter::with_builtin_fixtures(),
    ));
    let tool =
        apeireth_tools_canonical::device::device_control::DeviceControlTool::new(runner.clone());
    let call = |id: &str, arguments: Value| ToolCall {
        id: id.to_string(),
        name: "device_control".to_string(),
        arguments,
    };

    let planned = tool
        .invoke(&call(
            "c1",
            json!({"action": "plan", "device": "lamp", "device_action": "set_brightness", "params": {"level": 30}}),
        ))
        .await;
    assert!(planned.is_ok(), "{planned:?}");
    let planned = value_of(&planned);
    assert_eq!(planned["action"], json!("plan"));
    assert_eq!(planned["status"], json!("planned"));
    assert_eq!(planned["grade"], json!("reversible"));
    assert_eq!(
        planned["zero_side_effect"],
        json!(true),
        "plan 帧自报零副作用"
    );
    let plan_id = planned["plan_id"]
        .as_str()
        .expect("plan_id 进帧")
        .to_string();

    let previewed = tool
        .invoke(&call(
            "c2",
            json!({"action": "preview", "device": "lamp", "device_action": "set_brightness", "params": {"level": 30}, "plan_id": plan_id}),
        ))
        .await;
    assert!(previewed.is_ok(), "{previewed:?}");
    let previewed = value_of(&previewed);
    assert_eq!(previewed["status"], json!("previewed"));
    assert!(previewed["approval_card"].is_object(), "审批卡载荷随预览帧");

    // 宿主拍板 (决定账落一条批准记录): 「谁批」= 宿主署名。
    host_approves(&data_dir, &plan_id, "宿主:面板@local");

    let executed = tool
        .invoke(&call(
            "c3",
            json!({"action": "execute", "device": "lamp", "device_action": "set_brightness", "params": {"level": 30}, "plan_id": plan_id, "approved_by": "模型自称的主人"}),
        ))
        .await;
    assert!(executed.is_ok(), "{executed:?}");
    let executed = value_of(&executed);
    assert_eq!(executed["status"], json!("executed"));
    assert_eq!(
        executed["approved_by"],
        json!("宿主:面板@local"),
        "「谁批」只认宿主署名 (模型自称的不作数)"
    );
    assert!(executed["record_id"].is_string(), "状态账记录 id 进帧");

    // 缺 plan_id 的 preview/execute 即拒。
    let refused = tool
        .invoke(&call(
            "c4",
            json!({"action": "execute", "device": "lamp", "device_action": "set_brightness", "params": {"level": 30}, "approved_by": "模型自称的主人"}),
        ))
        .await;
    assert!(!refused.is_ok(), "{refused:?}");
    // 词表外拍即拒。
    let refused = tool
        .invoke(&call(
            "c5",
            json!({"action": "wish", "device": "lamp", "device_action": "power_on"}),
        ))
        .await;
    assert!(!refused.is_ok(), "{refused:?}");
}
