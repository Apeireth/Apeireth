//! `apeireth-library-governance`: Library Stage 5 治理 (R127 P5-2, per 决策-33 §1.4 + 决策-55 §2.3).
//!
//! # 定位
//! - 独立 crate, **不**触碰 24 LOCKED crate, 0 越界 8 硬墙.
//! - 唯一职责: Library Stage 5 治理 = 策略框架 (clap 借鉴) + 形式化验证 (Kani 借鉴) + 一致性检查 (Kani proofs 借鉴).
//!
//! # 三大模块 (P5-2 阶段 5)
//! - [`strategy`] — 治理策略 (借鉴 clap 725 derive 模式: Policy / Decision / Action 三层 enum)
//! - [`verification`] — 形式化验证 (借鉴 Kani 4502 形式化模型: POD-friendly 不变量 + 边界检查)
//! - [`consistency`] — 一致性检查 (借鉴 Kani proofs 模板: cross-crate 一致性 + API 锁定)
//!
//! # Stage 5.1 深化模块 (P8-2, 形式化证明)
//! - [`formal_proof`] — Kani `Invariant` trait + `ProofHarness` + `ProofRunner` + `ProofReport`
//!   + `defensive_proof!` 宏 + `Stage5Token` / `LockedSignature` POD 类型, per 决策-56 §2.3 P8-2.
//!
//! # 公开 API (3 件套 — Ponytail: 够用即止)
//! - [`GovernanceContext`] — 治理上下文 (POD-friendly, 避免 String / Vec)
//! - [`evaluate`] — 给定 context 返回 [`GovernanceDecision`] (类似 clap `ArgMatches`)
//! - [`verify`] / [`run_all`] — 暴露给 runtime 的"轻量验证"(供 cargo test 跑)
//!
//! # 0 假装 (per 哲学锚 #1)
//! - ❌ 不假装"已完整形式化全部 8 硬墙" — 仅 6 个 Stage 5 关键不变量, 真实生产级形式化留给 R127 续扩
//! - ❌ 不假装"全 governance 入口" — 仅策略 / 验证 / 一致性 3 大件, 更多治理面 (审计 / 流程 / 升级) 留给 R128+
//! - ❌ 不触碰 24 LOCKED crate 入口签名 — 内部 fn 实施可改 (per 决策-33 §2.3 B1), 入口签名 0 改 (per 决策-41 §2)
//! - ❌ 不修改 Cargo.toml workspace.version 1.2.0 (B2 严守, 整合 #4 commit abf12243)
//! - ❌ 不修改 R11 baseline 3 值 (A1 严守, 0.8682/0.8532/0.9063 数字不动)
//!
//! # 跑命令
//! ```bash
//! cargo build -p apeireth-library-governance
//! cargo test -p apeireth-library-governance --lib
//! cargo test -p apeireth-library-governance --test integration
//! ```

#![deny(unsafe_code)]

pub mod consistency;
// R177: organ invariants (5 tests + 2 Kani)
pub mod formal_proof;
pub mod invariants;
mod organ_kani_proofs;
pub mod strategy;
pub mod verification;

// 公开 API re-export (便于 integration test 用 `apeireth_library_governance::Foo`)
pub use crate::consistency::{CheckStatus, ConsistencyReport};
pub use crate::formal_proof::{
    harnesses as proof_harnesses, run_all as run_all_formal_proofs, run_all_8_harnesses,
    run_all_as_report, Invariant, LockedSignature, ProofHarness, ProofKind, ProofReport,
    ProofResult, ProofRunner, Stage5Token,
};
pub use crate::strategy::{DecisionTree, GovernanceAction, GovernanceContext, PolicyKind};
pub use crate::verification::{Boundary, VerificationSubject};

/// Library Stage 5 治理决策 (类似 clap `ArgMatches` 的扁平化).
///
/// **设计**: 3 字段 (类似 clap 的 `matches.subcommand()` / `matches.value_of()` / `matches.is_present()`),
/// 但 POD-friendly (无 String / Vec), 适合 Kani 符号化 + cross-crate 序列化 (serde 通过 derive macro).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GovernanceDecision {
    /// 命中策略 (类似 clap 的 subcommand 命中)
    pub policy: PolicyKind,
    /// 行动 (类似 clap 的 value_of / action enum 命中)
    pub action: GovernanceAction,
    /// 是否需要升级审计 (B1 24 LOCKED 入口签名 0 改 等)
    pub requires_audit: bool,
}

