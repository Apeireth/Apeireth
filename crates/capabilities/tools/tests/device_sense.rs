//! K 轴 L1 在场感知骨架 (`tool.device_sense` + `tool.weather`) 回归:
//! 白名单零设备默认 / 设备表拒开 (StoredDoc) / mock 感知真值 (固定 fixture) /
//! 非白名单拒读 / 摄像头敏感面显式授权 + 审计 / 天气 mock 如实标注数据源 /
//! 提醒信号格式 (只交信号) / 真实协议与 live 插槽留位不接 / 开关 fail-closed /
//! 自述在场节同表同源。

/// 本集成测试二进制内的 env 锁（进程级全局量 ⇒ 串行）。
/// 放在**模块级**（不在测试函数内）：同文件多个测试函数共用同一把。
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use apeireth_governance::{ToolGuard, ToolGuardRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::device::adapter::{AdapterRegistry, RESERVED_PROTOCOL_SLOTS};
use apeireth_tools_canonical::device::audit::DeviceSenseAuditLedger;
use apeireth_tools_canonical::device::registry::{
    Device, DeviceKind, DeviceRegistry, DevicesBody, DEVICES_FILE,
};
use apeireth_tools_canonical::device::sense::{
    device_presence_status, device_sense_enabled_from_env, DeviceSenseActivity,
    DeviceSenseBoundaryGuard, DeviceSenseConfig, DeviceSenseTool, DEVICE_SENSE_CAPABILITY_ID,
    DEVICE_SENSE_ENABLE_ENV,
};
use apeireth_tools_canonical::device::weather::{
    reminder_signal_id, weather_signals, LiveWeatherSource, MockWeatherSource, ReminderSignal,
    SignalLedger, WeatherError, WeatherSource, REMINDER_DISCIPLINE_NOTE, SOURCE_MOCK_FIXTURE,
    SOURCE_MOCK_NOTE, WEATHER_CAPABILITY_ID,
};
use apeireth_tools_canonical::{render_snapshot, SelfStatusSnapshot};
use serde_json::{json, Value};

/// 工具调用一帧 (按动作拼参数)。
fn call(arguments: Value) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: "device_sense".into(),
        arguments,
    }
}

/// 临时目录 (每测试一个, 落系统临时区)。
fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-device-sense-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试目录");
    dir
}

/// 设备表 + 感知工具 + 进程内计数 (同一数据目录)。
fn tool_with(tag: &str) -> (PathBuf, DeviceSenseTool, Arc<DeviceSenseActivity>) {
    let data_dir = temp_root(tag);
    let config = DeviceSenseConfig::new(&data_dir);
    let activity = Arc::new(DeviceSenseActivity::new());
    let tool = DeviceSenseTool::new(
        config.registry(),
        AdapterRegistry::builtin(),
        config.audit(),
        Arc::clone(&activity),
    );
    (data_dir, tool, activity)
}

/// 一台 mock 设备行。
fn mock_device(id: &str, kind: DeviceKind, enabled: bool) -> Device {
    Device {
        id: id.to_string(),
        kind,
        name: format!("设备 {id}"),
        adapter: "mock".to_string(),
        enabled,
    }
}

/// 落一张设备表 (直接走注册表写面)。
fn write_table(data_dir: &PathBuf, devices: Vec<Device>) {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    DeviceRegistry::under_data_dir(data_dir)
        .save(&DevicesBody { devices })
        .expect("设备表落盘");
}

async fn ok_value(tool: &DeviceSenseTool, arguments: Value) -> Value {
    let result = tool.invoke(&call(arguments)).await;
    assert!(result.is_ok(), "{}", result.render());
    serde_json::from_str(&result.render()).expect("结构化 json")
}

async fn refused(tool: &DeviceSenseTool, arguments: Value) -> String {
    let result = tool.invoke(&call(arguments)).await;
    assert!(!result.is_ok(), "必须拒绝即帧: {}", result.render());
    result.render()
}

