//! 注册式 Memory 工具面的**生产槽位**（`mem.registered_namespace` / `rt.owned_agents`）。
//!
//! # 它补的是哪个洞
//!
//! `canonical/production.rs` 的 `build_inner` 在 `memory_recall || memory_writeback`
//! 时构造了一个 `MemoryCoordinator`，但它**只喂给行为模块**（召回/写回），
//! **从未**把它变成 `MemoryModule` 工具面 ⇒ ⇒ 生产装配里 `module.memory` 不存在
//! ⇒ ⇒ 7 个 `memory_*` 工具一个都没注册（调用即 `requested tool is not registered`），
//! `BeforeModelCall` 固定检索 Hook 也不会跑 ⇒ 声明只能停在 IMPLEMENTED。
//! 测试里之所以能跑，是因为**测试自己**手动 `with_plugin(MemoryModule::new(...))`
//! ⇒ ⇒ 「测试在调用装配根不提供的东西」= 典型假绿。
//!
//! # 为什么放在这个文件
//!
//! `production.rs` 在 `scripts/arch-baseline.txt` 里登记尺寸（1897 行，只许不变或变小）
//! ⇒ ⇒ 装配细节搬到这里，`production.rs` 只留**一行**调用。
//!
//! # 边界（如实写清）
//!
//! * 只注册**兼容 ABI**（`MemoryModule::new` 的 7 工具面）：`with_module_contract()`
//!   是**宿主显式选择**（会多一个 `organizer_status` 并把 6 个工具标成 `Protected`
//!   ⇒ 需要 System One 复核面在场）⇒ 生产默认不做这个选择，保持与既有测试同形。
//! * 命名空间权威**只来自执行上下文**（`ToolExecutionContext.principal`）；Host 的
//!   `ProfileMemoryMappings` 由注册期 [`bind_profile_mappings`] **迟绑定**到同一实例
//!   （`MemoryModule` 内部的映射槽可后绑），而不是另建一个模块。

use super::memory_module::MemoryModule;
use crate::profile_memory::ProfileMemoryMappings;
use apeireth_core::kernel::Clock;
use apeireth_memory::{MemoryCoordinator, MemoryGovernanceStore};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_plugin::{Plugin, ToolCapability};
use std::sync::Arc;

/// 把已构造好的 coordinator 变成生产工具面，并登记它的工具与模块。
///
/// 返回模块句柄：`production::ProductionModules` 用它做注册期的映射迟绑定。
pub(super) fn wire(
    tool_modules: &mut Vec<Arc<dyn Plugin>>,
    capabilities: &mut Vec<Arc<dyn ToolCapability>>,
    coordinator: MemoryCoordinator,
    clock: &Arc<dyn Clock>,
) -> (Arc<MemoryModule>, Arc<MemoryCoordinator>) {
    let coordinator = Arc::new(coordinator);
    // ⚠️ **自动检索默认关**（主指挥裁决 2026-11-07）：注册工具面 ≠ 每回合自动注入。
    // `automatic_retrieval` 会在 `BeforeModelCall` 自动发一次 `memory.retrieve` 工具调用
    // ⇒ 每回合转录多两条（tool call + tool result）、上下文每回合被改写；而生产另有
    // `ProactiveRecallPolicy` 这条**显式选择**的召回入口 ⇒ 默认再开一条属于「无声行为变化」。
    // 实证：开它会让既有不变量测试 `learning_does_not_pollute_the_primary_transcript`
    // 由 2 条变 4 条（转录被工具回合污染）。固定 Hook 的能力仍在（Host 可显式开）。
    let module = Arc::new(
        MemoryModule::new(Arc::clone(&coordinator), Arc::clone(clock))
            .with_automatic_retrieval(false),
    );
    capabilities.extend(module.tools());
    tool_modules.push(Arc::clone(&module) as Arc<dyn Plugin>);
    (module, coordinator)
}

/// 注册期迟绑定 Host 映射表（未武装 = `None` = 零改动）。
pub(super) fn bind_profile_mappings(
    slot: &Option<Arc<MemoryModule>>,
    mappings: &Arc<ProfileMemoryMappings>,
) {
    if let Some(module) = slot {
        module.attach_profile_mappings(Arc::clone(mappings));
    }
}

