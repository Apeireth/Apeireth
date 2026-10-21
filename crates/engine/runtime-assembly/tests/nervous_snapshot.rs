//! ADR-0003 / R1a：`capabilities.nervous_system` 只读聚合的装配级证明。
//!
//! 钉住：
//! ① schema v1 与字段形状冻结；② `mechanisms` 恰 14 条、id 与契约词表同序唯一；
//! ③ 半程/未接线行 `level` + `gaps` + 信号源 reason 非空；④ `last_observation`
//! 恒 `null` 不伪造；⑤ `summary` 与 mechanisms 同一次采集且计数自洽；⑥ `enabled`
//! 只随既有开关解析函数变化（含 `bio_self_status` 严格 `"1"` 与其余 trim 口径）；
//! ⑦ 序列化往返稳定；⑧ 开关关时装饰器零动作（不读 env、逐字节同旧帧）；
//! ⑨ 开关开时子对象挂在 `capabilities` 内、顶层 21 节不变；⑩ `observed` = 注册
//! 表真实注册 probe 数（注册失败仍算接线）。
//!
//! 与 ADR 措辞的差异：ADR 写「21→22 节」，实际按 `bio_mechanisms` 既有口径落为
//! `capabilities` 内第二个**只读子对象**，顶层节数仍 21（`SELF_STATUS_SECTIONS`）。

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::canonical::nervous_bus::{NervousBus, NervousSignal};
use apeireth_runtime_assembly::canonical::self_status_source::nervous_snapshot::{
    nervous_self_status_tool, snapshot_from_values, snapshot_from_values_with_bus, EnvReader,
    NervousMechanism, NervousSelfStatusTool, NervousSnapshot, NERVOUS_MECHANISM_TOTAL,
    NERVOUS_SNAPSHOT_VERSION, NERVOUS_SYSTEM_SUBOBJECT,
};
use apeireth_runtime_assembly::canonical::self_status_source::ProductionSelfStatusSource;
use apeireth_runtime_assembly::canonical::tool_modules::SelfStatusModule;
use apeireth_runtime_assembly::canonical::CapabilityProvider;
use apeireth_tools_canonical::self_status::{
    BioMechanismKind, BioMechanismProbe, BioMechanismReading, CAPABILITIES_BIO_MECHANISMS,
    SELF_STATUS_SECTIONS,
};
use apeireth_tools_canonical::SelfStatusTool;
use serde_json::{json, Value};

fn call() -> ToolCall {
    ToolCall {
        id: "call_nervous_snapshot".into(),
        name: "self_status".into(),
        arguments: json!({}),
    }
}

async fn plain_frame(source: ProductionSelfStatusSource) -> Value {
    let tool = SelfStatusTool::new(Arc::new(source));
    let result = tool.invoke(&call()).await;
    assert!(result.is_ok(), "整帧必须成功: {}", result.render());
    serde_json::from_str(&result.render()).expect("structured self-status json")
}

async fn decorated_frame(source: ProductionSelfStatusSource, env: EnvReader) -> Value {
    let inner: Arc<dyn ToolCapability> = Arc::new(SelfStatusTool::new(Arc::new(source)));
    let tool = NervousSelfStatusTool::new(inner).with_env(env);
    let result = tool.invoke(&call()).await;
    assert!(result.is_ok(), "整帧必须成功: {}", result.render());
    serde_json::from_str(&result.render()).expect("structured self-status json")
}

async fn decorated_frame_with_bus(
    source: ProductionSelfStatusSource,
    env: EnvReader,
    bus: NervousBus,
) -> Value {
    let inner: Arc<dyn ToolCapability> = Arc::new(SelfStatusTool::new(Arc::new(source)));
    let tool = NervousSelfStatusTool::new(inner)
        .with_env(env)
        .with_bus(bus);
    let result = tool.invoke(&call()).await;
    assert!(result.is_ok(), "整帧必须成功: {}", result.render());
    serde_json::from_str(&result.render()).expect("structured self-status json")
}

fn source_with_face(enabled: bool) -> ProductionSelfStatusSource {
    ProductionSelfStatusSource::new(Vec::new(), false, None).with_bio_self_status(enabled)
}

fn env_values(pairs: &[(&str, &str)]) -> EnvReader {
    let map: BTreeMap<String, String> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    Arc::new(move |name| map.get(name).cloned())
}

