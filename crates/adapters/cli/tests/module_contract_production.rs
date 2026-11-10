//! `rt.module_execution_contract` 的**生产组合根**证据（真 CLI 装配, keyless, 无网络）。
//!
//! 这条测试走的是生产唯一组合根 `build_canonical_runtime_from_env`：
//! `cli/src/lib.rs:1706` → `ProductionModules::register_with_policy` →
//! `canonical::module_contract_wiring::attach`。它不自己拼装配路径, 因此能证明
//! **生产调用点真的执行了契约选举**, 而不只是「辅助函数可用」。
//!
//! 钉住四件事（都只收窄面, 不授权）:
//! ① 活的 `CapabilityRegistry` 上 `tool.self_status` 带 Logical/Pure 合同;
//! ② 该模块进入公共模块面 `module_public_declarations()`（补前生产里恒为空）;
//! ③ 模型面目录逐字不变: `self_status` 仍恰好声明一次, 且不要求 System One 复核;
//! ④ 表外工具仍保持兼容合同（默认零变化）。

use apeireth_plugin::module_contract::{EffectClass, ModuleToolContract, ToolSurface};
use apeireth_plugin::ToolCapability;

#[tokio::test]
async fn production_composition_root_elects_the_public_module_contract() {
    let dir = tempfile::tempdir().expect("temp data root");
    std::env::set_var("APEIRETH_SESSION_DB", dir.path().join("session.sqlite3"));
    std::env::set_var(
        "APEIRETH_COGNITIVE_DB",
        dir.path().join("cognitive.sqlite3"),
    );
    std::env::set_var("APEIRETH_DATA_DIR", dir.path());
    // Optional modules: keep the assembly to the defaults so the assertions below
    // describe default production, not a locally enabled feature set.
    for key in [
        "APEIRETH_ENABLE_MCP",
        "APEIRETH_ENABLE_FETCH",
        "APEIRETH_ENABLE_SHELL",
        "APEIRETH_ENABLE_LOCAL_READ_TOOLS",
        "APEIRETH_MODEL",
        "APEIRETH_API_KEY",
    ] {
        std::env::remove_var(key);
    }

    let mut runtime = apeireth_cli::build_canonical_runtime_from_env()
        .await
        .expect("keyless production runtime builds without network");

    for key in [
        "APEIRETH_SESSION_DB",
        "APEIRETH_COGNITIVE_DB",
        "APEIRETH_DATA_DIR",
    ] {
        std::env::remove_var(key);
    }

    let entries = runtime.capability_registry().entries();
    let self_status = entries
        .iter()
        .find(|(_, capability)| capability.id().as_str() == "tool.self_status")
        .map(|(_, capability)| capability.clone())
        .expect("tool.self_status is registered by default");

    // ① 生产装配选举的合同落在活的注册表上（不是只在构造函数里）。
    let contract = self_status.module_contract();
    assert_eq!(contract.surface, ToolSurface::Logical);
    assert_eq!(contract.effect, EffectClass::Pure);
    assert!(contract.module_visible);
    assert!(contract.query_safe);
    contract.validate().expect("elected contract is valid");

    // ② 公共模块面: 补前生产里 `module_public_declarations()` 结构性为空。
    let public = runtime.capability_registry().module_public_declarations();
    assert!(
        public
            .iter()
            .any(|declaration| declaration.name == "self_status"),
        "public module surface: {public:?}"
    );

    // ③ 没有新权限: 不触发 System One 复核; 模型面恰好一条同名声明。
    assert!(!apeireth_plugin::system_one::requires_system_one(&contract));
    assert_eq!(
        runtime
            .tool_declarations()
            .iter()
            .filter(|declaration| declaration.name == "self_status")
            .count(),
        1
    );

    // ④ 表外工具一字未动: 仍全部是兼容合同。
    for (owner, capability) in &entries {
        if capability.id().as_str() == "tool.self_status" {
            continue;
        }
        assert_eq!(
            capability.module_contract(),
            ModuleToolContract::default(),
            "{owner}/{} must keep the compatibility contract",
            capability.id()
        );
    }

    let shutdown_errors = runtime.shutdown().await;
    assert!(shutdown_errors.is_empty(), "{shutdown_errors:?}");
}
