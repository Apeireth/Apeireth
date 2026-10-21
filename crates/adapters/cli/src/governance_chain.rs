//! R8-B2 (ADR-0010 D3) 治理单一入口装配面。
//!
//! 生产治理栈 (pipeline 外包 permission preset 包装层) 作为
//! [`GovernanceChain`] 的 Gate 段, 再包成 [`ChainGovernanceHook`] 交给
//! runtime 的 `with_governance` —— 每次治理评估都过 `GovernanceChain::run`
//! 单点, preset 仍在链内 gate 位、语义不变; 未配置时 runtime 默认
//! `DenyUnconfigured` fail-closed 不变。链入口与旧入口四态行为等价真账:
//! `tests/governance_chain_equivalence.rs`。

use std::sync::Arc;

use apeireth_governance::{ChainGovernanceHook, GovernanceChain, GovernanceHook};

/// 把治理 gate 包成 chain 单入口 hook (R8-B2 装配根切换)。
///
/// 返回值即 runtime 认识的 [`GovernanceHook`]; 裁决种类与 gate 直连等价,
/// verdict 元数据按 R8-B1 适配器映射如实标 `governance_chain`。
pub fn adapt_gate(gate: Arc<dyn GovernanceHook>) -> Arc<dyn GovernanceHook> {
    let chain = Arc::new(GovernanceChain::new(gate));
    Arc::new(ChainGovernanceHook::new(chain))
}
