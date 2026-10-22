//! 生物机制族 W3.7 · 仪式完成面装配级接线测试
//! (`tool.ritual_done` / `tool.ritual_skip`, 默认关)。
//!
//! 覆盖点:
//! - 默认关: 装配根**不注册**两工具, 不构造存储, 不创建 `ritual_state.json`;
//! - env 开 + 无关怀数据目录: 仍**不注册** (无处落盘, fail-closed);
//! - env 开 + 数据目录注入: 恰好注册两个工具, 与单元面同 id / 同模型名;
//! - 注册后的真实调用写 `<care_dir>/ritual_state.json` (完成/跳过两路), 且
//!   装配本身零文件副作用、零 memory 副作用。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_core::clock::VirtualClock;
use apeireth_core::kernel::{Clock, Timestamp};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
use apeireth_runtime_assembly::canonical::{
    RITUAL_DONE_CAPABILITY_ID, RITUAL_DONE_TOOL_NAME, RITUAL_SKIP_CAPABILITY_ID,
    RITUAL_SKIP_TOOL_NAME,
};
use apeireth_runtime_assembly::{
    BuildInstrumentation, CountingBuildProbe, ModuleBuildProbe, ProductionBackends,
    ProductionModules, ProductionModulesConfig,
};
use serde_json::{json, Value};

/// 注入时钟的固定时刻 (测试永不读系统时钟)。
const NOW_MS: i64 = 1_700_000_000_123;

/// 关怀数据目录 env 名 (与 `care_reminder::BIO_CARE_CANDIDATES_ENABLE_ENV` 同值;
/// 这里用字面量避免对未导出常量的依赖, 测试同时断言两面一致由单元面覆盖)。
const CARE_CANDIDATES_ENV: &str = "APEIRETH_ENABLE_BIO_CARE_CANDIDATES";

/// 进程 env 只在这些装配测试里改; 全文件共用一把锁。
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// env 开关守卫: Drop 时清掉, panic 也不污染后续测试。
struct EnvSwitch;

impl EnvSwitch {
    fn set(value: &str) -> Self {
        std::env::set_var(CARE_CANDIDATES_ENV, value);
        Self
    }

    fn unset() -> Self {
        std::env::remove_var(CARE_CANDIDATES_ENV);
        Self
    }
}

impl Drop for EnvSwitch {
    fn drop(&mut self) {
        std::env::remove_var(CARE_CANDIDATES_ENV);
    }
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(NOW_MS)
            .expect("valid fixed timestamp")
            .as_datetime(),
    ))
}

/// 每个测试独立的数据目录 (先清空, 避免上次残留)。
fn care_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("apeireth-bio-ritual-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建关怀数据目录");
    dir
}

fn base_config(care: Option<PathBuf>) -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        care_schedule: care,
        ..ProductionModulesConfig::default()
    }
}

fn build(config: ProductionModulesConfig) -> ProductionModules {
    let probe = Arc::new(CountingBuildProbe::default());
    let concrete = Arc::clone(&probe);
    let probe: Arc<dyn ModuleBuildProbe> = concrete;
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
        id: format!("call-{name}"),
        name: name.to_string(),
        arguments,
    }
}

fn ok_value(frame: &apeireth_protocol::canonical::ToolResult) -> Value {
    match &frame.outcome {
        ToolOutcome::Ok { value } => value.clone(),
        other => panic!("expected an ok frame, got {other:?}"),
    }
}

fn ritual_ids(modules: &ProductionModules) -> Vec<String> {
    modules
        .capabilities()
        .iter()
        .map(|capability| capability.id().as_str().to_string())
        .filter(|id| id == RITUAL_DONE_CAPABILITY_ID || id == RITUAL_SKIP_CAPABILITY_ID)
        .collect()
}

#[test]
fn the_default_assembly_registers_no_ritual_tools_and_touches_no_file() {
    let _lock = env_lock();
    let _env = EnvSwitch::unset();
    let dir = care_dir("default-off");

    let modules = build(base_config(Some(dir.clone())));

    assert!(ritual_ids(&modules).is_empty(), "默认关 = 零注册");
    assert!(
        !dir.join("ritual_state.json").exists(),
        "装配不创建仪式状态文件"
    );
}