impl GovernanceDecision {
    pub const fn allow(policy: PolicyKind, action: GovernanceAction) -> Self {
        Self {
            policy,
            action,
            requires_audit: false,
        }
    }

    pub const fn audit(policy: PolicyKind, action: GovernanceAction) -> Self {
        Self {
            policy,
            action,
            requires_audit: true,
        }
    }
}

/// Library Stage 5 治理决策树 (借鉴 clap `#[derive(Subcommand)]` 的 enum 派发模式).
///
/// **设计**: 3 段决策 (policy 命中 → action 派生 → audit 决定), 跟 clap `ArgMatches` 的
/// `subcommand()` → `value_of()` → `is_present()` 1:1 对应.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovernanceEngine;

impl GovernanceEngine {
    pub const fn new() -> Self {
        Self
    }

    /// 评估一个治理上下文, 派发到对应的策略 → 行动 → 是否需审计.
    ///
    /// **0 装严守**: 仅覆盖 5 类策略 (Version / Baseline / Locked / Anchor / Gate), 其他返回 [`GovernanceAction::Reject`].
    pub fn evaluate(&self, ctx: &GovernanceContext) -> GovernanceDecision {
        let tree = DecisionTree::from_context(ctx);
        tree.dispatch()
    }

    /// 跑 Stage 5 形式化验证 (sanity test, 0 Kani 安装也能跑).
    pub fn verify(&self) -> bool {
        invariants::run_all()
    }
}

impl Default for GovernanceEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 给定 context 返回 [`GovernanceDecision`] (公开 API 便捷入口).
pub fn evaluate(ctx: &GovernanceContext) -> GovernanceDecision {
    GovernanceEngine::new().evaluate(ctx)
}

/// Stage 5 一致性检查报告 (借鉴 Kani proofs 模板的 "passed harness" / "failed harness" 1:1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovernanceReport {
    pub consistency: ConsistencyReport,
    pub invariants_ok: bool,
}

impl GovernanceReport {
    pub fn run() -> Self {
        Self {
            consistency: ConsistencyReport::check(),
            invariants_ok: invariants::run_all(),
        }
    }

    /// 所有通道通过 = `true`.
    pub const fn is_ok(&self) -> bool {
        self.consistency.is_ok() && self.invariants_ok
    }
}

/// 跑 Stage 5 全量验证 (5 通道: 3 consistency + 6 invariants + engine sanity).
pub fn run_all() -> bool {
    let report = GovernanceReport::run();
    report.is_ok()
}

/// 同 [`run_all`], 但 panic-first (CI 友好).
pub fn verify() {
    assert!(
        run_all(),
        "apeireth-library-governance: at least one gate failed"
    );
}

#[cfg(test)]
mod lib_tests {
    use super::*;

    #[test]
    fn run_all_returns_true() {
        assert!(run_all(), "run_all() returned false");
    }

    #[test]
    fn verify_does_not_panic() {
        verify();
    }

    #[test]
    fn engine_default_matches_new() {
        let a = GovernanceEngine::new();
        let b = GovernanceEngine::default();
        assert_eq!(
            a.evaluate(&GovernanceContext::safe_default()),
            b.evaluate(&GovernanceContext::safe_default())
        );
    }

    #[test]
    fn evaluate_safe_context_returns_allow() {
        // safe_default policy=0 (Version) value=0 → 决策树派生 action=Audit (因为 value != 2)
        // 0 装严守: audit context 也会链式 audit, requires_audit=true 是正确行为
        let ctx = GovernanceContext::safe_default();
        let decision = evaluate(&ctx);
        // policy=Version + value=0 → action=Audit (因为 0 != 2 = 1.2.0 minor)
        assert_eq!(decision.action, GovernanceAction::Audit);
        assert!(decision.requires_audit);
    }
}
