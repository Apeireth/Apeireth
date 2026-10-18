//! K 轴 L3/L4 受控执行自述接线 (设备审批卡→IM 批) 的探针真值回归: 自述
//! 设备受控执行节探测口 (`device_control_status_probe`) 与 `tool.device_control`
//! 工具面**同一台执行核心**同账同源 (控制计数 / 最近动作真值), 缺源显式
//! null + 原因 (0 装诚实)。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_runtime_assembly::{
    device_control_status_probe, ProductionSelfStatusSource, SelfStatusSource,
};
use apeireth_tools_canonical::device::device_control::{
    ApprovalEvidence, DeviceControlConfig, DeviceControlRunner, DeviceEntry, DeviceRegistry,
    DeviceState, MockDeviceAdapter,
};

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-device-control-probe-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试目录");
    dir
}

/// 执行核心夹具: 灯 (reversible) / 门 (irreversible) / 商店 (money) 三件显式
/// 登记启用 + mock 适配器 (零真实设备)。
fn runner_with_fixtures(data_dir: &PathBuf) -> Arc<DeviceControlRunner> {
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
            actions: vec!["purchase".into()],
        },
    ] {
        registry.register(entry).expect("登记设备");
    }
    let adapter = MockDeviceAdapter::new();
    adapter.ensure_device("lamp", DeviceState::new());
    adapter.ensure_device("door", DeviceState::new());
    adapter.ensure_device("shop", DeviceState::new());
    Arc::new(DeviceControlRunner::new(
        DeviceControlConfig::new(data_dir),
        registry,
        adapter,
    ))
}

fn evidence(approved_by: &str, im_card_id: Option<&str>) -> ApprovalEvidence {
    ApprovalEvidence {
        approved_by: Some(approved_by.to_string()),
        im_card_id: im_card_id.map(str::to_string),
    }
}

/// ① 探针真值: 控制计数 (分级计数) / 最近动作与工具面同一台执行核心同账同源
///    (三档各走一拍, 状态账真值逐列对得上)。
#[test]
fn the_device_control_probe_reports_control_counts_and_last_action_truth() {
    let data_dir = temp_root("truth");
    let runner = runner_with_fixtures(&data_dir);

    runner
        .preview(
            "lamp",
            "set_brightness",
            &serde_json::json!({"level": 42}),
            None,
        )
        .expect("preview");
    runner
        .execute(
            "lamp",
            "set_brightness",
            &serde_json::json!({"level": 42}),
            None,
            &evidence("主人", None),
        )
        .expect("execute reversible");
    runner
        .preview("door", "unlock_door", &serde_json::json!({}), None)
        .expect("preview");
    runner
        .execute(
            "door",
            "unlock_door",
            &serde_json::json!({}),
            None,
            &evidence("主人", None),
        )
        .expect("execute irreversible");
    runner
        .preview(
            "shop",
            "purchase",
            &serde_json::json!({"item": "牛奶", "amount": 12.5, "currency": "CNY"}),
            None,
        )
        .expect("preview");
    runner
        .execute(
            "shop",
            "purchase",
            &serde_json::json!({"item": "牛奶", "amount": 12.5, "currency": "CNY"}),
            None,
            &evidence("im:u_demo@primary", Some("m-out")),
        )
        .expect("execute money");

    let probe = device_control_status_probe(Arc::clone(&runner));
    let status = probe().expect("探针现探测");
    assert_eq!(status.devices_total, 3, "白名单登记设备数");
    assert_eq!(status.devices_enabled, 3, "白名单启用设备数");
    assert_eq!(status.actions_executed, 3, "控制计数 = 状态账行数");
    assert_eq!(status.executed_reversible, 1);
    assert_eq!(status.executed_irreversible, 1);
    assert_eq!(status.executed_money, 1);
    // 跨线机械补位 (K 轴 L2 消费端批代补编译断点, 报备): clone 取值不移出,
    // 下方 `section.last_action == status.last_action` 对账照旧。
    let last_action = status.last_action.clone().expect("最近动作真值在场");
    assert!(
        last_action.contains("shop / purchase") && last_action.contains("谁批: im:u_demo@primary"),
        "最近动作 = 设备/动作/时刻/谁批: {last_action}"
    );
    assert_eq!(status.reason, None);

    // 探测口真接进自述来源: 设备受控执行节带真值出帧。
    let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
        .with_device_control_probe(device_control_status_probe(Arc::clone(&runner)));
    let snapshot = SelfStatusSource::snapshot(&source);
    let section = snapshot.device_control.expect("设备受控执行节在场");
    assert_eq!(section.actions_executed, 3);
    assert_eq!(section.executed_money, 1);
    assert_eq!(section.last_action, status.last_action);
}

/// ② 缺源显式 null + 原因: 探测口没接 = 节显式 null + 「not wired」, 不编数。
#[test]
fn the_device_control_section_is_explicit_null_without_a_probe() {
    let source = ProductionSelfStatusSource::new(Vec::new(), false, None);
    let snapshot = SelfStatusSource::snapshot(&source);
    assert!(snapshot.device_control.is_none(), "缺探测口 = 缺节体");
    assert!(
        snapshot
            .device_control_reason
            .as_deref()
            .unwrap_or_default()
            .contains("not wired"),
        "缺源如实注明: {:?}",
        snapshot.device_control_reason
    );
}