/// 全部生物开关显式 `1` 的 env 面（含共享的自述面开关）。
fn env_all_on() -> EnvReader {
    env_values(&[
        ("APEIRETH_ENABLE_BIO_TONE", "1"),
        ("APEIRETH_ENABLE_BIO_CONTEXT_POLICY", "1"),
        ("APEIRETH_ENABLE_BIO_CIRCADIAN", "1"),
        ("APEIRETH_ENABLE_IMMUNE_MEMORY", "1"),
        ("APEIRETH_ENABLE_BIO_PAIN", "1"),
        ("APEIRETH_ENABLE_BIO_CARE_CANDIDATES", "1"),
        ("APEIRETH_ENABLE_BIO_REPAIR_GUIDANCE", "1"),
        ("APEIRETH_ENABLE_QUORUM_GATE", "1"),
        ("APEIRETH_ENABLE_BIO_LIFECYCLE", "1"),
        ("APEIRETH_ENABLE_BIO_PLAY", "1"),
        ("APEIRETH_ENABLE_BIO_SELF_STATUS", "1"),
    ])
}

struct FixedProbe;

impl BioMechanismProbe for FixedProbe {
    fn snapshot(&self, _now_ms: i64) -> Result<BioMechanismReading, String> {
        Ok(BioMechanismReading::wired(
            Some(json!({"count": 1})),
            None,
            None,
        ))
    }
}

struct FailingProbe;

impl BioMechanismProbe for FailingProbe {
    fn snapshot(&self, _now_ms: i64) -> Result<BioMechanismReading, String> {
        Err("probe source unavailable".to_string())
    }
}

fn mechanism<'a>(value: &'a Value, id: &str) -> &'a Value {
    value["mechanisms"]
        .as_array()
        .expect("mechanisms array")
        .iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("missing mechanism {id}"))
}

#[test]
fn snapshot_is_version_two_and_keeps_the_additive_bus_field() {
    let value = snapshot_from_values(|_| None, 0).to_value();
    assert_eq!(value["version"], json!(NERVOUS_SNAPSHOT_VERSION));
    assert_eq!(value["version"], json!(2), "R1b-4a 起 schema v2");
    assert_eq!(
        value
            .as_object()
            .expect("object")
            .keys()
            .collect::<Vec<_>>(),
        vec!["bus", "mechanisms", "summary", "version"],
        "schema v2 顶层字段 = v1 三字段 + bus（additive）"
    );
    assert!(
        value["bus"].is_null(),
        "未绑总线时 bus 必须是 null，不编造计数"
    );
    let summary = value["summary"].as_object().expect("summary object");
    assert_eq!(
        summary.keys().collect::<Vec<_>>(),
        vec!["half", "observed", "production_wired", "total", "unwired"],
        "summary 字段合同"
    );
    let row = &value["mechanisms"][0];
    let keys: Vec<&String> = row.as_object().expect("row").keys().collect();
    assert_eq!(
        keys,
        vec![
            "enabled",
            "enabled_env",
            "gaps",
            "id",
            "last_observation",
            "level",
            "signal_sources",
            "wiring",
        ],
        "机制行字段合同"
    );
    let signal = &row["signal_sources"][0];
    assert_eq!(
        signal
            .as_object()
            .expect("signal")
            .keys()
            .collect::<Vec<_>>(),
        vec!["id", "reason", "state"],
        "信号源字段合同"
    );
}

#[test]
fn mechanisms_are_exactly_fourteen_unique_and_follow_the_contract_order() {
    let value = snapshot_from_values(|_| None, 0).to_value();
    let rows = value["mechanisms"].as_array().expect("rows");
    assert_eq!(rows.len(), NERVOUS_MECHANISM_TOTAL);
    assert_eq!(rows.len(), 14);
    let ids: Vec<&str> = rows
        .iter()
        .map(|row| row["id"].as_str().expect("id"))
        .collect();
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), 14, "id 必须唯一");
    assert_eq!(BioMechanismKind::ALL.len(), 14, "契约词表仍 14 条");
    for (index, kind) in BioMechanismKind::ALL.iter().enumerate() {
        assert!(
            ids[index].ends_with(&format!(".{}", kind.as_str())),
            "行序必须与 BioMechanismKind::ALL 一致: {} vs {}",
            ids[index],
            kind.as_str()
        );
    }
    assert_eq!(ids[0], "SG-13.neuromodulation");
    assert_eq!(ids[13], "SG-26.capacity");
}

