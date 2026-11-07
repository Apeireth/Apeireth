//! Production election of the typed module contract (`rt.module_execution_contract`).
//!
//! # The gap this file closes
//!
//! Every production tool module is collected through one chokepoint,
//! `canonical/production.rs:634` `collect_tool_module`, which calls
//! `CapabilityProvider::tool_module()` — the *compatibility* default. No
//! production provider ever elected a contract, so in production
//! `ContractTool` was never constructed, `module_public_declarations()` was
//! structurally empty, and the internal-surface refusal branch was unreachable
//! (tests were the only electors). This module is the missing election.
//!
//! # One table, no new switch
//!
//! No `APEIRETH_*` variable is introduced (the env budget is spent) and none is
//! needed: the table below is static assembly policy. An id absent from it keeps
//! the compatibility ABI byte for byte, and an election only *restricts* a
//! surface — it never grants authority, never changes a wire name, and never
//! changes a frozen payload.
//!
//! # Two application seams, one truth
//!
//! * [`elect_provider_module`] — construction time, the drop-in body of
//!   `production::collect_tool_module` (a 3-line, line-neutral patch; see the
//!   delivery report).
//! * [`attach`] — builder time, called by
//!   `canonical::cognitive_registry::ProductionModules::register_with_policy`,
//!   which the CLI composition root calls at
//!   `crates/adapters/cli/src/lib.rs:1706`.
//!
//! Both read [`PRODUCTION_MODULE_CONTRACTS`] and both are idempotent: a module
//! whose tools already carry a non-default contract is left untouched, so
//! applying construction-time election first turns the builder-time seam into a
//! no-op instead of double-wrapping.

use apeireth_plugin::module_contract::{
    EffectClass, ExecutionMode, ModuleToolContract, ToolSurface,
};
use apeireth_plugin::Plugin;
use apeireth_runtime::canonical::CapabilityProvider;
use apeireth_runtime::RuntimeBuilder;
use std::sync::Arc;

/// A read-only, model-visible, module-visible surface: no System One review
/// (`effect` is not `Protected`), no Orchestration, no side effect at all.
const fn read_only_public_contract() -> ModuleToolContract {
    ModuleToolContract {
        version: 1,
        surface: ToolSurface::Logical,
        execution: ExecutionMode::Direct,
        effect: EffectClass::Pure,
        module_visible: true,
        query_safe: true,
    }
}

/// The production election table (module id → elected contract).
///
/// Only genuinely read-only, default-on modules belong here. An election changes
/// what the module's *public* surface advertises; it does not change the
/// model-facing catalog, the governance decision, or any executor behaviour.
/// Electing a module that writes, executes, or reaches the network would need its
/// own review (its effect class would stop being `Pure`).
pub const PRODUCTION_MODULE_CONTRACTS: &[(&str, ModuleToolContract)] =
    &[("module.tool.self_status", read_only_public_contract())];

/// The contract this assembly elects for `module_id`, if any.
///
/// `None` is the compatibility answer: the module keeps its default contract.
pub fn elected_contract(module_id: &str) -> Option<ModuleToolContract> {
    PRODUCTION_MODULE_CONTRACTS
        .iter()
        .find(|(id, _)| *id == module_id)
        .map(|(_, contract)| contract.clone())
}

/// Construction-time election for one provider (mirrors `collect_tool_module`).
///
/// Returns the provider's ordinary `ToolModule` when the table does not elect it,
/// so the caller sees the same object it built itself.
pub fn elect_provider_module(provider: &dyn CapabilityProvider) -> Result<Arc<dyn Plugin>, String> {
    let module: Arc<dyn Plugin> = Arc::new(provider.tool_module()?);
    match elected_contract(provider.id()) {
        Some(contract) => apeireth_plugin::tool_module::elect_module_contract(module, &contract),
        None => Ok(module),
    }
}

/// Builder-time election with the production table (the applied seam).
pub fn attach(builder: RuntimeBuilder) -> Result<RuntimeBuilder, String> {
    attach_with(builder, PRODUCTION_MODULE_CONTRACTS)
}

/// Builder-time election with an explicit table (tests and other hosts).
pub fn attach_with(
    builder: RuntimeBuilder,
    contracts: &[(&str, ModuleToolContract)],
) -> Result<RuntimeBuilder, String> {
    builder
        .with_module_contracts(contracts)
        .map_err(|error| error.to_string())
}
