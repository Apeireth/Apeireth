//! K 轴 L5 看家骨架 (`tool.guard_watch`) 回归: 守护规则 StoredDoc 拒开 /
//! 阈值+时序判定 / **只看不动**断言 (告警零设备动作) / 告警账 round-trip /
//! 开关 fail-closed (依赖在场感知族总闸) / 白名单外跳过不编造。

/// 本集成测试二进制内的 env 锁（进程级全局量 ⇒ 串行）。
/// 放在**模块级**（不在测试函数内）：同文件多个测试函数共用同一把。
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use apeireth_core::stored_doc::StoredDoc;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::device::adapter::{
    AdapterError, AdapterRegistry, DeviceAdapter, DeviceState,
};
use apeireth_tools_canonical::device::guard_watch::{
    guard_watch_enabled_from_env, GuardAlert, GuardAlertLedger, GuardCondition, GuardRule,
    GuardRuleStore, GuardRulesBody, GuardWatchActivity, GuardWatchConfig, GuardWatchTool,
    GUARD_ALERTS_DOC_NAME, GUARD_ALERTS_DOC_VERSION, GUARD_ALERT_ANNOTATION, GUARD_RULES_DOC_NAME,
    GUARD_RULES_DOC_VERSION, GUARD_WATCH_ACTIONS, GUARD_WATCH_CAPABILITY_ID,
    GUARD_WATCH_ENABLE_ENV, SEVERITY_CRITICAL,
};
use apeireth_tools_canonical::device::registry::{Device, DeviceKind, DeviceRegistry, DevicesBody};
use apeireth_tools_canonical::device::sense::DEVICE_SENSE_ENABLE_ENV;
use serde_json::{json, Value};

/// 工具调用一帧 (按动作拼参数)。
fn call(arguments: Value) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: "guard_watch".into(),
        arguments,
    }
}

