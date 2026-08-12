//! `apeireth-formal`: 形式化验证 skeleton (V2 战区 5, docs/v2-strategy/03 §4A).
//!
//! # 定位
//! - 独立 crate, **不进 workspace build** 也不污染其他 crate 的编译图。
//! - 唯一职责: 暴露 Kani `#[kani::proof]` harness + 配套公开 API,
//!   让 `cargo kani --harness <name>` 能在本地与 CI 跑通。
//!
//! # 公开 API (3 件套 — Ponytail: 够用即止)
//! - [`PermissionLayerConfig`] — POD 模型, Kani-friendly (避免 String / Vec / heap)
//! - [`l0_requires_ha_invariant`] — 6 层权限洋葱的核心不变量: L0 必须 `requires_ha=true`
//! - [`run_all`] / [`verify`] — 暴露给 runtime 的"轻量验证"(供 cargo test 跑)
//!
//! # Kani 怎么用
//! ```bash
//! cargo install --locked kani-verifier && cargo install --locked cargo-kani
//! cargo kani --harness double_onion_sample
//! ```
//! 详见 `docs/kani-setup.md` 与 `.github/workflows/kani.yml`。
//!
//! # 禁止
//! - ❌ 不引入 `unsafe` (Kani 默认禁止, 本 crate 同步禁止)
//! - ❌ 不引入 String / Vec / HashMap 等堆类型作 harness 输入
//! -   (Kani 面对非确定性堆分配会状态爆炸)
//! - ❌ 不调用 `apeireth-core` / `apeireth-onion` 的真实结构体进 harness
//!   (它们的字段类型不可被 Kani 直接符号化)
//! - ❌ 不在 CI 默认 pipeline 跑 — Kani 单独 workflow (`kani.yml`), 不挡 PR

#![deny(unsafe_code)]

pub mod invariants;
pub mod error;
pub mod example;
pub mod invariant;
pub mod kani_harness;
pub mod proof;
pub mod tla;
// R127-2 P9-1: 5 NEW POD 模型 + 5 NEW Kani harness (借脑 1.0)
pub mod borrowed_models_v2;
// R129-10: Stage 5.2 形式化扩展 — 10 模块 (F1-F10) (per 决策 #33 + #55 + #61 §3.1)
pub mod stage5_2;
// R129-20: Stage 5.3 跨模块证明 — 10 模块 (F11-F20) (per 决策 #33 + #55 + #61 §3.1 R129-20)
pub mod stage5_3;
// R131.8: Self-Disable 5 机制 Kani harness (critical missing proof 1)
pub mod self_disable_harness;
// R133.1: 字符串 ownership_token 形式化 (5 机制用 StringPod + 5 Kani proof + 10 unit test)
pub mod self_disable_string_harness;
// R131.9: 9 重守门 + flush_noop Kani harness (critical missing 2+3)
pub mod nine_fold_harness;
// R149: L0 HA 物理多签 M-of-N Kani harness (补 R131.6 audit 缺的 critical proof 2)
pub mod l0_ha_physical_multisig;

/// 权限洋葱的最小 POD 配置 (Kani-friendly).
///
/// **不**复用 `apeireth_core::PermissionLayer`(它有 `String` 字段,
/// Kani 面对非确定性 `String` 会状态爆炸)。本 crate 自带极简模型,
/// 证明"形式属性"即可, 不需要 1:1 复制生产类型。
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PermissionLayerConfig {
    /// 层身份: `0..=5` 对应 L0..L5 (其他值视为非法, 不变量仍应保持)
    pub kind: u8,
    /// 是否要求人类权威 (HA) 真实人类批准
    pub requires_ha: bool,
}

impl PermissionLayerConfig {
    /// 创建指定层的配置 (测试 / 手工调用方便).
    pub const fn new(kind: u8, requires_ha: bool) -> Self {
        Self { kind, requires_ha }
    }
}

/// 权限洋葱层深度 = 6 (L0..L5).
///
/// 编译时 hardcode: 任何调用方都不能"自由"调整洋葱层数,
/// 因为这会破坏电子环 11 节点的不变量 (5 原则 + 6 权限)。
pub const PERMISSION_ONION_DEPTH: usize = 6;

/// 核心不变量: **L0 永远要求 HA**.
/// 对任意 `PermissionLayerConfig`, 若 `kind == 0` 则 `requires_ha == true`。
///
/// 物理含义: L0 是 HA 核心 (apeireth-core §1.4 "🛡️ 最后护栏"),
/// 失去 HA = 失去最后一道门, 架构层不允许。
pub fn l0_requires_ha_invariant(cfg: PermissionLayerConfig) -> bool {
    if cfg.kind == 0 {
        cfg.requires_ha
    } else {
        true
    }
}

/// 运行时轻量验证 (供 cargo test 跑, **不**替代 Kani 证明).
///
/// 返回所有不变量通过 = `true`; 任一失败 = `false`。
pub fn run_all() -> bool {
    invariants::run_all()
}

/// 同 [`run_all`], 但 panic-first (CI 友好).
pub fn verify() {
    assert!(run_all(), "apeireth-formal: at least one invariant failed");
}

#[cfg(test)]
mod lib_tests {
    use super::*;

    /// **lib 入口验证**: `run_all()` 应返回 true (所有不变量 sanity pass).
    #[test]
    fn run_all_returns_true() {
        assert!(run_all(), "run_all() returned false");
    }

    /// **lib 入口验证**: `verify()` 不 panic (用于 CI gate).
    #[test]
    fn verify_does_not_panic() {
        verify();
    }

    /// **PERMISSION_ONION_DEPTH 编译期守**: 6 层 (L0..L5) 永不变.
    #[test]
    fn permission_onion_depth_is_six() {
        assert_eq!(PERMISSION_ONION_DEPTH, 6);
    }
}

pub use error::{FormalError, FormalResult, ProofBackend};
pub use invariant::{Invariant, InvariantKind};
pub use proof::{BackendRegistry, Cvc5BackendImpl, CoqBackendImpl, Lean4BackendImpl, ProofKind, ProofResult, ProofStatus, Z3BackendImpl};
pub use tla::{TlaExpr, TlaSpec};

/// Unified proof engine dispatching a catalog invariant to a backend.
pub struct FormalEngine { registry: BackendRegistry }
impl FormalEngine { pub fn with_defaults()->Self{Self{registry:BackendRegistry::with_defaults()}} pub fn check_invariant(&self,invariant:&Invariant)->FormalResult<ProofResult>{self.registry.prove(invariant)} pub fn dispatch_by_name(&self,name:&str)->FormalResult<ProofResult>{let i=invariant::presets::ALL.iter().find(|i|i.name==name).ok_or_else(||FormalError::UnknownInvariant(name.into()))?;self.check_invariant(i)} pub fn health_check(&self)->bool{self.registry.health_check()} }