/// ① 白名单零设备默认: `devices.json` 缺省 = 空表, 感知零设备不编造读数。
#[tokio::test]
async fn the_whitelist_defaults_to_zero_devices() {
    let (_data_dir, tool, _activity) = tool_with("zero-default");
    let listed = ok_value(&tool, json!({"action": "list"})).await;
    assert_eq!(listed["devices_total"], 0);
    assert_eq!(listed["whitelisted"], 0);
    assert_eq!(listed["table_present"], false);

    let sensed = ok_value(&tool, json!({"action": "sense"})).await;
    let states = sensed["states"].as_array().expect("states 在场");
    assert!(states.is_empty(), "默认零设备 = 零读数");
    assert!(sensed["note"].as_str().unwrap().contains("白名单零设备"));
}

/// ② 设备表拒开 (StoredDoc 语义): 坏表不静默修复, 工具如实拒绝即帧。
#[tokio::test]
async fn a_defective_device_table_refuses_to_open() {
    let (data_dir, tool, _activity) = tool_with("defect");
    std::fs::write(data_dir.join(DEVICES_FILE), b"{ not json").unwrap();
    let frame = refused(&tool, json!({"action": "sense"})).await;
    assert!(frame.contains("设备表拒开"), "{frame}");
    let frame = refused(&tool, json!({"action": "list"})).await;
    assert!(frame.contains("设备表拒开"), "{frame}");
}

/// ③ mock 感知真值: 固定 fixture 数据, 重复感知逐值一致, 数据源如实标注。
#[tokio::test]
async fn mock_sensing_reports_fixed_fixture_truth() {
    let (data_dir, tool, activity) = tool_with("mock-truth");
    write_table(
        &data_dir,
        vec![mock_device("thermo-1", DeviceKind::Sensor, true)],
    );
    let first = ok_value(&tool, json!({"action": "sense"})).await;
    let second = ok_value(&tool, json!({"action": "sense"})).await;
    let state = |frame: &Value| frame["states"].as_array().unwrap()[0].clone();
    let one = state(&first);
    let two = state(&second);
    assert_eq!(one["device_id"], "thermo-1");
    assert_eq!(one["kind"], "sensor");
    assert_eq!(one["adapter"], "mock");
    assert_eq!(one["readings"]["temperature_c"], 21.5, "固定 fixture 真值");
    assert_eq!(one["readings"]["humidity_pct"], 48.0);
    assert_eq!(one["source_note"], SOURCE_MOCK_FIXTURE);
    assert_eq!(one["readings"], two["readings"], "重复感知逐值一致");
    // 进程内计数与工具面同源: 2 次调用、2 台次。
    assert_eq!(activity.sense_calls(), 2);
    assert_eq!(activity.sensed_devices(), 2);
}

/// ④ 白名单制: 表外 / 停用设备一律拒绝即帧 (底线①)。
#[tokio::test]
async fn off_list_and_disabled_devices_are_refused() {
    let (data_dir, tool, _activity) = tool_with("whitelist");
    write_table(
        &data_dir,
        vec![
            mock_device("thermo-1", DeviceKind::Sensor, true),
            mock_device("cam-1", DeviceKind::Camera, false),
        ],
    );
    // 停用设备 (enabled=false) = 白名单外。
    let frame = refused(&tool, json!({"action": "sense", "device_ids": ["cam-1"]})).await;
    assert!(frame.contains("未启用"), "{frame}");
    // 表外 id。
    let frame = refused(&tool, json!({"action": "sense", "device_ids": ["ghost-1"]})).await;
    assert!(frame.contains("不在设备表上"), "{frame}");
    // 白名单内可读。
    let ok = ok_value(
        &tool,
        json!({"action": "sense", "device_ids": ["thermo-1"]}),
    )
    .await;
    assert_eq!(ok["whitelisted"], 1);
}

