//! SG-25 · 工具生态健康 → 生物机制自述格 (`ecosystem_health`) 的装配级 E2E。
//!
//! 覆盖装配契约（不是内核单测的重复）:
//! 1. 默认关：自述开关关 → `capabilities.bio_mechanisms` = disabled +
//!    `mechanisms: null`，一个 probe 都不调用（迟绑定槽不武装、剖面不填）；
//! 2. 开关开：装配末端把静态工具能力表填进迟绑定槽，自述帧的
//!    `ecosystem_health` 格 `wired: true` 且值 = 纯观测 JSON：覆盖度、
//!    内核三项分数、重叠对、策展表版本 —— 未收录工具如实计入 `untagged`；
//! 3. 分数只在已收录工具上计算（`tool_count` = tagged，不把未收录洗成低多样性）；
//! 4. 只观测不剪枝：读数里没有任何删除/注销字段；
//! 5. 开关只认字面 `"1"`（`" 1 "` / `"true"` 等一律关，fail-closed）。
//!
//! **0 装诚实**：快照 = 装配期静态注册面；运行期动态发现的 MCP 工具不在其中
//! （`taxonomy_version` 与 `untagged` 暴露覆盖不全），无 live E2E（无 CLI 手测）。

use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_core::clock::VirtualClock;
use apeireth_core::kernel::{Clock, Timestamp};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    BuildInstrumentation, CountingBuildProbe, ModuleBuildProbe, ProductionBackends,
    ProductionModules, ProductionModulesConfig,
};
use apeireth_tools_canonical::self_status::{
    BIO_SELF_STATUS_ENABLE_ENV, CAPABILITIES_BIO_MECHANISMS,
};
use serde_json::{json, Value};

const NOW_MS: i64 = 1_700_000_000_123;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct EnvSwitch;

impl EnvSwitch {
    fn set(value: &str) -> Self {
        std::env::set_var(BIO_SELF_STATUS_ENABLE_ENV, value);
        Self
    }

    fn unset() -> Self {
        std::env::remove_var(BIO_SELF_STATUS_ENABLE_ENV);
        Self
    }
}

impl Drop for EnvSwitch {
    fn drop(&mut self) {
        std::env::remove_var(BIO_SELF_STATUS_ENABLE_ENV);
    }
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(NOW_MS)
            .expect("valid fixed timestamp")
            .as_datetime(),
    ))
}

fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

fn build() -> ProductionModules {
    let probe: Arc<dyn ModuleBuildProbe> = CountingBuildProbe::new();
    let instrument = BuildInstrumentation::default().with_probe(probe);
    ProductionModules::build_instrumented(
        base_config(),
        ProductionBackends::default(),
        clock(),
        &instrument,
    )
    .expect("装配必须成功")
}

fn find_capability<'a>(
    modules: &'a ProductionModules,
    id: &str,
) -> Option<&'a Arc<dyn ToolCapability>> {
    modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == id)
}

fn self_status_call() -> ToolCall {
    ToolCall {
        id: "call_tool_ecosystem_self_status".into(),
        name: "self_status".into(),
        arguments: json!({}),
    }
}

/// 走真实自述工具入口取一帧 JSON。
async fn self_status_frame(modules: &ProductionModules) -> Value {
    let tool = find_capability(modules, "tool.self_status").expect("自述工具必须在装配面注册");
    let result = tool.invoke(&self_status_call()).await;
    assert!(result.is_ok(), "整帧必须成功: {}", result.render());
    serde_json::from_str(&result.render()).expect("结构化自述 JSON")
}

#[tokio::test]
async fn the_default_assembly_keeps_the_ecosystem_mechanism_disabled() {
    let _guard = env_lock();
    let _env = EnvSwitch::unset();
    let modules = build();
    let frame = self_status_frame(&modules).await;
    let bio = &frame["capabilities"][CAPABILITIES_BIO_MECHANISMS];
    assert_eq!(bio["enabled"], json!(false));
    assert_eq!(bio["mechanisms"], Value::Null, "关时一格也不采");
    let reason = bio["reason"].as_str().expect("reason");
    assert!(reason.contains("未启用"), "{reason}");
}

#[tokio::test]
async fn the_switch_reads_the_static_tool_ecosystem_read_only() {
    let _guard = env_lock();
    let _env = EnvSwitch::set("1");
    let modules = build();
    let registered = modules.capabilities().len();
    assert!(registered > 0, "生产装配面至少应有自述工具: {registered}");
    let frame = self_status_frame(&modules).await;
    let row = &frame["capabilities"][CAPABILITIES_BIO_MECHANISMS]["mechanisms"]["ecosystem_health"];
    assert_eq!(row["stage"], json!("wired"), "{row}");
    assert_eq!(row["wired"], json!(true), "{row}");
    let value = &row["value"];
    assert_eq!(value["taxonomy_version"], json!(1), "{value}");
    assert_eq!(
        value["registered"].as_u64().expect("registered"),
        registered as u64,
        "快照必须覆盖装配面全部静态能力"
    );
    let tagged = value["tagged"].as_u64().expect("tagged");
    assert!(tagged > 0, "至少自述工具本身应命中策展表: {value}");
    assert!(
        tagged <= registered as u64,
        "已收录数不得超过注册总数: {value}"
    );
    assert_eq!(
        value["untagged"].as_u64().expect("untagged"),
        registered as u64 - tagged,
        "未收录数必须与注册/已收录一致: {value}"
    );
    assert_eq!(
        value["tool_count"].as_u64().expect("tool_count"),
        tagged,
        "内核分数只覆盖已收录工具: {value}"
    );
    assert!(
        value["coverage"].as_f64().expect("coverage") > 0.0,
        "覆盖率必须 > 0: {value}"
    );
    assert!(value["diversity_score"].is_f64(), "{value}");
    assert!(value["dysbiosis_score"].is_f64(), "{value}");
    assert!(value["alert"].is_boolean(), "{value}");
    assert!(value["overlap_pairs"].is_array(), "{value}");
    assert!(value["recommendations"].is_array(), "{value}");
    // 只观测不剪枝：读数里没有写动作字段。
    for forbidden in ["removed", "pruned", "deleted", "unregistered", "mutated"] {
        assert!(
            value.get(forbidden).is_none(),
            "读数不得含写动作 {forbidden}"
        );
    }
    // 阈值取自内核默认值（不是另编一套）。
    assert_eq!(
        value["thresholds"]["overlap"]
            .as_f64()
            .expect("overlap threshold"),
        0.8
    );
    assert_eq!(
        value["thresholds"]["dysbiosis_alert"]
            .as_f64()
            .expect("dysbiosis threshold"),
        0.6
    );
}

#[tokio::test]
async fn non_literal_switch_values_stay_off() {
    let _guard = env_lock();
    for raw in ["0", "true", "yes", " 1", "1 "] {
        let _env = EnvSwitch::set(raw);
        let modules = build();
        let frame = self_status_frame(&modules).await;
        let bio = &frame["capabilities"][CAPABILITIES_BIO_MECHANISMS];
        assert_eq!(bio["enabled"], json!(false), "开关 {raw:?} 必须关");
        assert_eq!(bio["mechanisms"], Value::Null, "开关 {raw:?} 必须零采集");
    }
}
