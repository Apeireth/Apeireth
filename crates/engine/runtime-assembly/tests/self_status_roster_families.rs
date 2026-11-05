//! 自述名册补行批 (2026-11-04) + 纯函数化批 (2026-11-05): 4 类「装配后才存在」
//! 的族在能力名册里的行。
//!
//! 覆盖点 (0 装诚实, 行值 = **实际装配事实**, 不是「能力是否存在」):
//! - 4 类新行**恒在场** (键存在), 开关关时一律 false —— 名册不缺行 = 可自察;
//! - 侦察 (`tool.recon.*`, 9 件) / 外部适配器 (`tool.external.*`, 10 件) 两行
//!   按**实际注册条件**取值 (装配条件缺一即 false);
//! - 游戏练习场 (`tool.play_*`, 4 件) / 仪式完成 (`tool.ritual_done/skip`,
//!   2 件) 两行报**注册事实** (键名带 `_registered` 后缀);
//! - **已注册 ≠ 可调用**: 两族治理面无 grant ⇒ 默认策略恒拒 (行名如实区分);
//! - 补行**不破坏** 21 节自述冻结合同 (节位/节数不变, 新行只在 `capabilities` 节内);
//! - 开关全关时装配面零变化 (四族一件工具都不注册)。
//!
//! 纯函数化批 (2026-11-05): play / ritual 两族生物总闸已折进
//! `ProductionModulesConfig`, 名册 `roster_from_config` **只读 config, 不读进程
//! env** ⇒ 恢复纯函数 (同输入同输出)。故:
//! - 名册级用例**一律构造 config**, 不再靠改 env 造场景;
//! - 装配级用例 (真调 `ProductionModules::build`) 仍走 env —— env 是**装配根**
//!   的开关来源 (装配根把解析结果回填进 config), 那一层本就该读 env; 名册层不该读。
//!
//! 纪律: 本文件是**新建**测试文件, 不改任何既有测试文件。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_core::clock::VirtualClock;
use apeireth_core::kernel::{Clock, Timestamp};
use apeireth_governance::{Decision, PermissionPolicy};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    roster_from_config, ProductionBackends, ProductionModules, ProductionModulesConfig,
};
use apeireth_tools_canonical::recon::Scope;
use apeireth_tools_canonical::{ExternalToolsConfig, ReconConfig, SELF_STATUS_SECTIONS};

/// 注入时钟的固定时刻 (测试永不读系统时钟)。
const NOW_MS: i64 = 1_700_000_000_123;

/// 游戏练习场开关 env 名 (装配根同一张开关表读的就是它)。
const PLAY_ENV: &str = "APEIRETH_ENABLE_BIO_PLAY";
/// 生物候选总闸 env 名 (仪式完成面复用族总闸)。
const CARE_ENV: &str = "APEIRETH_ENABLE_BIO_CARE_CANDIDATES";
/// 节律时区偏移 env 名 (装配根唯一的「参数」; 非法值 = 装配硬失败)。
const TZ_OFFSET_ENV: &str = "APEIRETH_BIO_CIRCADIAN_TZ_OFFSET_MIN";

/// 四类补行族的键名 (断言用同一份字面量, 免得改键时漏改测试)。
const ROSTER_KEYS: [&str; 4] = ["recon", "external", "play_registered", "ritual_registered"];

/// 进程 env 只在本文件的**装配级**用例与纯函数判据里改; 全文件共用一把锁
/// (并行下仍确定)。
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// env 开关守卫: Drop 时清掉, panic 也不污染后续测试。
struct EnvSwitch {
    name: &'static str,
}

impl EnvSwitch {
    fn set(name: &'static str, value: &str) -> Self {
        std::env::set_var(name, value);
        Self { name }
    }
}

impl Drop for EnvSwitch {
    fn drop(&mut self) {
        std::env::remove_var(self.name);
    }
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(NOW_MS)
            .expect("valid fixed timestamp")
            .as_datetime(),
    ))
}

/// 与生产同款的最简配置 (记忆/偏好槽默认关, 逐用例显式打开)。
fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