#[test]
fn the_env_switch_without_a_care_dir_registers_nothing() {
    let _lock = env_lock();
    let _env = EnvSwitch::set("1");

    let modules = build(base_config(None));

    assert!(
        ritual_ids(&modules).is_empty(),
        "缺数据目录 = 无处落盘 = 不注册 (fail-closed)"
    );
}

#[tokio::test]
async fn the_env_switch_registers_exactly_two_tools_that_write_the_care_file() {
    let _lock = env_lock();
    let _env = EnvSwitch::set("1");
    let dir = care_dir("wired-on");
    let file = dir.join("ritual_state.json");

    let modules = build(base_config(Some(dir.clone())));
    let ids = ritual_ids(&modules);
    assert_eq!(
        ids,
        vec![
            RITUAL_DONE_CAPABILITY_ID.to_string(),
            RITUAL_SKIP_CAPABILITY_ID.to_string()
        ],
        "恰好两个仪式工具"
    );
    assert!(!file.exists(), "装配本身不落盘");

    let done = find_capability(&modules, RITUAL_DONE_CAPABILITY_ID)
        .expect("完成工具已注册")
        .clone();
    assert_eq!(done.declaration().name, RITUAL_DONE_TOOL_NAME);
    let frame = done
        .invoke(&call(
            RITUAL_DONE_TOOL_NAME,
            json!({"kind": "morning_check_in"}),
        ))
        .await;
    assert!(frame.is_ok(), "注册后的完成调用必须成功: {frame:?}");
    let body = ok_value(&frame);
    assert_eq!(body["last_done_ms"], NOW_MS as u64);
    assert_eq!(body["consecutive_done"], 1);

    let skip = find_capability(&modules, RITUAL_SKIP_CAPABILITY_ID)
        .expect("跳过工具已注册")
        .clone();
    assert_eq!(skip.declaration().name, RITUAL_SKIP_TOOL_NAME);
    let frame = skip
        .invoke(&call(
            RITUAL_SKIP_TOOL_NAME,
            json!({"kind": "evening_wrap"}),
        ))
        .await;
    assert!(frame.is_ok(), "注册后的跳过调用必须成功: {frame:?}");
    assert_eq!(ok_value(&frame)["skipped_count"], 1);

    // 真源是文件本身: 信封身份 + 两条状态, 新读取面一致。
    assert!(file.exists(), "调用后必须落盘");
    let raw = std::fs::read_to_string(&file).expect("读文件");
    let value: Value = serde_json::from_str(&raw).expect("信封 JSON");
    assert_eq!(value["name"], "apeireth.ritual_state");
    assert_eq!(value["version"], 1);
    assert_eq!(value["body"]["states"].as_array().map(Vec::len), Some(2));
}

#[test]
fn the_switch_is_fail_closed_for_non_exact_values() {
    let _lock = env_lock();
    let dir = care_dir("fail-closed");
    for value in ["0", "true", "yes", " 1", "1 "] {
        let _env = EnvSwitch::set(value);
        let modules = build(base_config(Some(dir.clone())));
        assert!(
            ritual_ids(&modules).is_empty(),
            "{value:?} 必须关 (只认精确 \"1\")"
        );
    }
    assert!(
        !Path::new(&dir).join("ritual_state.json").exists(),
        "全关期间零文件"
    );
}

#[test]
fn the_ritual_tool_ids_are_classified_as_write_or_execute_for_read_only_presets() {
    use apeireth_core::kernel::CapabilityId;
    use apeireth_runtime_assembly::canonical::is_write_or_execute_capability;

    for id in [RITUAL_DONE_CAPABILITY_ID, RITUAL_SKIP_CAPABILITY_ID] {
        let id = CapabilityId::new(id).expect("static id");
        assert!(
            is_write_or_execute_capability(&id),
            "{id} 是本地写面, 只读预设必须拒绝"
        );
    }
}