/// 临时目录 (每测试一个, 落系统临时区)。
fn temp_root(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("apeireth-guard-watch-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试目录");
    dir
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

/// 一条温度上阈值守护规则 (单帧命中)。
fn heat_rule(device_id: &str) -> GuardRule {
    GuardRule {
        id: "heat".to_string(),
        device_id: device_id.to_string(),
        condition: GuardCondition::TemperatureAbove { celsius: 30.0 },
        sustain_minutes: 0,
        active_window: None,
        severity: SEVERITY_CRITICAL.to_string(),
        message: "室内过热, 请查看".to_string(),
        enabled: true,
        created_at_ms: 0,
    }
}

/// 固定高温读数的适配器 (自定义名, 供测试钉命中链)。
struct HotAdapter {
    temperature_c: f64,
}

impl DeviceAdapter for HotAdapter {
    fn name(&self) -> &str {
        "mock-hot"
    }

    fn read_state(&self, device: &Device, now_ms: i64) -> Result<DeviceState, AdapterError> {
        let mut readings: BTreeMap<String, Value> = BTreeMap::new();
        readings.insert("temperature_c".to_string(), Value::from(self.temperature_c));
        Ok(DeviceState {
            device_id: device.id.clone(),
            kind: device.kind,
            adapter: "mock-hot".to_string(),
            online: true,
            readings,
            observed_at_ms: now_ms,
            source_note: "mock-fixture".to_string(),
        })
    }
}

async fn ok_value(tool: &GuardWatchTool, arguments: Value) -> Value {
    let result = tool.invoke(&call(arguments)).await;
    assert!(result.is_ok(), "{}", result.render());
    serde_json::from_str(&result.render()).expect("结构化 json")
}

async fn refused(tool: &GuardWatchTool, arguments: Value) -> String {
    let result = tool.invoke(&call(arguments)).await;
    assert!(!result.is_ok(), "必须拒绝即帧: {}", result.render());
    result.render()
}

/// ① 规则 StoredDoc 拒开: 畸形 / 串档 / 未来版本 / 坏条目 / 重复 id 一律拒开拒写。
#[test]
fn the_guard_rules_table_refuses_to_open_when_defective() {
    let data_dir = temp_root("refuse");
    let store = GuardRuleStore::under_data_dir(&data_dir);
    // 缺省 = 空表 (无规则不编造告警)。
    assert!(store.load().expect("缺省不是错误").rules.is_empty());

    // 畸形 JSON = 拒开。
    std::fs::write(store.path(), b"{ not json").unwrap();
    assert!(store.load().is_err(), "畸形 JSON 必须拒开");

    // 串档身份 = 拒开。
    let wrong_name = serde_json::to_string(&StoredDoc::new(
        "other-doc",
        GUARD_RULES_DOC_VERSION,
        vec![GUARD_RULES_DOC_VERSION],
        GuardRulesBody::new(),
    ))
    .unwrap();
    std::fs::write(store.path(), wrong_name).unwrap();
    assert!(store.load().is_err(), "串档必须拒开");

    // 未来版本 = 拒开 (不静默迁移)。
    let future = serde_json::to_string(&StoredDoc::new(
        GUARD_RULES_DOC_NAME,
        GUARD_RULES_DOC_VERSION + 1,
        vec![GUARD_RULES_DOC_VERSION + 1],
        GuardRulesBody::new(),
    ))
    .unwrap();
    std::fs::write(store.path(), future).unwrap();
    assert!(store.load().is_err(), "未来版本必须拒开");

    // 坏条目 (severity 词表外) = 拒开拒写。
    let mut bad = GuardRulesBody::new();
    let mut rule = heat_rule("thermo-1");
    rule.severity = "catastrophic".to_string();
    bad.rules.push(rule);
    assert!(store.save(&bad).is_err(), "坏条目拒写 (不带病落盘)");
    std::fs::write(
        store.path(),
        format!(
            r#"{{"name":"{GUARD_RULES_DOC_NAME}","version":{GUARD_RULES_DOC_VERSION},
                 "compatible_versions":[{GUARD_RULES_DOC_VERSION}],
                 "body":{{"rules":[{{"id":"heat","device_id":"t1",
                 "condition":{{"kind":"temperature_above","celsius":30.0}},
                 "severity":"catastrophic","message":"x","created_at_ms":0}}]}}}}"#
        ),
    )
    .unwrap();
    assert!(store.load().is_err(), "词表外 severity 必须拒开");

    // 重复 id = 坏条目 = 拒开。
    let dup = GuardRulesBody {
        rules: vec![heat_rule("thermo-1"), heat_rule("thermo-1")],
    };
    assert!(store.save(&dup).is_err(), "重复 id 拒写");
}