/// 把 **Host 配置面**的检索旋钮（`retrieval.json`）施加到 coordinator 的排序配置上。
///
/// # 为什么放这里
/// `production.rs` 登记了尺寸（只许不变或变小）⇒ ⇒ 装配细节搬到这里，
/// `production.rs` 只留**一行**调用。
///
/// # 三种结果（失败闭合）
///
/// * 文件缺失 ⇒ **不施加任何旋钮** ⇒ 逐字节零行为变化（默认态）；
/// * 文件存在但坏 ⇒ **报错**（装配失败）：坏配置不许被当成「没配置」；
/// * 文件存在且合法 ⇒ 只施加**文件里显式写了**的旋钮。
///
/// ⚠️ **只在启动时读一次**（不做运行中热重载）：热重载 = 会话中途静默改排序权重
/// ⇒ 与治理链/审计口径冲突；我们另有**受治理的运行时调权**（证据驱动、有账）。
pub(super) fn apply_retrieval_knobs(
    coordinator: MemoryCoordinator,
) -> Result<MemoryCoordinator, String> {
    apply_retrieval_knobs_at(coordinator, &crate::paths::default_data_root())
}

/// 装配记忆的**三个面**：**排序旋钮配置面** + **契约口径自检** + **影子双跑/换算法**。
///
/// # 为什么放这里
/// `production.rs` 登记了尺寸（只许不变或变小）⇒ ⇒ 装配细节搬到这里，
/// `production.rs` 只留**一行**调用。
///
/// # 失败闭合（三条，逐条可复核）
///
/// * 排序旋钮配置坏 ⇒ **拒绝装配**（坏配置不许被当成「没配置」）；
/// * 契约可见性口径与 canonical 分叉 ⇒ **拒绝启动**（`memory_contract_wire::install`）；
/// * 影子配置坏 ⇒ **拒绝装配**；配置**缺失** ⇒ **不构造任何东西**（逐字节零行为变化）。
///
/// ⚠️ 影子/换算法的**治理链**暂传 `None` ⇒ ⇒ **切换会被拒绝**（失败闭合，刻意）
/// ⇒ ⇒ 要真放行，需要装配根手里有治理链句柄（当前没有 ⇒ 这是**如实缺口**，不是漏接）。
pub(super) fn assemble_memory(
    memory: Arc<dyn MemoryBackend>,
    governance: Arc<dyn MemoryGovernanceStore>,
) -> Result<MemoryCoordinator, String> {
    // ① 契约口径自检（两套可见性口径分叉 ⇒ 进程起不来）
    crate::canonical::memory_contract_wire::install()?;
    // ② 排序旋钮配置面（默认关 ⇒ 零行为变化；坏配置 ⇒ 拒绝装配）
    let coordinator = apply_retrieval_knobs(MemoryCoordinator::new(
        Arc::clone(&memory),
        Arc::clone(&governance),
    ))?;
    // ③ 影子双跑 + 换算法流程（服务落在进程级槽里 ⇒ 调用方**不必**持有句柄）
    let _shadow = crate::canonical::shadow_wire::install_shadow(
        memory,
        governance,
        &crate::paths::default_data_root(),
        None,
    )?;
    Ok(coordinator)
}

/// 同上，但**数据根显式给出**（测试用；生产走 [`apply_retrieval_knobs`]）。
pub(super) fn apply_retrieval_knobs_at(
    coordinator: MemoryCoordinator,
    data_root: &std::path::Path,
) -> Result<MemoryCoordinator, String> {
    let Some(body) = apeireth_memory::ranking_knobs::knobs_from_data_root(data_root)? else {
        return Ok(coordinator);
    };
    let mut ranking = apeireth_memory::MemoryRankingConfig::default();
    apeireth_memory::ranking_knobs::apply_knobs(&mut ranking, &body);
    Ok(coordinator.with_ranking_config(ranking))
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_memory::{MemoryCoordinator, MemoryGovernanceStore, SqliteMemoryStore};
    use apeireth_plugin::memory_backend::MemoryBackend;
    use std::sync::Arc;

    fn coordinator() -> MemoryCoordinator {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("store"));
        let backend: Arc<dyn MemoryBackend> = store.clone();
        let governance: Arc<dyn MemoryGovernanceStore> = store;
        MemoryCoordinator::new(backend, governance)
    }

    /// ⭐ **没有配置文件 ⇒ 不施加任何旋钮**（默认态 ⇒ 零行为变化）。
    #[test]
    fn a_missing_knob_file_leaves_the_coordinator_untouched() {
        let dir = std::env::temp_dir().join(format!("apeireth-knobs-none-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let result = apply_retrieval_knobs_at(coordinator(), &dir);
        assert!(result.is_ok(), "缺文件必须照常装配（不是错误）");
    }

    /// ⭐ **配置文件坏了 ⇒ 装配失败**（失败闭合：坏配置不许被当成「没配置」）。
    #[test]
    fn a_broken_knob_file_refuses_to_assemble() {
        let dir = std::env::temp_dir().join(format!("apeireth-knobs-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(dir.join("retrieval.json"), b"{ not json").expect("write");
        match apply_retrieval_knobs_at(coordinator(), &dir) {
            Ok(_) => panic!("坏配置必须报错（不许被当成「没配置」）"),
            Err(error) => assert!(!error.is_empty(), "必须带真因"),
        }
    }
}
