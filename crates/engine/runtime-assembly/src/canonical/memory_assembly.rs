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
use apeireth_memory::MemoryCoordinator;
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