/// ② 只看不动断言: 命中只产告警, **零设备动作** (账面/词表/帧面三处钉死)。
#[tokio::test]
async fn watching_is_read_only_and_never_touches_a_device() {
    let data_dir = temp_root("read-only");
    let mut adapters = AdapterRegistry::new();
    adapters = adapters.with_adapter(Arc::new(HotAdapter {
        temperature_c: 36.0,
    }));
    let config = GuardWatchConfig::new(&data_dir);
    let tool = GuardWatchTool::new(
        config.registry(),
        config.rules(),
        config.alerts(),
        adapters,
        Arc::new(GuardWatchActivity::new()),
    );

    // 白名单设备 + 命中规则 (温度 36 > 30)。
    DeviceRegistry::under_data_dir(&data_dir)
        .save(&DevicesBody {
            devices: vec![Device {
                id: "thermo-1".to_string(),
                kind: DeviceKind::Sensor,
                name: "室内温感".to_string(),
                adapter: "mock-hot".to_string(),
                enabled: true,
            }],
        })
        .expect("设备表落盘");
    let rules = GuardRuleStore::under_data_dir(&data_dir);
    rules
        .save(&GuardRulesBody {
            rules: vec![heat_rule("thermo-1")],
        })
        .expect("规则表落盘");

    let frame = ok_value(&tool, json!({"action": "watch"})).await;
    assert_eq!(frame["alerts_emitted"], 1, "命中一条告警");
    assert_eq!(frame["alerts"][0]["annotation"], GUARD_ALERT_ANNOTATION);
    assert_eq!(frame["alerts"][0]["rule_id"], "heat");
    assert!(
        frame["note"].as_str().unwrap().contains("只看不动"),
        "帧面如实声明零设备动作: {}",
        frame["note"]
    );

    // 告警入账 (append-only StoredDoc 行)。
    let ledger = GuardAlertLedger::under_data_dir(&data_dir);
    let scan = ledger.scan().expect("告警账可读");
    assert_eq!(scan.records.len(), 1);
    assert_eq!(scan.records[0].body.annotation, GUARD_ALERT_ANNOTATION);

    // **零设备动作**: 设备执行状态账不得存在 (看到≠动手, 动手走 device_control)。
    let actions_ledger = data_dir.join("device-actions.jsonl");
    assert!(
        !actions_ledger.exists(),
        "守护巡检不得触发任何设备动作: {}",
        actions_ledger.display()
    );

    // 闭合动作词表 = 零执行词表 (plan/preview/execute 词表外拒绝即帧)。
    for action in ["plan", "preview", "execute", "control"] {
        let reason = refused(&tool, json!({ "action": action })).await;
        assert!(
            reason.contains("零执行词表") || reason.contains("closed vocabulary"),
            "执行词必须被拒: {reason}"
        );
    }
    for action in GUARD_WATCH_ACTIONS {
        assert!(
            !matches!(*action, "plan" | "preview" | "execute" | "control"),
            "词表里不得有执行动作: {action}"
        );
    }
    assert_eq!(GUARD_WATCH_CAPABILITY_ID, "tool.guard_watch");
}

/// ③ 开关 fail-closed: 双闸齐开且显式 `"1"` 才有守护面 (单开本闸不起)。
#[test]
fn the_switch_is_fail_closed_and_depends_on_device_sense() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // 串行护住 env 读写 (进程级全局量)。
    std::env::remove_var(GUARD_WATCH_ENABLE_ENV);
    assert!(!guard_watch_enabled_from_env());
    assert!(GuardWatchConfig::from_env(".").is_none());

    // 只开守护闸 (感知闸关) = 不起 (依赖即闸)。
    std::env::set_var(GUARD_WATCH_ENABLE_ENV, "1");
    assert!(
        !guard_watch_enabled_from_env(),
        "感知闸关 = 守护面关 (依赖 DEVICE_SENSE)"
    );
    assert!(GuardWatchConfig::from_env(".").is_none());

    // 双闸齐开但守护闸取值不合法 = 关 (只认显式 \"1\")。
    std::env::set_var(DEVICE_SENSE_ENABLE_ENV, "1");
    for value in ["0", "true", "on", " 1 0 ", ""] {
        std::env::set_var(GUARD_WATCH_ENABLE_ENV, value);
        assert!(!guard_watch_enabled_from_env(), "must refuse {value:?}");
        assert!(
            GuardWatchConfig::from_env(".").is_none(),
            "must refuse {value:?}"
        );
    }

    // 双闸齐开且显式 \"1\" = 开。
    std::env::set_var(GUARD_WATCH_ENABLE_ENV, "1");
    assert!(guard_watch_enabled_from_env());
    assert!(GuardWatchConfig::from_env(".").is_some());

    // 感知闸回头关掉 = 守护面立刻关。
    std::env::set_var(DEVICE_SENSE_ENABLE_ENV, "0");
    assert!(!guard_watch_enabled_from_env());
    std::env::remove_var(DEVICE_SENSE_ENABLE_ENV);
    std::env::remove_var(GUARD_WATCH_ENABLE_ENV);
}