/// ⑤ 摄像头敏感面: 显式授权 + 审计 —— 无授权拒绝即帧且审计留痕, 授权齐备放行
/// 且审计留痕 (append-only)。
#[tokio::test]
async fn camera_reads_need_explicit_authorization_and_land_in_the_audit() {
    let (data_dir, tool, _activity) = tool_with("camera-audit");
    write_table(
        &data_dir,
        vec![mock_device("cam-1", DeviceKind::Camera, true)],
    );
    let audit = DeviceSenseAuditLedger::under_data_dir(&data_dir);

    // 无显式授权: 拒绝即帧 + 审计行 denied。
    let frame = refused(&tool, json!({"action": "sense"})).await;
    assert!(frame.contains("敏感读面"), "{frame}");
    let scan = audit.scan().expect("审计账可读");
    assert_eq!(scan.records.len(), 1);
    let row = &scan.records[0].body;
    assert_eq!(row.device_id, "cam-1");
    assert_eq!(row.outcome.as_str(), "denied");
    assert_eq!(row.authorized_by, None, "无授权人如实留痕");
    assert_eq!(row.action, "sense");

    // 显式授权齐备: 放行 + 审计行 granted (授权人如实留痕)。
    let ok = ok_value(
        &tool,
        json!({
            "action": "sense",
            "sensitive_read_authorized": true,
            "authorized_by": "owner",
        }),
    )
    .await;
    assert_eq!(ok["states"].as_array().unwrap().len(), 1);
    let scan = audit.scan().expect("审计账可读");
    assert_eq!(scan.records.len(), 2);
    let row = &scan.records[1].body;
    assert_eq!(row.outcome.as_str(), "granted");
    assert_eq!(row.authorized_by.as_deref(), Some("owner"));
    // 摄像头 fixture 只回事件位, 不回图像内容。
    let state = &ok["states"][0];
    for key in state["readings"].as_object().unwrap().keys() {
        assert!(
            !key.contains("frame") && !key.contains("image"),
            "摄像头读面不回图像内容: {key}"
        );
    }
}

/// ⑥ 敏感读面授权诚实性: 开了授权旗但没给授权人 = 拒 (工具边界 + 单调 Guard 双侧)。
#[tokio::test]
async fn sensitive_authorization_needs_an_honest_authorizer() {
    let (data_dir, tool, _activity) = tool_with("camera-honest");
    write_table(
        &data_dir,
        vec![mock_device("cam-1", DeviceKind::Camera, true)],
    );
    let frame = refused(
        &tool,
        json!({"action": "sense", "sensitive_read_authorized": true, "authorized_by": "  "}),
    )
    .await;
    assert!(frame.contains("敏感读面"), "{frame}");

    // 单调边界 Guard 同判 (只拒不放, 任何后续阶段不能翻回)。
    let guard = DeviceSenseBoundaryGuard::new();
    let arguments = json!({"action": "sense", "sensitive_read_authorized": true});
    let request = ToolGuardRequest {
        capability: &apeireth_core::kernel::CapabilityId::new(DEVICE_SENSE_CAPABILITY_ID).unwrap(),
        tool_name: "device_sense",
        arguments: &arguments,
    };
    let refusal = guard.deny(&request).expect("不诚实授权必须被 Guard 拒");
    assert!(refusal.contains("authorized_by"), "{refusal}");
}

/// ⑦ 天气 mock 档如实标注数据源: `mock-fixture` + 非实测注记, 每帧恒在场。
#[tokio::test]
async fn weather_mock_is_labeled_as_mock_honestly() {
    let source = MockWeatherSource::new();
    let report = source
        .observe("阳台", 1_700_000_000_000)
        .expect("mock 可读");
    assert_eq!(report.data_source, SOURCE_MOCK_FIXTURE);
    assert_eq!(report.data_note, SOURCE_MOCK_NOTE);
    assert!(report.data_note.contains("非实测天气"));
    assert_eq!(report.temperature_c, Some(2.5));
}

