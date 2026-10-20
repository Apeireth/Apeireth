//! 生物机制族 W3.5 · 生命周期模块在**生产装配根**的注册测试（SG-18）。
//!
//! 覆盖装配级契约（不是 `bio_lifecycle_wiring.rs` 内核/静态接线的重复）：
//! ① 默认关：`ProductionModules` 不注册 `bio_lifecycle`、无状态句柄、零构造；
//! ② env 只认 `"1"`：其余值（空 / `0` / `true` / `on` / 混合）一律 fail-closed；
//! ③ env 开：恰好注册一个模块，getter 与模块共享同一 `Arc<BioLifecycleState>`；
//! ④ 种子：装配根真实工具声明名以 `Newborn` 入账（与能力面同一份名字）；
//! ⑤ 诚实边界：生命周期不新增 `ModuleFamily` 变体，构造探针零上报。
//!
//! **0 装诚实**：状态仅进程内（无持久化）；`bio_lifecycle_probe` 未接入
//! self_status（Lead 后续）；无 live E2E；开关关 = 缺省。

use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_core::kernel::{system_clock, Clock};
use apeireth_orchestration::lifecycle::{AssetKind, LifecycleStage};
use apeireth_runtime_assembly::canonical::care_heartbeat::bio_lifecycle::{
    BioLifecycleState, BIO_LIFECYCLE_ENABLE_ENV, BIO_LIFECYCLE_MODULE_ID,
};
use apeireth_runtime_assembly::{
    BuildInstrumentation, CountingBuildProbe, JudgeConfig, ProductionBackends, ProductionModules,
    ProductionModulesConfig,
};

/// 进程 env 只在装配期读：全文件共用一把锁，只在同步装配段持有。
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// env 开关守卫：`Drop` 时清掉，panic 也不污染后续测试。
struct EnvSwitch;

impl EnvSwitch {
    fn set(value: &str) -> Self {
        std::env::set_var(BIO_LIFECYCLE_ENABLE_ENV, value);
        Self
    }

    fn unset() -> Self {
        std::env::remove_var(BIO_LIFECYCLE_ENABLE_ENV);
        Self
    }
}

impl Drop for EnvSwitch {
    fn drop(&mut self) {
        std::env::remove_var(BIO_LIFECYCLE_ENABLE_ENV);
    }
}

fn clock() -> Arc<dyn Clock> {
    system_clock()
}

/// 全默认开族显式关：空后端也能装配（只留本批被测的 env 门）。
fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        judge: JudgeConfig {
            enabled: false,
            ..JudgeConfig::default()
        },
        filesystem: false,
        search: false,
        repo: false,
        self_status: false,
        ..ProductionModulesConfig::default()
    }
}

fn build(config: ProductionModulesConfig) -> (ProductionModules, Arc<CountingBuildProbe>) {
    let probe = CountingBuildProbe::new();
    let instrument = BuildInstrumentation::default().with_probe(probe.clone());
    let modules = ProductionModules::build_instrumented(
        config,
        ProductionBackends::default(),
        clock(),
        &instrument,
    )
    .expect("装配必须成功");
    (modules, probe)
}

/// 开/关两种装配都在锁内完成（env 只在 `build` 内读；之后状态自持有）。
fn build_with_env(value: Option<&str>) -> (ProductionModules, Arc<CountingBuildProbe>) {
    let _lock = env_lock();
    let _switch = match value {
        Some(value) => EnvSwitch::set(value),
        None => EnvSwitch::unset(),
    };
    build(base_config())
}

fn lifecycle_ids(modules: &ProductionModules) -> Vec<String> {
    modules
        .ids()
        .into_iter()
        .filter(|id| id == BIO_LIFECYCLE_MODULE_ID)
        .collect()
}

#[test]
fn default_off_registers_no_lifecycle_module_and_constructs_nothing() {
    let (modules, probe) = build_with_env(None);

    assert!(
        modules.ids().iter().all(|id| !id.contains("bio_lifecycle")),
        "默认关 = 不注册生命周期模块: {:?}",
        modules.ids()
    );
    assert!(
        modules.bio_lifecycle_state().is_none(),
        "默认关 = 无生命周期状态句柄"
    );
    assert_eq!(probe.total(), 0, "默认关 = 零构造");
}

#[test]
fn non_one_env_values_stay_fail_closed() {
    for raw in ["", "0", "true", "on", "2", "01", " 1x ", "yes"] {
        let (modules, probe) = build_with_env(Some(raw));
        assert!(
            modules.bio_lifecycle_state().is_none(),
            "env={raw:?} 不是 \"1\" 必须关"
        );
        assert!(
            lifecycle_ids(&modules).is_empty(),
            "env={raw:?} 不是 \"1\" 必须不注册"
        );
        assert_eq!(probe.total(), 0, "env={raw:?} 关 = 零构造");
    }
}

#[test]
fn env_one_registers_exactly_one_module_sharing_one_state() {
    let (modules, probe) = build_with_env(Some("1"));

    assert_eq!(lifecycle_ids(&modules).len(), 1, "开关开 = 恰好注册一次");
    let state = modules.bio_lifecycle_state().expect("开关开 = 状态句柄");
    let again = modules.bio_lifecycle_state().expect("开关开 = 状态句柄");
    assert!(Arc::ptr_eq(&state, &again), "getter 每次返回同一份共享状态");
    assert_eq!(
        state.ledger().len(),
        0,
        "无能力面 = 空种子账本（不编造资产）"
    );
    // 诚实边界：生命周期模块不新增 `ModuleFamily` 变体，构造探针不上报。
    assert_eq!(probe.total(), 0, "生命周期注册不得新增构造探针上报");
}

#[test]
fn registered_tool_names_are_seeded_as_newborn_assets() {
    let _lock = env_lock();
    let _switch = EnvSwitch::set("1");
    let mut config = base_config();
    config.education = true;
    let (modules, probe) = build(config);

    let state: Arc<BioLifecycleState> = modules.bio_lifecycle_state().expect("开关开");
    let tool_names: Vec<String> = modules
        .capabilities()
        .iter()
        .map(|capability| capability.declaration().name)
        .collect();
    assert!(!tool_names.is_empty(), "education 开 = 至少一件真实工具");
    assert_eq!(probe.total(), 1, "只有 Education 上报构造，生命周期不上报");

    let ledger = state.ledger();
    for name in &tool_names {
        let record = ledger
            .get(name)
            .unwrap_or_else(|| panic!("真实工具声明名 {name:?} 必须以 Newborn 入账"));
        assert_eq!(record.asset_id, *name);
        assert_eq!(record.kind, AssetKind::Tool);
        assert_eq!(record.stage, LifecycleStage::Newborn);
        assert_eq!(record.use_count, 0, "注册即事实，尚未使用");
    }
}
