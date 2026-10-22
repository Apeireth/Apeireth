//! R8-B2 (ADR-0010 D3) 治理单一入口装配面。
//!
//! 生产治理栈 (pipeline 外包 permission preset 包装层) 作为
//! [`GovernanceChain`] 的 Gate 段, 再包成 [`ChainGovernanceHook`] 交给
//! runtime 的 `with_governance` —— 每次治理评估都过 `GovernanceChain::run`
//! 单点, preset 仍在链内 gate 位、语义不变; 未配置时 runtime 默认
//! `DenyUnconfigured` fail-closed 不变。链入口与旧入口四态行为等价真账:
//! `tests/governance_chain_equivalence.rs`。

use std::sync::Arc;

use apeireth_governance::{
    AuditSink, ChainGovernanceHook, GovernanceChain, GovernanceHook, InMemoryAuditSink,
};

/// 把治理 gate 包成 chain 单入口 hook (R8-B2 装配根切换; 内存审计默认)。
///
/// 返回值即 runtime 认识的 [`GovernanceHook`]; 裁决种类与 gate 直连等价,
/// verdict 元数据按 R8-B1 适配器映射如实标 `governance_chain`。
pub fn adapt_gate(gate: Arc<dyn GovernanceHook>) -> Arc<dyn GovernanceHook> {
    adapt_gate_with(gate, Arc::new(InMemoryAuditSink::new()))
}

/// chain 单入口 hook + 显式审计 sink (R8-B3 注入面)。
pub fn adapt_gate_with(
    gate: Arc<dyn GovernanceHook>,
    audit: Arc<dyn AuditSink>,
) -> Arc<dyn GovernanceHook> {
    let chain = Arc::new(GovernanceChain::new(gate).with_audit_sink(audit));
    Arc::new(ChainGovernanceHook::new(chain))
}

/// 生产装配 (R8-B2 + R8-B3): gate → chain 单入口 hook + 持久审计 sink。
///
/// 审计账本 = `APEIRETH_DATA_DIR`/`~/.apeireth` 下 `governance-audit.jsonl`
/// (与面板/会话库同根)。账本开不出时**如实降级内存账并打印** (ADR-0010 D4
/// 「持久化通过 AuditSink 注入实现, 默认仍是内存」; 降级 = 审计不持久、重启后
/// 无链可 verify —— 喊破, 不静默、不冒充); `run()` 期追加失败仍 fail-closed
/// (`ChainOutcome::Denied`), 降级只影响持久性、不影响守门。
pub fn production(gate: impl GovernanceHook + 'static) -> Arc<dyn GovernanceHook> {
    let audit: Arc<dyn AuditSink> = match apeireth_gateway::FileChainAuditSink::open(
        crate::default_panel_data_dir().join("governance-audit.jsonl"),
    ) {
        Ok(sink) => Arc::new(sink),
        Err(error) => {
            eprintln!(
                "[governance-audit] 持久审计账本不可用: {error}; 降级内存账 (审计不持久, 如实)"
            );
            Arc::new(InMemoryAuditSink::new())
        }
    };
    adapt_gate_with(Arc::new(gate), audit)
}