/// ⑧ 提醒信号格式: 字段合同稳定 + 纪律恒声明「只交信号, 提醒侧消费」+ 入账。
#[tokio::test]
async fn reminder_signals_follow_the_frozen_format_and_are_only_handed_over() {
    let report = MockWeatherSource::fixture("local", 1_700_000_000_000);
    let signals = weather_signals(&report);
    assert_eq!(signals.len(), 1);
    let signal = &signals[0];
    assert!(
        signal.signal_id.starts_with("wxsig-1700000000000-"),
        "{}",
        signal.signal_id
    );
    assert_eq!(
        signal.signal_id,
        reminder_signal_id(
            1_700_000_000_000,
            signal
                .signal_id
                .rsplit('-')
                .next()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        )
    );
    assert_eq!(signal.kind, "weather");
    assert_eq!(signal.trigger, "temperature_drop");
    assert_eq!(signal.severity, "warn");
    assert_eq!(signal.source, WEATHER_CAPABILITY_ID);
    assert_eq!(signal.source_data, SOURCE_MOCK_FIXTURE);
    assert_eq!(signal.discipline, REMINDER_DISCIPLINE_NOTE);
    assert!(signal.discipline.contains("只交信号"));
    assert!(signal.discipline.contains("提醒侧"));
    assert!(signal.evidence.contains("temperature_c"));

    // 信号账 (提醒侧消费面) round-trip: append-only, 逐行 StoredDoc。
    let data_dir = temp_root("signal-format");
    let ledger = SignalLedger::under_data_dir(&data_dir);
    ledger.append(signal).expect("信号入账");
    let scan = ledger.scan().expect("信号账可读");
    assert_eq!(scan.records.len(), 1);
    let stored: &ReminderSignal = &scan.records[0].body;
    assert_eq!(stored, signal);
    // 字段合同冻结: 序列化键逐个在场 (提醒侧消费契约)。
    let as_json = serde_json::to_value(signal).unwrap();
    for key in [
        "signal_id",
        "kind",
        "trigger",
        "severity",
        "message",
        "evidence",
        "source",
        "source_data",
        "created_at_ms",
        "discipline",
    ] {
        assert!(as_json.get(key).is_some(), "信号字段合同缺 {key}");
    }
}

/// ⑨ 真实协议族与 live 天气档都**留插槽不接**: 未接线如实报, 不出数不冒充。
#[tokio::test]
async fn real_protocol_and_live_slots_refuse_until_wired() {
    let registry = AdapterRegistry::builtin();
    for slot in RESERVED_PROTOCOL_SLOTS {
        assert!(
            registry.resolve(slot).is_err(),
            "协议插槽 {slot} 未接线必须拒"
        );
    }
    let live = LiveWeatherSource::slot("https://weather.example.invalid");
    let error = live.observe("local", 0).expect_err("live 插槽必须报未接线");
    assert!(matches!(error, WeatherError::SlotNotWired { .. }));

    // 设备表把设备挂在未接线插槽上 = 感知拒绝即帧 (真因如实)。
    let (data_dir, tool, _activity) = tool_with("slot");
    write_table(
        &data_dir,
        vec![Device {
            id: "cam-bridge".into(),
            kind: DeviceKind::Camera,
            name: "桥接摄像头".into(),
            adapter: "cloud_bridge".into(),
            enabled: true,
        }],
    );
    let frame = refused(
        &tool,
        json!({"action": "sense", "sensitive_read_authorized": true, "authorized_by": "owner"}),
    )
    .await;
    assert!(frame.contains("留插槽不接"), "{frame}");
}