#[test]
fn half_rows_expose_gaps_and_reasons_and_wired_rows_are_honest() {
    let value = snapshot_from_values(|_| None, 0).to_value();
    let half = mechanism(&value, "SG-22.ritual");
    assert_eq!(half["level"], json!("HALF"));
    assert!(!half["gaps"].as_array().expect("gaps").is_empty());

    // SG-18 / SG-25 自 R1b-1b 起已是 PRODUCTION WIRED（装配根注册），但仍必须
    // 如实列出残余缺口；不得再出现「装配根注册待 Lead」这类过期占位。
    for id in ["SG-18.lifecycle", "SG-25.ecosystem_health"] {
        let row = mechanism(&value, id);
        assert_eq!(row["level"], json!("PRODUCTION WIRED"), "{id}");
        assert!(!row["gaps"].as_array().expect("gaps").is_empty(), "{id}");
        assert!(
            !row["gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|gap| gap.as_str().unwrap_or_default().contains("待 Lead")),
            "{id} 台账不得保留过期占位"
        );
    }
    assert!(mechanism(&value, "SG-18.lifecycle")["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|gap| gap.as_str().unwrap_or_default().contains("lifecycle probe")));
    assert!(mechanism(&value, "SG-25.ecosystem_health")["enabled_env"].is_null());
}

#[test]
fn last_observation_is_always_null_and_never_fabricated() {
    let value = snapshot_from_values(env_all_on_reader(), 3).to_value();
    for row in value["mechanisms"].as_array().expect("rows") {
        assert!(
            row["last_observation"].is_null(),
            "{} must not fabricate observations",
            row["id"]
        );
    }
}

fn env_all_on_reader() -> impl Fn(&str) -> Option<String> {
    let map: BTreeMap<String, String> = [
        "APEIRETH_ENABLE_BIO_TONE",
        "APEIRETH_ENABLE_BIO_CONTEXT_POLICY",
        "APEIRETH_ENABLE_BIO_CIRCADIAN",
        "APEIRETH_ENABLE_IMMUNE_MEMORY",
        "APEIRETH_ENABLE_BIO_PAIN",
        "APEIRETH_ENABLE_BIO_CARE_CANDIDATES",
        "APEIRETH_ENABLE_BIO_REPAIR_GUIDANCE",
        "APEIRETH_ENABLE_QUORUM_GATE",
        "APEIRETH_ENABLE_BIO_LIFECYCLE",
        "APEIRETH_ENABLE_BIO_PLAY",
        "APEIRETH_ENABLE_BIO_SELF_STATUS",
    ]
    .into_iter()
    .map(|name| (name.to_string(), "1".to_string()))
    .collect();
    move |name: &str| map.get(name).cloned()
}

#[test]
fn summary_counts_match_the_mechanism_rows() {
    let snapshot = snapshot_from_values(|_| None, 1);
    let summary = snapshot.summary;
    assert_eq!(summary.total, NERVOUS_MECHANISM_TOTAL);
    assert_eq!(
        summary.production_wired + summary.half + summary.unwired,
        summary.total,
        "三档之和必须等于总数"
    );
    // R1b-1b：SG-18 / SG-25 由 IMPLEMENTED 更正为 PRODUCTION WIRED（装配根已注册，
    // 残余缺口仍在 gaps 如实列出），故 production_wired 13 / half 1 / unwired 0。
    assert_eq!(summary.production_wired, 13);
    assert_eq!(summary.half, 1);
    assert_eq!(summary.unwired, 0);
    assert_eq!(summary.observed, 1);
    let value = snapshot.to_value();
    assert_eq!(value["summary"]["total"], json!(summary.total));
    assert_eq!(
        value["summary"]["production_wired"],
        json!(summary.production_wired)
    );
    assert_eq!(value["summary"]["half"], json!(summary.half));
    assert_eq!(value["summary"]["unwired"], json!(summary.unwired));
    assert_eq!(value["summary"]["observed"], json!(summary.observed));
    // 观察数不超过总数（注册表不存在时 = 0）。
    let zero = snapshot_from_values(|_| None, 0);
    assert_eq!(zero.summary.observed, 0);
    let capped = snapshot_from_values(|_| None, 99);
    assert_eq!(capped.summary.observed, NERVOUS_MECHANISM_TOTAL);
}

#[test]
fn enabled_only_reflects_the_existing_switch_parsers() {
    let off = snapshot_from_values(|_| None, 0);
    assert!(off.mechanisms.iter().all(|row| !row.enabled));

    let on = snapshot_from_values(env_all_on_reader(), 0);
    let enabled: Vec<&str> = on
        .mechanisms
        .iter()
        .filter(|row| row.enabled)
        .map(|row| row.id.as_str())
        .collect();
    assert_eq!(enabled.len(), 13, "14 条中 13 条有 env 开关: {enabled:?}");
    assert!(!mechanism_row(&on, "SG-25.ecosystem_health").enabled);

    // 既有解析函数口径: 自述面严格 "1"（不 trim），其余 trim 后等于 "1"。
    let strict = snapshot_from_values(
        |name| match name {
            "APEIRETH_ENABLE_BIO_SELF_STATUS" => Some(" 1 ".to_string()),
            "APEIRETH_ENABLE_BIO_TONE" => Some(" 1 ".to_string()),
            _ => None,
        },
        0,
    );
    assert!(!mechanism_row(&strict, "SG-26.capacity").enabled);
    assert!(mechanism_row(&strict, "SG-13.neuromodulation").enabled);

    // 非 "1" 的怪值一律关（fail-closed）。
    let weird = snapshot_from_values(|_| Some("true".to_string()), 0);
    assert!(weird.mechanisms.iter().all(|row| !row.enabled));
}

fn mechanism_row<'a>(snapshot: &'a NervousSnapshot, id: &str) -> &'a NervousMechanism {
    snapshot
        .mechanisms
        .iter()
        .find(|row| row.id == id)
        .unwrap_or_else(|| panic!("missing mechanism {id}"))
}

#[test]
fn serialization_roundtrips_stably() {
    let first = snapshot_from_values(env_all_on_reader(), 2);
    let value = first.to_value();
    let back: NervousSnapshot = serde_json::from_value(value.clone()).expect("roundtrip");
    assert_eq!(first, back);
    assert_eq!(value, back.to_value());
    assert_eq!(snapshot_from_values(env_all_on_reader(), 2), first);
}

#[tokio::test]
async fn decoration_is_a_noop_and_reads_no_env_when_the_bio_face_is_off() {
    let plain = plain_frame(source_with_face(false)).await;
    let decorated = decorated_frame(
        source_with_face(false),
        Arc::new(|_| {
            panic!("开关关时必须不读 env");
        }),
    )
    .await;
    assert_eq!(
        serde_json::to_string(&plain).unwrap(),
        serde_json::to_string(&decorated).unwrap(),
        "开关关时旧帧逐字节不变"
    );
    assert!(decorated["capabilities"]
        .get(NERVOUS_SYSTEM_SUBOBJECT)
        .is_none());
}

#[tokio::test]
async fn decoration_adds_the_subobject_under_capabilities_only() {
    let source = source_with_face(true);
    let plain = plain_frame(source_with_face(true)).await;
    let decorated = decorated_frame(source, env_all_on()).await;
    assert_eq!(SELF_STATUS_SECTIONS.len(), 21, "顶层 21 节合同冻结");
    assert!(!SELF_STATUS_SECTIONS.contains(&NERVOUS_SYSTEM_SUBOBJECT));
    assert!(decorated.get(NERVOUS_SYSTEM_SUBOBJECT).is_none());
    let nervous = &decorated["capabilities"][NERVOUS_SYSTEM_SUBOBJECT];
    assert_eq!(nervous["version"], json!(2));
    assert!(nervous["bus"].is_null(), "未绑总线时 bus 为 null");
    assert_eq!(nervous["mechanisms"].as_array().unwrap().len(), 14);
    assert_eq!(nervous["summary"]["total"], json!(14));
    assert_eq!(nervous["summary"]["observed"], json!(0));
    // 既有 `bio_mechanisms` 子对象不被装饰器改写。
    assert_eq!(
        plain["capabilities"][CAPABILITIES_BIO_MECHANISMS],
        decorated["capabilities"][CAPABILITIES_BIO_MECHANISMS]
    );
    // 其余 20 节内容不变；`capabilities` 节只新增只读子对象。
    for section in SELF_STATUS_SECTIONS {
        if *section == "capabilities" {
            continue;
        }
        assert_eq!(plain[section], decorated[section], "节 {section} 必须不变");
    }
    for (name, value) in decorated["capabilities"].as_object().unwrap() {
        if name == CAPABILITIES_BIO_MECHANISMS || name == NERVOUS_SYSTEM_SUBOBJECT {
            continue;
        }
        assert_eq!(
            plain["capabilities"][name], *value,
            "能力开关 {name} 必须不变"
        );
    }
}

#[tokio::test]
async fn observed_counts_really_registered_probes_even_when_they_fail() {
    let mut source = source_with_face(true);
    source.register_bio_probe(BioMechanismKind::Capacity, Arc::new(FixedProbe));
    source.register_bio_probe(BioMechanismKind::Nociception, Arc::new(FailingProbe));
    let value = decorated_frame(source, env_all_on()).await;
    let nervous = &value["capabilities"][NERVOUS_SYSTEM_SUBOBJECT];
    assert_eq!(
        nervous["summary"]["observed"],
        json!(2),
        "注册即接线: 读失败仍计入 observed"
    );
    let rows = &value["capabilities"][CAPABILITIES_BIO_MECHANISMS]["mechanisms"];
    assert_eq!(rows["capacity"]["wired"], json!(true));
    assert_eq!(rows["nociception"]["wired"], json!(true));
    assert_eq!(rows["nociception"]["value"], Value::Null);

    let none = decorated_frame(source_with_face(true), env_all_on()).await;
    assert_eq!(
        none["capabilities"][NERVOUS_SYSTEM_SUBOBJECT]["summary"]["observed"],
        json!(0)
    );
}

#[tokio::test]
async fn self_status_module_uses_the_nervous_decorator() {
    let module = SelfStatusModule::new(Arc::new(source_with_face(true)));
    let capabilities = module.capabilities();
    assert_eq!(capabilities.len(), 1);
    let result = capabilities[0].invoke(&call()).await;
    assert!(result.is_ok());
    let value: Value = serde_json::from_str(&result.render()).expect("json");
    assert_eq!(
        value["capabilities"][NERVOUS_SYSTEM_SUBOBJECT]["version"],
        json!(2)
    );
    assert!(value["capabilities"][NERVOUS_SYSTEM_SUBOBJECT]["bus"].is_null());
    // 工具 id/声明保持不变（装饰器只改帧，不改工具身份）。
    assert_eq!(capabilities[0].id().as_str(), "tool.self_status");
    assert_eq!(
        nervous_self_status_tool(Arc::new(source_with_face(true)))
            .id()
            .as_str(),
        "tool.self_status"
    );
}

#[tokio::test]
async fn bus_observation_reports_real_counts_and_never_fabricates_last_observation() {
    let bus = NervousBus::new();
    let received = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = Arc::clone(&received);
    let _ticket = bus.subscribe(
        apeireth_runtime_assembly::canonical::nervous_bus::NervousSignalKind::ToolOutcome,
        move |_signal: &NervousSignal| {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        },
    );
    let _ = bus.publish(&NervousSignal::tool_outcome(
        true,
        Some("tool.self_status"),
        false,
    ));
    let _ = bus.publish(&NervousSignal::error_observed("boom"));
    assert_eq!(received.load(std::sync::atomic::Ordering::SeqCst), 1);

    let value = decorated_frame_with_bus(source_with_face(true), env_all_on(), bus).await;
    let bus_status = &value["capabilities"][NERVOUS_SYSTEM_SUBOBJECT]["bus"];
    assert_eq!(bus_status["subscribers"], json!(1));
    assert_eq!(bus_status["published"], json!(2));
    assert_eq!(bus_status["delivered"], json!(1));
    assert!(
        bus_status["last_observation"].is_null(),
        "总线不保留历史载荷，last_observation 必须为 null"
    );
}

#[tokio::test]
async fn self_status_module_with_bus_attaches_the_bus_face() {
    let module =
        SelfStatusModule::new_with_bus(Arc::new(source_with_face(true)), NervousBus::new());
    let capabilities = module.capabilities();
    assert_eq!(capabilities.len(), 1);
    let result = capabilities[0].invoke(&call()).await;
    assert!(result.is_ok());
    let value: Value = serde_json::from_str(&result.render()).expect("json");
    let nervous = &value["capabilities"][NERVOUS_SYSTEM_SUBOBJECT];
    assert_eq!(nervous["version"], json!(2));
    assert_eq!(nervous["bus"]["subscribers"], json!(0));
    assert_eq!(nervous["bus"]["published"], json!(0));
    assert_eq!(nervous["bus"]["delivered"], json!(0));
}