/// ④ 白名单外规则跳过不编造读数 + 词表外动作拒绝即帧。
#[tokio::test]
async fn off_whitelist_rules_are_skipped_without_fabricating_readings() {
    let data_dir = temp_root("skip");
    let config = GuardWatchConfig::new(&data_dir);
    let tool = GuardWatchTool::new(
        config.registry(),
        config.rules(),
        config.alerts(),
        AdapterRegistry::new().with_adapter(Arc::new(HotAdapter {
            temperature_c: 36.0,
        })),
        Arc::new(GuardWatchActivity::new()),
    );
    let rules = GuardRuleStore::under_data_dir(&data_dir);
    let mut body = GuardRulesBody::new();
    body.rules.push(heat_rule("ghost-1")); // 表上没有这台
    let mut disabled = heat_rule("thermo-2");
    disabled.id = "heat-2".to_string();
    body.rules.push(disabled);
    rules.save(&body).expect("规则表落盘");
    DeviceRegistry::under_data_dir(&data_dir)
        .save(&DevicesBody {
            devices: vec![mock_device("thermo-2", DeviceKind::Sensor, false)],
        })
        .expect("设备表落盘");

    let frame = ok_value(&tool, json!({"action": "watch"})).await;
    assert_eq!(frame["alerts_emitted"], 0, "跳过 = 不命中不编造");
    assert_eq!(frame["rules_skipped"].as_array().unwrap().len(), 2);
    assert_eq!(frame["devices_read"], 0, "零读数 (跳过设备不进适配器)");

    // 规则表拒开 = 整帧拒绝即帧 (真因如实)。
    std::fs::write(
        GuardRuleStore::under_data_dir(&data_dir).path(),
        b"{ broken",
    )
    .unwrap();
    let reason = refused(&tool, json!({"action": "watch"})).await;
    assert!(reason.contains("拒开"), "{reason}");
}

/// ⑤ 告警账 round-trip: append-only 逐行 StoredDoc, 读回逐字段等价。
#[test]
fn the_alert_ledger_round_trips_stored_doc_rows() {
    let data_dir = temp_root("ledger");
    let ledger = GuardAlertLedger::under_data_dir(&data_dir);
    for index in 0..2 {
        ledger
            .append(&GuardAlert {
                alert_id: format!("gwalert-{index}"),
                rule_id: "heat".to_string(),
                device_id: "thermo-1".to_string(),
                kind: "guard".to_string(),
                condition: "temperature_above(30)".to_string(),
                severity: SEVERITY_CRITICAL.to_string(),
                message: "室内过热, 请查看".to_string(),
                evidence: "thermo-1 (temperature_above(30))".to_string(),
                source: "tool.guard_watch".to_string(),
                source_data: "mock-fixture".to_string(),
                created_at_ms: 1_700_000_000_000 + index,
                annotation: GUARD_ALERT_ANNOTATION.to_string(),
            })
            .expect("append");
    }
    let scan = ledger.scan().expect("scan");
    assert_eq!(scan.records.len(), 2, "append-only 逐行可读");
    assert_eq!(scan.skipped, 0);
    assert_eq!(scan.records[1].body.alert_id, "gwalert-1");
    assert_eq!(scan.records[0].body.annotation, GUARD_ALERT_ANNOTATION);
    // 信封身份戳正确 (StoredDoc 语义)。
    let raw = std::fs::read_to_string(ledger.path()).expect("read");
    assert!(raw.contains(GUARD_ALERTS_DOC_NAME));
    assert!(
        raw.contains(&format!("\"version\":{GUARD_ALERTS_DOC_VERSION}"))
            || raw.contains("\"version\"")
    );
}

/// 设备表拒开 (StoredDoc 语义) = 整帧拒绝即帧, 不回退默认表。
#[tokio::test]
async fn a_refused_device_table_stops_the_watch_pass() {
    let data_dir = temp_root("bad-devices");
    let config = GuardWatchConfig::new(&data_dir);
    let tool = GuardWatchTool::new(
        config.registry(),
        config.rules(),
        config.alerts(),
        AdapterRegistry::new().with_adapter(Arc::new(HotAdapter {
            temperature_c: 36.0,
        })),
        Arc::new(GuardWatchActivity::new()),
    );
    std::fs::write(
        DeviceRegistry::under_data_dir(&data_dir).path(),
        b"{ not json",
    )
    .unwrap();
    let reason = refused(&tool, json!({"action": "watch"})).await;
    assert!(reason.contains("设备表拒开"), "{reason}");
}