/// ⑩ 开关 fail-closed: `APEIRETH_ENABLE_DEVICE_SENSE` 只认显式 "1"。
#[tokio::test]
async fn the_device_sense_switch_is_fail_closed() {
    // 串行护住 env 读写 (进程级全局量)。
    // ⚠️ **2026-10-27 补**：本测试**改进程级 env**（`DEVICE_SENSE_ENABLE_ENV`）
    // ⇒ **同二进制的其它测试**可能同时读到 ⇒ ⇒ 跨测试污染
    // ⇒ （`scripts/check_env_race.py` 清单把它列为「未见同函数内持锁」的候选）。
    // ⇒ 照本文件既有写法（`:82`）取同一把锁。
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    std::env::remove_var(DEVICE_SENSE_ENABLE_ENV);
    assert!(!device_sense_enabled_from_env());
    assert!(DeviceSenseConfig::from_env(".").is_none());
    assert!(
        apeireth_tools_canonical::device::weather::WeatherConfig::from_env(".").is_none(),
        "天气面同族同闸 (默认关 fail-closed)"
    );
    for value in ["0", "true", "on", " 1 0 ", ""] {
        std::env::set_var(DEVICE_SENSE_ENABLE_ENV, value);
        assert!(!device_sense_enabled_from_env(), "must refuse {value:?}");
    }
    std::env::set_var(DEVICE_SENSE_ENABLE_ENV, "1");
    assert!(device_sense_enabled_from_env());
    assert!(DeviceSenseConfig::from_env(".").is_some());
    assert!(apeireth_tools_canonical::device::weather::WeatherConfig::from_env(".").is_some());
    std::env::remove_var(DEVICE_SENSE_ENABLE_ENV);
}

/// ⑪ 自述在场节: 设备数 / 白名单数 / 最近感知与工具面同表同源, 缺省表如实注。
#[tokio::test]
async fn the_presence_section_reports_the_same_truth_as_the_tool_face() {
    let (data_dir, tool, activity) = tool_with("presence");
    let registry = DeviceRegistry::under_data_dir(&data_dir);

    // 缺省表: 默认零设备, 尚无感知不编时间。
    let status = device_presence_status(&registry, &activity).expect("缺省不是错误");
    assert_eq!(status.devices, Some(0));
    assert_eq!(status.whitelisted, Some(0));
    assert_eq!(status.last_sense_at_ms, None);

    // 落表 + 感知一次: 同一真值入自述节。
    write_table(
        &data_dir,
        vec![
            mock_device("thermo-1", DeviceKind::Sensor, true),
            mock_device("cam-1", DeviceKind::Camera, false),
        ],
    );
    let sensed = ok_value(&tool, json!({"action": "sense"})).await;
    assert_eq!(sensed["states"].as_array().unwrap().len(), 1);
    let status = device_presence_status(&registry, &activity).expect("表可开");
    assert_eq!(status.devices, Some(2));
    assert_eq!(status.whitelisted, Some(1));
    let last_sense_at_ms = status.last_sense_at_ms.expect("最近感知时刻在场");
    assert!(status
        .last_sense_note
        .as_deref()
        .unwrap()
        .contains("已感知"));
    assert!(status.note.as_deref().unwrap().contains("进程内计数"));

    // 渲染进自述帧 (在场节恒在场)。
    let mut snapshot = SelfStatusSnapshot::minimal(Default::default(), Vec::new());
    snapshot.device_presence = Some(status);
    snapshot.device_presence_reason = None;
    let value = render_snapshot(&snapshot);
    assert_eq!(value["presence"]["devices"], 2);
    assert_eq!(value["presence"]["whitelisted"], 1);
    assert_eq!(value["presence"]["last_sense_at_ms"], last_sense_at_ms);
    assert_eq!(value["presence"]["reason"], Value::Null);
}

/// ⑫ 词表外动作拒绝即帧 (闭合词表)。
#[tokio::test]
async fn unknown_actions_are_refused_with_the_closed_vocabulary() {
    let (_data_dir, tool, _activity) = tool_with("vocab");
    let frame = refused(&tool, json!({"action": "reboot"})).await;
    assert!(frame.contains("closed vocabulary"), "{frame}");
    let frame = refused(&tool, json!({"surprise": 1})).await;
    assert!(frame.contains("invalid device_sense parameters"), "{frame}");
}
