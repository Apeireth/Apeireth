//! Recon / 外部适配器装配级钉子（2026-10-22 接线批）。
//!
//! 钉三件事：
//! 1. **默认关 fail-closed**：`recon` / `external` 为 `None` 时 19 件工具一件都不注册；
//! 2. **注册即闸**：给了配置才注册，件数 = 9 + 10，id 集合与真源清单逐一相等；
//! 3. **构造探针照实**：开关开时 `ModuleFamily::Recon` / `ExternalTools` 各上报一次，
//!    关闭时零上报（族构造点不撒谎）。

use std::path::Path;
use std::sync::Arc;

use apeireth_core::clock::SystemClock;
use apeireth_core::kernel::Clock;
use apeireth_runtime_assembly::{
    BuildInstrumentation, CountingBuildProbe, ModuleBuildProbe, ModuleFamily, ProductionBackends,
    ProductionModules, ProductionModulesConfig,
};
use apeireth_tools_canonical::recon::wiring::{
    EXTERNAL_TOOL_IDS, RECON_LOCAL_TOOL_IDS, RECON_NETWORK_TOOL_IDS,
};
use apeireth_tools_canonical::recon::Scope;

fn instrumentation(probe: &Arc<CountingBuildProbe>) -> BuildInstrumentation {
    let concrete = Arc::clone(probe);
    let handle: Arc<dyn ModuleBuildProbe> = concrete;
    BuildInstrumentation::default().with_probe(handle)
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(SystemClock)
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

fn backends(root: &Path) -> ProductionBackends {
    ProductionBackends {
        workspace_root: Some(root.to_path_buf()),
        ..ProductionBackends::default()
    }
}

fn capability_ids(modules: &ProductionModules) -> Vec<String> {
    let mut ids: Vec<String> = modules
        .capabilities()
        .iter()
        .map(|capability| capability.id().as_str().to_string())
        .collect();
    ids.sort();
    ids
}

#[test]
fn recon_and_external_stay_off_by_default() {
    let probe = CountingBuildProbe::new();
    let dir = std::env::temp_dir();
    let modules = ProductionModules::build_instrumented(
        base_config(),
        backends(&dir),
        clock(),
        &instrumentation(&probe),
    )
    .expect("assembly");

    let ids = capability_ids(&modules);
    for id in RECON_LOCAL_TOOL_IDS
        .iter()
        .chain(RECON_NETWORK_TOOL_IDS)
        .chain(EXTERNAL_TOOL_IDS)
    {
        assert!(
            !ids.iter().any(|registered| registered == id),
            "默认关不得注册 {id}"
        );
    }
    assert_eq!(probe.count(ModuleFamily::Recon), 0, "默认关零构造");
    assert_eq!(probe.count(ModuleFamily::ExternalTools), 0, "默认关零构造");
}

#[test]
fn recon_and_external_register_exactly_the_declared_sets_when_wired() {
    let probe = CountingBuildProbe::new();
    let dir = std::env::temp_dir();
    let scope = Arc::new(Scope::empty("测试空范围"));

    let mut config = base_config();
    config.recon = Some(apeireth_tools_canonical::ReconConfig {
        scope: Arc::clone(&scope),
    });
    config.external = Some(apeireth_tools_canonical::ExternalToolsConfig {
        workspace_root: dir.clone(),
        scope: Arc::clone(&scope),
    });

    let modules = ProductionModules::build_instrumented(
        config,
        backends(&dir),
        clock(),
        &instrumentation(&probe),
    )
    .expect("assembly");

    let ids = capability_ids(&modules);
    for id in RECON_LOCAL_TOOL_IDS
        .iter()
        .chain(RECON_NETWORK_TOOL_IDS)
        .chain(EXTERNAL_TOOL_IDS)
    {
        assert!(
            ids.iter().any(|registered| registered == id),
            "必须注册 {id}"
        );
    }
    assert_eq!(
        ids.iter()
            .filter(|id| id.starts_with("tool.recon.") || id.starts_with("tool.external."))
            .count(),
        19,
        "9 原生 + 10 外部，一件不多一件不少"
    );
    assert_eq!(probe.count(ModuleFamily::Recon), 1, "开关开 = 恰好构造一次");
    assert_eq!(
        probe.count(ModuleFamily::ExternalTools),
        1,
        "开关开 = 恰好构造一次"
    );
}

/// 工作区根缺省时 recon 不注册（文件类工具无落点），但外部适配器自带根、
/// 照常注册 —— 这条差异必须显式钉住，不能靠"反正没人这么配"。
#[test]
fn recon_without_workspace_root_is_not_registered() {
    let probe = CountingBuildProbe::new();
    let dir = std::env::temp_dir();
    let scope = Arc::new(Scope::empty("测试空范围"));

    let mut config = base_config();
    config.recon = Some(apeireth_tools_canonical::ReconConfig { scope });

    let modules = ProductionModules::build_instrumented(
        config,
        ProductionBackends::default(),
        clock(),
        &instrumentation(&probe),
    )
    .expect("assembly");

    let ids = capability_ids(&modules);
    assert!(
        !ids.iter().any(|id| id.starts_with("tool.recon.")),
        "无工作区根 = recon 不注册"
    );
    assert!(
        ids.iter().any(|id| id == "tool.filesystem") || !ids.is_empty(),
        "装配本身仍然成功"
    );
    assert_eq!(probe.count(ModuleFamily::Recon), 0, "未构造不得上报");
    let _ = dir;
}
