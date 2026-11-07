//! `rt.module_execution_contract` 的接线级证据（生产装配面选举）。
//!
//! 缺口（补前）: 生产组合根经 `production::collect_tool_module`
//! (`canonical/production.rs:634`) 把所有工具模块收进来, 那条路径只会调
//! `CapabilityProvider::tool_module()`（兼容合同）⇒ 生产里 `ContractTool` 零构造、
//! `module_public_declarations()` 结构性为空、内部面拒答分支不可达。
//!
//! 本文件钉住**生产装配辅助**而不是另写一条测试专用路径:
//! `canonical::module_contract_wiring::attach` 正是
//! `ProductionModules::register_with_policy` 调用的那个函数
//! (`canonical/cognitive_registry.rs`, 由 CLI 组合根 `cli/src/lib.rs:1706` 调用);
//! `elect_provider_module` 是 `collect_tool_module` 的补丁落点。三者共用
//! [`PRODUCTION_MODULE_CONTRACTS`] 一张表。
//!
//! 断言口径（只收窄面, 不授权）:
//! ① 表内模块在**活的** CapabilityRegistry 上带 Logical/Pure 合同;
//! ② 该模块进入公共模块面 (`module_public_declarations`);
//! ③ 模型面目录与工具名逐字不变, 且不触发 System One 复核（不是新权限）;
//! ④ 表外模块保持兼容合同（默认零变化）;
//! ⑤ 构造期选举与 builder 期选举给出同一合同, 且重复应用幂等。

use apeireth_core::kernel::PluginId;
use apeireth_governance::AllowAll;
use apeireth_plugin::module_contract::{EffectClass, ModuleToolContract, ToolSurface};
use apeireth_plugin::{Plugin, ToolCapability};
use apeireth_runtime::{Runtime, RuntimeBuilder};
use apeireth_runtime_assembly::canonical::module_contract_wiring;
use apeireth_runtime_assembly::canonical::self_status_source::ProductionSelfStatusSource;
use apeireth_runtime_assembly::canonical::tool_modules::{FilesystemModule, SelfStatusModule};
use apeireth_runtime_assembly::canonical::CapabilityProvider;
use std::sync::Arc;

/// The exact provider production builds at `canonical/production.rs:1355-1358`.
fn provider() -> SelfStatusModule {
    SelfStatusModule::new(Arc::new(ProductionSelfStatusSource::new(
        Vec::new(),
        false,
        None,
    )))
}

fn builder(provider: &SelfStatusModule) -> RuntimeBuilder {
    Runtime::builder()
        .with_default_model("fixture")
        .with_governance(Arc::new(AllowAll))
        .with_tool_module(provider)
        .unwrap()
}

#[tokio::test]
async fn production_assembly_elects_the_public_contract_onto_the_live_registry() {
    let module = provider();
    assert!(
        module_contract_wiring::PRODUCTION_MODULE_CONTRACTS
            .iter()
            .any(|(elected, _)| *elected == module.id()),
        "装配表必须覆盖生产实际注册的模块 id"
    );

    // 补前形态（今天生产的样子）: 兼容合同 ⇒ 公共模块面为空。
    let compat = builder(&module).build().await.unwrap();
    assert_eq!(
        compat.capability_registry().capabilities()[0].module_contract(),
        ModuleToolContract::default()
    );
    assert!(compat
        .capability_registry()
        .module_public_declarations()
        .is_empty());
    drop(compat);

    // 生产装配辅助（`register_with_policy` 调的就是它）。
    let elected = module_contract_wiring::attach(builder(&module))
        .unwrap()
        .build()
        .await
        .unwrap();
    let tool = elected.capability_registry().capabilities()[0].clone();
    let contract = tool.module_contract();
    assert_eq!(contract.surface, ToolSurface::Logical);
    assert_eq!(contract.effect, EffectClass::Pure);
    assert!(contract.module_visible);
    assert!(contract.query_safe);
    // 只收窄面: 不是新权限 ⇒ 不要求 System One 复核。
    assert!(!apeireth_plugin::system_one::requires_system_one(&contract));

    // 输出点①: 模块公共面（补前恒空）。
    let public = elected.capability_registry().module_public_declarations();
    assert_eq!(public.len(), 1);
    assert_eq!(public[0].name, tool.declaration().name);
    assert_eq!(public[0].name, "self_status");
    // 输出点②: 模型面目录逐字不变（仍是同一件工具, 同一名字）。
    let declarations = elected.tool_declarations();
    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].name, "self_status");

    // 声明面（manifest）与可执行面一致: 这是 PluginManager 注册门自己校验的那条。
    let id = PluginId::new(module.id()).unwrap();
    let declared = elected
        .plugins()
        .plugins()
        .get(&id)
        .unwrap()
        .manifest()
        .capability(tool.id())
        .unwrap()
        .module_tool_contract()
        .unwrap()
        .unwrap();
    assert_eq!(declared, contract);
}

#[tokio::test]
async fn construction_time_election_matches_builder_time_and_attach_stays_idempotent() {
    let module = provider();
    let expected = module_contract_wiring::elected_contract(module.id()).unwrap();
    // `collect_tool_module` 补丁的落点: 构造期选举。
    let constructed = module_contract_wiring::elect_provider_module(&module).unwrap();
    let elected = module_contract_wiring::attach(
        Runtime::builder()
            .with_default_model("fixture")
            .with_governance(Arc::new(AllowAll))
            .with_plugin(constructed),
    )
    .unwrap()
    .build()
    .await
    .unwrap();
    let tools = elected.capability_registry().capabilities();
    assert_eq!(tools.len(), 1, "选举不得把一个模块变成两件工具");
    assert_eq!(tools[0].module_contract(), expected);
    assert_eq!(
        elected
            .capability_registry()
            .module_public_declarations()
            .len(),
        1,
        "重复应用必须幂等（不得重复包装成多条公共声明）"
    );
}

#[tokio::test]
async fn modules_absent_from_the_table_keep_the_compatibility_contract() {
    let dir = tempfile::tempdir().unwrap();
    let module = FilesystemModule::new(dir.path());
    assert!(
        module_contract_wiring::elected_contract(module.id()).is_none(),
        "表外模块不得被默认选举"
    );
    let runtime = module_contract_wiring::attach(
        Runtime::builder()
            .with_default_model("fixture")
            .with_governance(Arc::new(AllowAll))
            .with_tool_module(&module)
            .unwrap(),
    )
    .unwrap()
    .build()
    .await
    .unwrap();
    let tools = runtime.capability_registry().capabilities();
    assert!(!tools.is_empty());
    for tool in tools {
        assert_eq!(tool.module_contract(), ModuleToolContract::default());
    }
    assert!(runtime
        .capability_registry()
        .module_public_declarations()
        .is_empty());
}

/// 表内每一项都必须是「只读公共查询」档: 一旦有人把 Protected/Orchestration
/// 塞进生产表, 这条断言先红（那需要单独的权限复核, 不属于本声明的接线）。
#[test]
fn every_production_election_is_a_read_only_public_surface() {
    for (id, contract) in module_contract_wiring::PRODUCTION_MODULE_CONTRACTS {
        assert_eq!(
            contract.surface,
            ToolSurface::Logical,
            "{id}: 生产选举必须是逻辑面"
        );
        assert_eq!(
            contract.effect,
            EffectClass::Pure,
            "{id}: 生产选举必须无副作用"
        );
        assert!(contract.module_visible, "{id}: 公共面必须模块可见");
        assert!(!apeireth_plugin::system_one::requires_system_one(contract));
        contract.validate().unwrap();
    }
}
