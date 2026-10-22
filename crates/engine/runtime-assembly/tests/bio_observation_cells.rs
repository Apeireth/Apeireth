//! Wave 4 · 生物机制自述 14 格观测补格（调质 / 昼夜节律 / 仪式）的装配级 E2E。
//!
//! 覆盖装配契约（不是内核单测的重复）:
//! 1. 默认关：自述开关关 → `capabilities.bio_mechanisms` = disabled +
//!    `mechanisms: null`，迟绑定槽不武装、一个 probe 都不调用；
//! 2. 全开：装配根在模块安装后用同一份生产句柄填槽 →
//!    `neuromodulation` / `circadian` / `ritual` 三格 `wired: true` 且带真值；
//!    本装配观测面 wired 计数 = 12/14（未配任务板 ⇒ 承载力格不注册；练习开关未开），
//!    其余格如实 `wired:false` + 真因（上下文装配格已注册但策略开关关 ⇒ 单格 null）；
//! 3. 只读：三格读数不含写动作字段，且 `tool.ritual_done` 账本在采帧前后字节不变；
//! 4. 开关只认字面 `"1"`（`" 1 "` / `"true"` 等一律关，fail-closed）。
//!
//! **0 装诚实**：本测试不自带生产服务进程（无 live E2E / CLI 手测）；调质/仪式取值
//! 只覆盖「装配根交付句柄 → 只读帧」这一跳，不证明运行期语义效果。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
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
const TONE_ENV: &str = "APEIRETH_ENABLE_BIO_TONE";
const CARE_CANDIDATES_ENV: &str = "APEIRETH_ENABLE_BIO_CARE_CANDIDATES";
const CONTEXT_POLICY_ENV: &str = "APEIRETH_ENABLE_BIO_CONTEXT_POLICY";
const REPAIR_GUIDANCE_ENV: &str = "APEIRETH_ENABLE_BIO_REPAIR_GUIDANCE";

static ENV_LOCK: Mutex<()> = Mutex::new(());
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 三个开关一起设/清（Drop 清理全部三项，测试间互不污染）。
struct EnvSwitches;

impl EnvSwitches {
    fn all_on() -> Self {
        std::env::set_var(BIO_SELF_STATUS_ENABLE_ENV, "1");
        std::env::set_var(TONE_ENV, "1");
        std::env::set_var(CARE_CANDIDATES_ENV, "1");
        std::env::set_var(CONTEXT_POLICY_ENV, "1");
        std::env::set_var(REPAIR_GUIDANCE_ENV, "1");
        Self
    }

    fn all_off() -> Self {
        std::env::remove_var(BIO_SELF_STATUS_ENABLE_ENV);
        std::env::remove_var(TONE_ENV);
        std::env::remove_var(CARE_CANDIDATES_ENV);
        std::env::remove_var(CONTEXT_POLICY_ENV);
        std::env::remove_var(REPAIR_GUIDANCE_ENV);
        Self
    }
}

impl Drop for EnvSwitches {
    fn drop(&mut self) {
        std::env::remove_var(BIO_SELF_STATUS_ENABLE_ENV);
        std::env::remove_var(TONE_ENV);
        std::env::remove_var(CARE_CANDIDATES_ENV);
        std::env::remove_var(CONTEXT_POLICY_ENV);
        std::env::remove_var(REPAIR_GUIDANCE_ENV);
    }
}

fn care_dir() -> PathBuf {
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("apeireth-bio-obs-{}-{seq}", std::process::id()))
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(NOW_MS)
            .expect("valid fixed timestamp")
            .as_datetime(),
    ))
}

fn base_config(care: Option<PathBuf>) -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        care_schedule: care,
        context_budget_chars: Some(24_000),
        ..ProductionModulesConfig::default()
    }
}

