//! 书→技能沉淀 (I1) 的装配级接线证明: 注册即闸 / 名册如实 / 只读预设执法面。
//!
//! 覆盖: `tool.skill_distill` 只在写总闸族同开时注册 (file_write 总闸 +
//! 开关 + 工作区根 + 技能库目录); `tool.skill_search` 只读档独立注册;
//! 自省通道名册按真实生效值报告; `read_only` 会话预设的写能力名单覆盖
//! 沉淀写而不覆盖只读检索。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, Clock};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    is_write_or_execute_capability, roster_from_config, ProductionBackends, ProductionModules,
    ProductionModulesConfig,
};
use apeireth_tools_canonical::{
    SKILL_DISTILL_CAPABILITY_ID, SKILL_DISTILL_TOOL_NAME, SKILL_SEARCH_CAPABILITY_ID,
    SKILL_SEARCH_TOOL_NAME,
};

fn clock() -> Arc<dyn Clock> {
    apeireth_core::kernel::system_clock()
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

fn backends() -> ProductionBackends {
    ProductionBackends {
        workspace_root: Some(PathBuf::from(".")),
        ..ProductionBackends::default()
    }
}

fn registered_ids(modules: &ProductionModules) -> Vec<String> {
    modules
        .capabilities()
        .iter()
        .map(|capability| capability.id().to_string())
        .collect()
}

fn skill_config(distill: bool, search: bool, file_write: bool) -> ProductionModulesConfig {
    let mut config = base_config();
    config.skill_distill = distill;
    config.skill_search = search;
    config.file_write = file_write;
    config.skills_dir = Some(PathBuf::from(".").join("skills"));
    config
}

/// 注册即闸: 沉淀写面要写总闸族同开 (开关 + file_write + 工作区根 + 技能库
/// 目录齐备); 只读检索只需自己的开关 + 技能库目录。
#[tokio::test]
async fn production_registers_the_skill_tools_only_under_their_gates() {
    // 双开 (写总闸同开): 两级都注册。
    let modules = ProductionModules::build(skill_config(true, true, true), backends(), clock())
        .expect("build");
    let ids = registered_ids(&modules);
    assert!(
        ids.iter().any(|id| id == SKILL_DISTILL_CAPABILITY_ID),
        "{ids:?}"
    );
    assert!(
        ids.iter().any(|id| id == SKILL_SEARCH_CAPABILITY_ID),
        "{ids:?}"
    );

    // 声明面 (模型看到的工具) 名实相符。
    let distill = modules
        .capabilities()
        .iter()
        .find(|c| c.id().as_str() == SKILL_DISTILL_CAPABILITY_ID)
        .expect("tool.skill_distill");
    assert_eq!(distill.declaration().name, SKILL_DISTILL_TOOL_NAME);
    let search = modules
        .capabilities()
        .iter()
        .find(|c| c.id().as_str() == SKILL_SEARCH_CAPABILITY_ID)
        .expect("tool.skill_search");
    assert_eq!(search.declaration().name, SKILL_SEARCH_TOOL_NAME);

    // 写总闸关: 沉淀不注册 (fail-closed, 即使开关开) —— 只读检索照常。
    let modules = ProductionModules::build(skill_config(true, true, false), backends(), clock())
        .expect("build");
    let ids = registered_ids(&modules);
    assert!(
        !ids.iter().any(|id| id == SKILL_DISTILL_CAPABILITY_ID),
        "file_write 关时沉淀写面不得注册: {ids:?}"
    );
    assert!(
        ids.iter().any(|id| id == SKILL_SEARCH_CAPABILITY_ID),
        "{ids:?}"
    );

    // 默认关 (opt-in): 不开开关 = 两级都不注册。
    let modules = ProductionModules::build(base_config(), backends(), clock()).expect("build");
    let ids = registered_ids(&modules);
    assert!(!ids.iter().any(|id| id == SKILL_DISTILL_CAPABILITY_ID));
    assert!(!ids.iter().any(|id| id == SKILL_SEARCH_CAPABILITY_ID));

    // 技能库目录缺位 = 注册不成 (注册即闸)。
    let mut no_dir = skill_config(true, true, true);
    no_dir.skills_dir = None;
    let modules = ProductionModules::build(no_dir, backends(), clock()).expect("build");
    let ids = registered_ids(&modules);
    assert!(!ids.iter().any(|id| id == SKILL_DISTILL_CAPABILITY_ID));
    assert!(!ids.iter().any(|id| id == SKILL_SEARCH_CAPABILITY_ID));
}

/// 自省通道名册: 两个开关按真实生效值报告 (写面的生效值含写总闸与技能库
/// 目录, fail-closed 不虚报)。
#[tokio::test]
async fn roster_reports_skill_switches_from_effective_config() {
    async fn roster_flags(config: ProductionModulesConfig) -> serde_json::Value {
        let modules = ProductionModules::build(config, backends(), clock()).expect("build");
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
        let report: serde_json::Value =
            serde_json::from_str(&result.render()).expect("structured self-report json");
        report["capabilities"].clone()
    }

    let capabilities = roster_flags(skill_config(true, true, true)).await;
    assert_eq!(capabilities["skill_distill"], true);
    assert_eq!(capabilities["skill_search"], true);

    // 写总闸关: 写面照实 false, 只读检索不受牵连。
    let capabilities = roster_flags(skill_config(true, true, false)).await;
    assert_eq!(capabilities["skill_distill"], false);
    assert_eq!(capabilities["skill_search"], true);
}

/// 名册投影稳定: 技能两行无重复键、按注册条件取值 (技能库目录缺位 = false)。
#[test]
fn roster_projection_is_stable_for_the_skill_rows() {
    let config = skill_config(true, true, true);
    let roster = roster_from_config(&config, true, false, false, true, &[]);
    let skill_rows: Vec<_> = roster
        .iter()
        .filter(|row| row.name.starts_with("skill_"))
        .map(|row| (row.name.as_str(), row.enabled))
        .collect();
    assert_eq!(
        skill_rows,
        vec![("skill_distill", true), ("skill_search", true)]
    );

    let mut no_dir = config;
    no_dir.skills_dir = None;
    let roster = roster_from_config(&no_dir, true, false, false, true, &[]);
    let skill_rows: Vec<_> = roster
        .iter()
        .filter(|row| row.name.starts_with("skill_"))
        .map(|row| (row.name.as_str(), row.enabled))
        .collect();
    assert_eq!(
        skill_rows,
        vec![("skill_distill", false), ("skill_search", false)],
        "技能库目录缺位时名册照实 false"
    );
}

/// `read_only` 会话预设的执法面: 沉淀写入是写能力 (预设拒); 只读检索不是
/// (预设放行) —— 名单不得有静默缺口。
#[test]
fn read_only_preset_refuses_the_distill_write_but_not_the_read_only_search() {
    assert!(
        is_write_or_execute_capability(&CapabilityId::new(SKILL_DISTILL_CAPABILITY_ID).unwrap()),
        "tool.skill_distill 必须入写能力名单 (read_only 预设拒)"
    );
    assert!(
        !is_write_or_execute_capability(&CapabilityId::new(SKILL_SEARCH_CAPABILITY_ID).unwrap()),
        "tool.skill_search 是只读检索面, 不入写能力名单"
    );
}