/// 每个测试独立的工作区目录 (先清空, 避免上次残留)。
fn workspace(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-roster-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试工作区");
    dir
}

/// 显式授权的空 scope (fail-closed 语义的另一侧: 本用例只关心名册行)。
fn scope() -> Arc<Scope> {
    Arc::new(Scope {
        name: "roster-test".to_string(),
        authorized: true,
        paths: Vec::new(),
        domains: Vec::new(),
        cidrs: Vec::new(),
    })
}

/// 名册投影 (键 → 生效值)。
fn roster_map(
    config: &ProductionModulesConfig,
    workspace_root_present: bool,
) -> HashMap<String, bool> {
    roster_from_config(config, workspace_root_present, false, false, false, &[])
        .into_iter()
        .map(|row| (row.name, row.enabled))
        .collect()
}

/// 真调 `tool.self_status` 取整帧 (与生产同一条投影路径)。
async fn self_status_frame(modules: &ProductionModules) -> serde_json::Value {
    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.self_status")
        .cloned()
        .expect("tool.self_status must be registered");
    let call = ToolCall {
        id: "call_roster".into(),
        name: "self_status".into(),
        arguments: serde_json::json!({}),
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    serde_json::from_str(&result.render()).expect("structured self-report json")
}

/// 装配面注册的能力 id 清单。
fn capability_ids(modules: &ProductionModules) -> Vec<String> {
    modules
        .capabilities()
        .iter()
        .map(|capability| capability.id().as_str().to_string())
        .collect()
}

/// 名册补行 ①: 4 类新行**恒在场**, 开关关时一律 false (名册不缺行 = 可自察)。
#[test]
fn the_four_late_assembled_families_always_have_a_roster_row() {
    let roster = roster_map(&base_config(), true);
    for key in ROSTER_KEYS {
        assert!(roster.contains_key(key), "名册必须有 {key} 行");
        assert_eq!(roster[key], false, "{key} 开关关时必须照实 false");
    }
}

/// 名册补行 ②: 侦察 / 外部适配器两行按**实际注册条件**取值。
#[test]
fn recon_and_external_rows_follow_their_registration_conditions() {
    let mut config = base_config();
    config.recon = Some(ReconConfig { scope: scope() });
    config.external = Some(ExternalToolsConfig {
        workspace_root: workspace("external"),
        scope: scope(),
    });

    // 装配条件齐备 (侦察要工作区根, 外部适配器自带根): 两行照实 true。
    let roster = roster_map(&config, true);
    assert_eq!(roster["recon"], true);
    assert_eq!(roster["external"], true);

    // 缺工作区根 = 侦察族注册不成 (文件类工具无落点), 名册照实 false;
    // 外部适配器族的注册条件不看工作区根, 仍 true。
    let roster = roster_map(&config, false);
    assert_eq!(roster["recon"], false);
    assert_eq!(roster["external"], true);
}

/// 名册补行 ③: 游戏练习场 / 仪式完成两行报**注册事实** (键名带 `_registered`)。
///
/// 纯函数化批: 两族总闸改由 config 承载 (**不读 env**), 且「总闸开但注入缺」
/// 仍照实 false —— 总闸是必要条件, 不是注册事实本身。
#[test]
fn play_and_ritual_rows_report_registration_not_callability() {
    let mut config = base_config();
    config.bio_play = true;
    config.bio_care_candidates = true;
    config.care_schedule = Some(workspace("care"));

    let roster = roster_map(&config, true);
    assert_eq!(roster["play_registered"], true);
    assert_eq!(roster["ritual_registered"], true);

    // 装配条件缺一即照实 false: 练习场要工作区根, 仪式要关怀数据目录。
    let roster = roster_map(&config, false);
    assert_eq!(
        roster["play_registered"], false,
        "缺工作区根 = 练习场注册不成"
    );
    let roster = roster_map(&base_config(), true);
    assert_eq!(
        roster["ritual_registered"], false,
        "缺关怀数据目录 = 仪式完成面注册不成"
    );
}

/// 名册补行 ③bis: **总闸开但注入缺** ⇒ 行仍 false (总闸 ≠ 注册事实)。
#[test]
fn an_enabled_switch_without_its_injection_slot_still_reports_false() {
    let mut config = base_config();
    config.bio_play = true;
    config.bio_care_candidates = true;

    // 练习场: 总闸开, 工作区根缺席 = 注册不成。
    let roster = roster_map(&config, false);
    assert_eq!(roster["play_registered"], false);
    // 仪式: 总闸开, 关怀数据目录缺席 = 无处落盘 = 注册不成。
    assert_eq!(roster["ritual_registered"], false);

    // 两族注入齐备 ⇒ 照实 true (总闸 + 注入 = 注册事实)。
    config.care_schedule = Some(workspace("care-both"));
    let roster = roster_map(&config, true);
    assert_eq!(roster["play_registered"], true);
    assert_eq!(roster["ritual_registered"], true);
}

/// 名册补行 ④: **已注册 ≠ 可调用** —— 两族治理面无 grant ⇒ 默认策略恒拒。
///
/// 这正是两行键名带 `_registered` 后缀的理由: 名册说「已注册」, 不说「可调」。
#[test]
fn the_play_and_ritual_families_are_registered_but_never_granted() {
    let policy = PermissionPolicy::default();
    for capability in [
        "tool.play_propose",
        "tool.play_start",
        "tool.play_finish",
        "tool.play_abort",
        "tool.ritual_done",
        "tool.ritual_skip",
    ] {
        assert!(
            matches!(
                policy.decision_for_capability(capability),
                Decision::Deny { .. }
            ),
            "{capability} 无 grant 时必须恒拒 (名册只报注册事实, 不冒充可调)"
        );
    }
}

/// 名册补行 ⑤: 补行**不破坏** 21 节自述冻结合同 (节位/节数不变; 新行只在
/// `capabilities` 节内, 值仍是布尔)。
#[tokio::test]
async fn the_new_rows_do_not_break_the_frozen_section_contract() {
    let _guard = env_lock();
    let modules = ProductionModules::build(
        base_config(),
        ProductionBackends {
            workspace_root: Some(workspace("contract")),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .expect("装配必须成功");

    let frame = self_status_frame(&modules).await;
    assert_eq!(
        SELF_STATUS_SECTIONS.len(),
        21,
        "21 节自述是冻结合同: 节数不得变"
    );
    let sections = frame.as_object().expect("自述帧是对象");
    assert_eq!(
        sections.len(),
        SELF_STATUS_SECTIONS.len(),
        "帧顶层节数必须与冻结表一致 (补行不得新增顶层节)"
    );
    for section in SELF_STATUS_SECTIONS {
        assert!(sections.contains_key(*section), "缺节 {section}");
    }

    let capabilities = frame["capabilities"]
        .as_object()
        .expect("capabilities 是对象");
    for key in ROSTER_KEYS {
        assert!(capabilities.contains_key(key), "capabilities 节缺 {key} 行");
        assert!(capabilities[key].is_boolean(), "{key} 行必须是布尔");
    }
}

/// 名册补行 ⑥: 开关全关时装配面**零变化** —— 四族一件工具都不注册。
#[tokio::test]
async fn with_the_switches_off_the_four_families_register_nothing() {
    let _guard = env_lock();
    let modules = ProductionModules::build(
        base_config(),
        ProductionBackends {
            workspace_root: Some(workspace("off")),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .expect("装配必须成功");

    let ids = capability_ids(&modules);
    for id in &ids {
        assert!(!id.starts_with("tool.recon."), "侦察族不该注册: {id}");
        assert!(
            !id.starts_with("tool.external."),
            "外部适配器族不该注册: {id}"
        );
        assert!(!id.starts_with("tool.play_"), "游戏练习场族不该注册: {id}");
        assert!(
            id != "tool.ritual_done" && id != "tool.ritual_skip",
            "仪式族不该注册: {id}"
        );
    }
}

/// 名册补行 ⑦: 开关开且**真装配**时, 两行照实 true (装配级, 经真调用取证)。
///
/// 纯函数化批: env 是**装配根**的开关来源 —— 装配根把解析结果回填进 config,
/// 名册再只读 config。故本用例仍设 env (装配根那一层本就该读 env), 且顺带钉住
/// 「装配根回填 ⇒ 名册照实 true」这条新链路 (传入的 config 两总闸为默认值 = 关)。
#[tokio::test]
async fn the_rows_turn_true_only_when_the_families_are_really_assembled() {
    let _guard = env_lock();
    let _play = EnvSwitch::set(PLAY_ENV, "1");
    let _care = EnvSwitch::set(CARE_ENV, "1");

    let mut config = base_config();
    config.care_schedule = Some(workspace("on-care"));
    // 传入 config 的两总闸是默认值 (关) —— 真值只能来自装配根回填。
    assert_eq!(config.bio_play, false);
    assert_eq!(config.bio_care_candidates, false);
    let modules = ProductionModules::build(
        config,
        ProductionBackends {
            workspace_root: Some(workspace("on")),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .expect("装配必须成功");

    let ids = capability_ids(&modules);
    for id in [
        "tool.play_propose",
        "tool.play_start",
        "tool.play_finish",
        "tool.play_abort",
        "tool.ritual_done",
        "tool.ritual_skip",
    ] {
        assert!(
            ids.iter().any(|registered| registered == id),
            "开关开必须真注册 {id}"
        );
    }

    let frame = self_status_frame(&modules).await;
    assert_eq!(frame["capabilities"]["play_registered"], true);
    assert_eq!(frame["capabilities"]["ritual_registered"], true);
}

/// 纯函数判据 (2026-11-05): `roster_from_config` **不读进程 env** —— 同进程内
/// 改 env 后再调它, 输出**逐行不变** (同输入同输出)。
#[test]
fn the_roster_is_a_pure_function_and_ignores_process_env() {
    let _guard = env_lock();

    // 样本一: 两总闸皆开 + 注入齐备。
    let mut on = base_config();
    on.bio_play = true;
    on.bio_care_candidates = true;
    on.care_schedule = Some(workspace("pure-on"));
    let on_before = roster_map(&on, true);

    // 样本二: 两总闸皆关 (默认 fail-closed)。
    let off = base_config();
    let off_before = roster_map(&off, true);

    // 同进程内把两族 env 开关打开 —— 名册输出必须**一字不变**。
    let _play = EnvSwitch::set(PLAY_ENV, "1");
    let _care = EnvSwitch::set(CARE_ENV, "1");
    assert_eq!(
        on_before,
        roster_map(&on, true),
        "改 env 后名册输出必须不变 (名册不读 env ⇒ 纯函数)"
    );
    assert_eq!(
        off_before,
        roster_map(&off, true),
        "env 打开也不许把默认关闭的两族翻成 true (名册只认 config)"
    );

    // 复核: 两样本的值本身仍符合「总闸 + 注入」口径 (断言未被上面的等价性稀释)。
    assert_eq!(on_before["play_registered"], true);
    assert_eq!(on_before["ritual_registered"], true);
    assert_eq!(off_before["play_registered"], false);
    assert_eq!(off_before["ritual_registered"], false);
}

/// fail-closed 判据 (2026-11-05): 开关表解析失败 (节律时区参数非法) 时,
/// **装配根硬失败** (拒开 = 不产生任何名册), 而不是「静默全关后照常开跑」。
#[tokio::test]
async fn an_invalid_switchboard_parameter_refuses_the_assembly() {
    let _guard = env_lock();
    let _tz = EnvSwitch::set(TZ_OFFSET_ENV, "not-a-number");

    let Err(error) = ProductionModules::build(
        base_config(),
        ProductionBackends {
            workspace_root: Some(workspace("bad-tz")),
            ..ProductionBackends::default()
        },
        clock(),
    ) else {
        panic!("非法节律时区参数必须拒开 (fail-closed, 不静默回退)");
    };
    assert!(
        error.to_string().contains("nervous switchboard"),
        "拒绝原因必须指向开关表解析, got: {error}"
    );
}