fn build(config: ProductionModulesConfig) -> ProductionModules {
    let probe: Arc<dyn ModuleBuildProbe> = CountingBuildProbe::new();
    let instrument = BuildInstrumentation::default().with_probe(probe);
    ProductionModules::build_instrumented(
        config,
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

fn call(name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: format!("call_bio_obs_{name}"),
        name: name.to_string(),
        arguments,
    }
}

async fn self_status_frame(modules: &ProductionModules) -> Value {
    let tool = find_capability(modules, "tool.self_status").expect("自述工具必须在装配面注册");
    let result = tool.invoke(&call("self_status", json!({}))).await;
    assert!(result.is_ok(), "整帧必须成功: {}", result.render());
    serde_json::from_str(&result.render()).expect("结构化自述 JSON")
}

fn mechanism_rows(frame: &Value) -> &serde_json::Map<String, Value> {
    frame["capabilities"][CAPABILITIES_BIO_MECHANISMS]["mechanisms"]
        .as_object()
        .expect("14 格在场")
}

#[tokio::test]
async fn the_default_assembly_keeps_the_three_new_cells_disabled() {
    let _guard = env_lock();
    let _env = EnvSwitches::all_off();
    let modules = build(base_config(None));
    let frame = self_status_frame(&modules).await;
    let bio = &frame["capabilities"][CAPABILITIES_BIO_MECHANISMS];
    assert_eq!(bio["enabled"], json!(false));
    assert_eq!(bio["mechanisms"], Value::Null, "关时一格也不采");
}

#[tokio::test]
async fn the_three_new_cells_read_real_production_state_and_never_write() {
    let _guard = env_lock();
    let _env = EnvSwitches::all_on();
    let dir = care_dir();
    let modules = build(base_config(Some(dir.clone())));

    // 先走真实工具面写一笔仪式账（工具 = 唯一写者），随后只读采帧。
    let done = find_capability(&modules, "tool.ritual_done").expect("仪式工具必须在装配面注册");
    let result = done
        .invoke(&call("ritual_done", json!({"kind": "morning_check_in"})))
        .await;
    assert!(result.is_ok(), "写入必须成功: {}", result.render());
    let ledger = dir.join("ritual_state.json");
    let before = std::fs::read(&ledger).expect("账本应已落盘");

    // SG-20 事件源：真实总线发布一次「人工拒绝」→ 社会修复观测源折算修复计划。
    use apeireth_runtime_assembly::canonical::{ApprovalSocialOutcome, NervousSignal};
    let delivered = modules
        .nervous_bus()
        .publish(&NervousSignal::approval_social(
            ApprovalSocialOutcome::Rejected,
            "cli.approval",
        ));
    assert!(delivered >= 1, "拒绝信号必须至少投递给社会修复订阅者");

    let frame = self_status_frame(&modules).await;
    let rows = mechanism_rows(&frame);

    let neuromodulation = &rows["neuromodulation"];
    assert_eq!(neuromodulation["wired"], json!(true), "{neuromodulation}");
    let value = &neuromodulation["value"];
    assert!(value["vector"]["dopamine"].is_f64(), "{value}");
    assert!(value["behavior"]["warmth"].is_f64(), "{value}");
    assert_eq!(value["tone_style_is_baseline"], json!(true), "{value}");

    let circadian = &rows["circadian"];
    assert_eq!(circadian["wired"], json!(true), "{circadian}");
    let value = &circadian["value"];
    assert!(value["phase"].is_string(), "{value}");
    assert_eq!(value["timezone_offset_minutes"], json!(0), "{value}");
    assert_eq!(value["gate_enabled"], json!(false), "昼夜行为开关未开");
    assert!(value["sleep_pressure"].is_null(), "压力累加器不在装配根");
    assert!(
        value["sleep_pressure_reason"]
            .as_str()
            .expect("reason")
            .contains("装配根"),
        "{value}"
    );

    let ritual = &rows["ritual"];
    assert_eq!(ritual["wired"], json!(true), "{ritual}");
    let value = &ritual["value"];
    assert_eq!(value["state_count"], json!(1), "{value}");
    assert_eq!(
        value["kinds"]["morning_check_in"]["consecutive_done"],
        json!(1),
        "{value}"
    );
    assert_eq!(value["actionable_count"], json!(0), "{value}");

    // 只观测不剪枝、不写账：三格读数无写动作字段；账本字节前后一致。
    for row in [neuromodulation, circadian, ritual] {
        for forbidden in ["removed", "pruned", "deleted", "mutated", "written"] {
            assert!(row.get(forbidden).is_none(), "读数不得含写动作 {forbidden}");
        }
    }
    let after = std::fs::read(&ledger).expect("账本字节 after");
    assert_eq!(before, after, "自述采帧不得改写仪式账本");

    // 上下文装配格（SG-14）：策略开关开 + 装配根注入预算 → 真值（尚未发生回合）。
    let homeostasis = &rows["homeostasis"];
    assert_eq!(homeostasis["wired"], json!(true), "{homeostasis}");
    let value = &homeostasis["value"];
    assert_eq!(value["context_policy_enabled"], json!(true), "{value}");
    assert_eq!(value["configured_budget_chars"], json!(24_000), "{value}");
    assert_eq!(value["assembled_turns"], json!(0), "{value}");
    assert!(value["last_assembly"].is_null(), "{value}");

    // 社会修复格（SG-20）：人工拒绝 = 真实事件源 → 观测到 1 次摩擦 + 内核修复计划。
    let social_repair = &rows["social_repair"];
    assert_eq!(social_repair["wired"], json!(true), "{social_repair}");
    let value = &social_repair["value"];
    assert_eq!(value["rejections_seen"], json!(1), "{value}");
    assert_eq!(
        value["last_plan"]["moves"],
        json!(["acknowledge_uncertainty", "restate_understanding"]),
        "{value}"
    );
    assert_eq!(value["last_plan"]["ask_user"], json!(false), "{value}");

    let wired = rows
        .values()
        .filter(|row| row["wired"] == json!(true))
        .count();
    assert_eq!(
        wired, 12,
        "观测面 = 12/14：未配任务板（承载力格不注册）+ 练习开关未开 → 14 - 2"
    );
}

#[tokio::test]
async fn non_literal_switch_values_stay_off() {
    let _guard = env_lock();
    for raw in ["0", "true", "yes", " 1", "1 "] {
        let _env = EnvSwitches::all_off();
        std::env::set_var(BIO_SELF_STATUS_ENABLE_ENV, raw);
        let modules = build(base_config(None));
        let frame = self_status_frame(&modules).await;
        let bio = &frame["capabilities"][CAPABILITIES_BIO_MECHANISMS];
        assert_eq!(bio["enabled"], json!(false), "开关 {raw:?} 必须关");
        assert_eq!(bio["mechanisms"], Value::Null, "开关 {raw:?} 必须零采集");
    }
}
