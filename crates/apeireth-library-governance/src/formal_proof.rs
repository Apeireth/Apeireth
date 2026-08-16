//! Library Stage 5.1 治理 — 形式化证明 (深化 P5-2, per 决策-56 §2.3 P8-2).
//!
//! # 定位
//! - 独立模块, **不**触碰 24 LOCKED crate, 0 越界 8 硬墙.
//! - 唯一职责: 形式化证明机制 (Invariant trait + ProofHarness + ProofRunner + ProofReport + 错误报告).
//! - 0 引 kani crate 依赖: 借鉴 Kani 4502 模式, 全部 runtime 实施, 0 装"已 Kani 形式化".
//!
//! # 借鉴来源 (per 决策-56 §2.3 P8-2)
//! - `model-checking/kani` v4502 — 形式化模型 + proofs 模板 (R125-10 ✅ done)
//! - 模式: Invariant trait (1 个 `is_safe` 方法) + `#[kani::proof]` harness + POD-friendly
//! - 1:1 翻译: Kani `kani::Invariant` → 我们 [`Invariant`] trait, Kani `kani::proof` → 我们 [`proof_harness`] 函数
//!
//! # 1:1 翻译
//! - Kani `kani::Invariant::is_safe(&self) -> bool` (borrowed from `library/kani/src/invariant.rs:90`)
//!   → 我们 [`Invariant::is_safe`]
//! - Kani `trivial_invariant!` macro (borrowed from `library/kani/src/invariant.rs:98`)
//!   → 我们 [`trivial_invariant!`]
//! - Kani `HarnessMetadata { pretty_name, mangled_name, original_file, original_start_line, ... }`
//!   (borrowed from `kani_metadata/src/harness.rs:22`) → 我们 [`ProofHarness`]
//! - Kani `HarnessKind { Proof, ProofForContract { target_fn }, Test }` (borrowed from
//!   `kani_metadata/src/harness.rs:65`) → 我们 [`ProofKind`]
//! - Kani `VerificationStatus { Success, Failure }` (borrowed from
//!   `kani-driver/src/call_cbmc.rs:34`) → 我们 [`ProofResult::Success`] / [`ProofResult::Failure`]
//! - Kani `kani::any()` (symbolic) → 我们 `safe_default()` 兜底, cargo test 模式
//! - Kani `kani::assume(cond)` → 我们 [`defensive_proof!`] 宏, runtime 强制断言
//! - Kani `MyDate` (borrowed from `library/kani/src/invariant.rs:32`) → 我们 [`Stage5Token`]
//!
//! # 0 触碰 Kani 本体
//! - 仅借鉴 POD + Invariant trait + harness 模板模式, 0 引 kani crate 依赖
//! - 0 装严守: `#[cfg_attr(kani, kani::proof)]` 兜底 (借鉴 P5-2), Kani 离线时退化为普通 fn
//!
//! # 0 装严守
//! - ❌ 0 假装"已 Kani 形式化" — Invariant trait + runtime check, Kani 求解器 = R128 续扩
//! - ❌ 0 假装"覆盖 8 硬墙全部" — 仅 6 Stage 5 关键不变量 + Stage5Token + LockedSignature
//! - ❌ 0 假装"运行时验证 = 形式化证明" — sanity check 跟 Kani 形式化是 2 通道 (per 哲学锚 #1)
//! - ❌ 0 假装"已装 kani" — 0 引 kani dep, 0 跑 `cargo kani`
//!
//! # 公开 API
//! - [`Invariant`] — 类型安全不变量 trait (Kani 1:1)
//! - [`ProofHarness`] — 证明元数据 (Kani `HarnessMetadata` 1:1)
//! - [`ProofResult`] — 单次证明结果 (Kani `VerificationStatus` 1:1)
//! - [`ProofRunner`] — 跑 harness 列表
//! - [`ProofReport`] — 聚合报告 (Kani `HarnessResult` 1:1)
//! - [`Stage5Token`] — Stage 5 治理 token POD (Kani `MyDate` 1:1)
//! - [`LockedSignature`] — B1 24 LOCKED 入口签名 POD (1:1)
//! - [`defensive_proof!`] — runtime 防御性断言 (Kani `kani::assume` 1:1)
//! - [`proof_harness`] — 8 Kani-style proof harness (cfg_attr 兜底)

#![allow(clippy::module_name_repetitions)]

use crate::verification::VerificationSubject;

// ============================================================================
// §1. Invariant trait (Kani 1:1, borrowed from library/kani/src/invariant.rs:90)
// ============================================================================

/// 类型安全不变量 trait (Kani `kani::Invariant` 1:1).
///
/// **设计**: 1 个方法 `is_safe(&self) -> bool`, 借鉴 Kani 1:1 (`library/kani/src/invariant.rs:90`).
/// 用法: 类型实现 `Invariant`, 然后用 [`ProofRunner`] 跑 `is_safe()` 自动验证.
///
/// Kani 原文:
/// > This trait should be used to specify and check type safety invariants for a type.
///
/// **0 装严守**: 这是 trait 抽象 (跟 Kani 一样), 0 假装"形式化证明" — 需要 Kani 求解器
/// 才能形式化证明. 我们做 runtime sanity check, Kani 形式化留 R128 续扩.
pub trait Invariant
where
    Self: Sized,
{
    /// 检查 `&self` 是否满足安全不变量 (Kani `is_safe` 1:1).
    ///
    /// **Kani 1:1 语义**:
    /// - 返回 `true` = 数据满足不变量, 安全
    /// - 返回 `false` = 数据违反不变量, 不安全
    fn is_safe(&self) -> bool;
}

/// 给原生类型实现 `is_safe() = true` 的宏 (Kani `trivial_invariant!` 1:1, 借鉴
/// `library/kani/src/invariant.rs:98`).
///
/// **Kani 原文**:
/// ```ignore
/// macro_rules! trivial_invariant {
///     ( $type: ty ) => {
///         impl Invariant for $type {
///             #[inline(always)]
///             fn is_safe(&self) -> bool {
///                 true
///             }
///         }
///     };
/// }
/// ```
///
/// **0 装严守**: 原生类型的所有值都"安全" (Rust 类型系统已经保证). 形式化证明需要 Kani,
/// 我们仅做 runtime check, Kani 留 R128.
#[macro_export]
macro_rules! trivial_invariant {
    ( $type:ty ) => {
        impl $crate::formal_proof::Invariant for $type {
            #[inline(always)]
            fn is_safe(&self) -> bool {
                true
            }
        }
    };
}

// 15 个原生类型的 trivial Invariant impl (Kani 1:1, 借鉴 `library/kani/src/invariant.rs:109-133`).
//
// 0 装严守: 0 装 f16/f128 (需 nightly `feature(f16)` / `feature(f128)`, Kani 原文有, 我们省略).
// 仅 15 类型: u8/u16/u32/u64/u128/usize + i8/i16/i32/i64/i128/isize + () + bool + char.
trivial_invariant!(u8);
trivial_invariant!(u16);
trivial_invariant!(u32);
trivial_invariant!(u64);
trivial_invariant!(u128);
trivial_invariant!(usize);

trivial_invariant!(i8);
trivial_invariant!(i16);
trivial_invariant!(i32);
trivial_invariant!(i64);
trivial_invariant!(i128);
trivial_invariant!(isize);

trivial_invariant!(());
trivial_invariant!(bool);
trivial_invariant!(char);

// ============================================================================
// §2. ProofKind + ProofHarness (Kani HarnessKind + HarnessMetadata 1:1)
// ============================================================================

/// 证明类型 (Kani `HarnessKind` 1:1, 借鉴 `kani_metadata/src/harness.rs:65`).
///
/// **Kani 原文**:
/// ```ignore
/// pub enum HarnessKind {
///     #[strum(serialize = "#[kani::proof]")]
///     Proof,
///     #[strum(serialize = "#[kani::proof_for_contract]")]
///     ProofForContract { target_fn: String },
///     #[strum(serialize = "#[test]")]
///     Test,
/// }
/// ```
///
/// **设计**: 3 类 — Proof (形式化证明) / ProofForContract (合同形式化) / Test (普通单元测试).
/// 0 装严守: 我们仅 8 个 Proof harness, 0 假装合同形式化.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProofKind {
    /// `#[kani::proof]` 形式化证明.
    Proof,
    /// `#[kani::proof_for_contract(target_fn)]` 合同形式化 (0 装, 仅 enum, R128 续扩).
    ProofForContract,
    /// `#[test]` 普通单元测试 (sanity 兜底).
    Test,
}

impl ProofKind {
    /// Kani 序列化字符串 (借鉴 `#[strum(serialize = "...")]` 1:1).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proof => "#[kani::proof]",
            Self::ProofForContract => "#[kani::proof_for_contract]",
            Self::Test => "#[test]",
        }
    }
}

/// 证明 harness 元数据 (Kani `HarnessMetadata` 1:1, 借鉴 `kani_metadata/src/harness.rs:22`).
///
/// **Kani 原文** (节选):
/// ```ignore
/// pub struct HarnessMetadata {
///     pub pretty_name: String,
///     pub mangled_name: String,
///     pub crate_name: String,
///     pub original_file: String,
///     pub original_start_line: usize,
///     pub original_end_line: usize,
///     pub goto_file: Option<PathBuf>,
///     pub attributes: HarnessAttributes,
///     pub contract: Option<AssignsContract>,
///     pub has_loop_contracts: bool,
///     pub is_automatically_generated: bool,
/// }
/// ```
///
/// **设计**: 我们用 5 字段 (name/file/line/kind/should_panic), POD-friendly (无 String/Vec),
/// 0 假装"全 HarnessMetadata" — 0 借 `mangled_name` / `goto_file` / `contract` 等 Kani 特有字段.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProofHarness {
    /// 用户给定的函数名 (类似 Kani `pretty_name`).
    pub name: &'static str,
    /// harness 所在文件 (类似 Kani `original_file`).
    pub file: &'static str,
    /// harness 起始行号 (类似 Kani `original_start_line`).
    pub line: u32,
    /// harness 类型 (类似 Kani `kind`).
    pub kind: ProofKind,
    /// harness 是否预期 panic (类似 Kani `should_panic`).
    pub should_panic: bool,
}

impl ProofHarness {
    /// 构造一个 Proof harness (最常用).
    pub const fn proof(name: &'static str, file: &'static str, line: u32) -> Self {
        Self {
            name,
            file,
            line,
            kind: ProofKind::Proof,
            should_panic: false,
        }
    }

    /// 构造一个 Test harness (兜底, 借用 cargo test).
    pub const fn test(name: &'static str, file: &'static str, line: u32) -> Self {
        Self {
            name,
            file,
            line,
            kind: ProofKind::Test,
            should_panic: false,
        }
    }
}

// ============================================================================
// §3. ProofResult (Kani VerificationStatus 1:1)
// ============================================================================

/// 单次证明结果 (Kani `VerificationStatus` 1:1, 借鉴 `kani-driver/src/call_cbmc.rs:34`).
///
/// **Kani 原文**:
/// ```ignore
/// pub enum VerificationStatus {
///     Success,
///     Failure,
/// }
/// ```
///
/// **设计**: 我们 3 状态 — Success / Failure { harness, message } / Skipped { reason }.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofResult {
    /// 证明成功 (类似 Kani `Verification successful`).
    Success,
    /// 证明失败, 含 harness 名字 + 错误信息 (类似 Kani `Failed assertion`).
    Failure {
        /// 失败的 harness 名字.
        harness: &'static str,
        /// 错误信息 (Kani 输出 CBMC trace, 我们用 1 行 string).
        message: &'static str,
    },
    /// 证明跳过, 含跳过原因 (类似 Kani `Unreachable` / `Undetermined`).
    Skipped {
        /// 跳过原因.
        reason: &'static str,
    },
}

impl ProofResult {
    /// 是否成功 (1:1 跟 Kani `status == Success`).
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }

    /// 是否失败 (1:1 跟 Kani `status == Failure`).
    pub const fn is_failure(self) -> bool {
        matches!(self, Self::Failure { .. })
    }

    /// 是否跳过.
    pub const fn is_skipped(self) -> bool {
        matches!(self, Self::Skipped { .. })
    }
}

// ============================================================================
// §4. ProofRunner + ProofReport (Kani HarnessRunner + HarnessResult 1:1)
// ============================================================================

/// 证明 runner (Kani `HarnessRunner` 1:1, 借鉴 `kani-driver/src/harness_runner.rs:23`).
///
/// **Kani 原文** (节选):
/// ```ignore
/// pub(crate) struct HarnessRunner<'sess, 'pr> {
///     pub sess: &'sess KaniSession,
///     pub project: &'pr Project,
/// }
/// ```
///
/// **设计**: 我们 0 借 Kani session, 仅跑闭包 (`FnOnce() -> ProofResult`), 0 装"全 HarnessRunner".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ProofRunner;

impl ProofRunner {
    /// 构造一个 ProofRunner.
    pub const fn new() -> Self {
        Self
    }

    /// 跑单个 harness 闭包, 返 [`ProofResult`].
    ///
    /// **设计**: 0 借 Kani 求解器, 直接调闭包 (cargo test 模式). Kani 求解器留 R128 续扩.
    pub fn run<F: FnOnce() -> ProofResult>(self, f: F) -> ProofResult {
        f()
    }

    /// 跑单个断言 (返 bool → 转 `ProofResult`).
    ///
    /// **便利函数**: 简化 ProofResult 构造.
    pub fn check(self, harness_name: &'static str, cond: bool) -> ProofResult {
        if cond {
            ProofResult::Success
        } else {
            ProofResult::Failure {
                harness: harness_name,
                message: "assertion failed",
            }
        }
    }
}

/// 证明报告 (Kani `HarnessResult` 1:1, 借鉴 `kani-driver/src/harness_runner.rs:32`).
///
/// **Kani 原文**:
/// ```ignore
/// pub(crate) struct HarnessResult<'pr> {
///     pub harness: &'pr HarnessMetadata,
///     pub result: VerificationResult,
/// }
/// ```
///
/// **设计**: 我们用 Vec 存 (harness, result) 对, 含 pass/fail/skipped 计数.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProofReport {
    /// (harness, result) 对列表.
    pub entries: Vec<(ProofHarness, ProofResult)>,
}

impl ProofReport {
    /// 新建空报告.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 加一条 (harness, result) 记录.
    pub fn record(&mut self, harness: ProofHarness, result: ProofResult) {
        self.entries.push((harness, result));
    }

    /// 成功数.
    pub fn pass_count(&self) -> usize {
        self.entries.iter().filter(|(_, r)| r.is_success()).count()
    }

    /// 失败数.
    pub fn fail_count(&self) -> usize {
        self.entries.iter().filter(|(_, r)| r.is_failure()).count()
    }

    /// 跳过数.
    pub fn skipped_count(&self) -> usize {
        self.entries.iter().filter(|(_, r)| r.is_skipped()).count()
    }

    /// 总数.
    pub fn total(&self) -> usize {
        self.entries.len()
    }

    /// 是否全过 (0 失败) — 1:1 跟 Kani `Verification successful` 状态.
    pub fn is_ok(&self) -> bool {
        self.fail_count() == 0
    }
}

// ============================================================================
// §5. defensive_proof! macro (Kani kani::assume 1:1, runtime 版本)
// ============================================================================

/// Runtime 防御性断言 (Kani `kani::assume(cond)` 1:1, runtime 版本).
///
/// **Kani 模式** (`library/kani/src/lib.rs`):
/// ```ignore
/// kani::assume(cond);
/// ```
/// 告诉 Kani 求解器 "假设 cond 为真", 求解器会用 cond 约束后续证明.
///
/// **Runtime 模式 (我们)**:
/// ```rust,ignore
/// defensive_proof!("harness_name", cond);
/// // 返 ProofResult::Success 或 ProofResult::Failure { harness, message }
/// ```
/// 在 runtime 把 cond 当作强制断言, 失败返 `ProofResult::Failure`.
///
/// **用法示例**:
/// ```rust
/// use apeireth_library_governance::defensive_proof;
/// use apeireth_library_governance::formal_proof::ProofResult;
///
/// let x: u8 = 5;
/// let r: ProofResult = defensive_proof!("x_positive", x > 0);
/// assert!(r.is_success());
///
/// let r2: ProofResult = defensive_proof!("x_positive", x > 100);
/// assert!(r2.is_failure());
/// ```
#[macro_export]
macro_rules! defensive_proof {
    ( $harness:expr, $cond:expr ) => {{
        if !$cond {
            $crate::formal_proof::ProofResult::Failure {
                harness: $harness,
                message: concat!("defensive_proof failed: ", stringify!($cond),),
            }
        } else {
            $crate::formal_proof::ProofResult::Success
        }
    }};
}

// ============================================================================
// §6. impl Invariant for VerificationSubject (深度集成 P5-2 verification)
// ============================================================================

/// Stage 5 验证主对象的不变量 (深度集成 P5-2 `VerificationSubject`).
///
/// **设计**: 6 字段 (B2 major / B2 minor / A1 baseline / B1 locked / B5 anchor / B4 gate)
/// 全严守 → `is_safe() = true`. 任何字段偏离整合 #4 commit 严守状态 → `is_safe() = false`.
///
/// **0 装严守**: 这是 runtime check, 0 假装"形式化证明" — Kani 求解器留 R128 续扩.
impl Invariant for VerificationSubject {
    fn is_safe(&self) -> bool {
        // 6 验证不变量全过 (B2 / B2 / A1 / B1 / B5 / B4)
        self.version_major == 1
            && self.version_minor == 2
            && self.baseline_index == 0
            && self.locked_signatures_intact
            && self.anchor_count == 8
            && self.gate_layers == 6
    }
}

// ============================================================================
// §7. Stage5Token — Kani MyDate 1:1 例子 (borrowed from library/kani/src/invariant.rs:32)
// ============================================================================

/// Stage 5.1 治理 token (POD-friendly, 借鉴 Kani `MyDate` 1:1).
///
/// **Kani 原文** (`library/kani/src/invariant.rs:32`):
/// ```ignore
/// #[derive(kani::Arbitrary)]
/// pub struct MyDate {
///   day: u8,
///   month: u8,
///   year: i64,
/// }
/// ```
///
/// **我们的 1:1 翻译**: 6 字段对应 6 Stage 5 不变量, 全部 u8 / bool, Kani-friendly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stage5Token {
    /// B2: workspace.version major (严守 1).
    pub version_major: u8,
    /// B2: workspace.version minor (严守 2).
    pub version_minor: u8,
    /// A1: baseline index (严守 0 = R11).
    pub baseline_index: u8,
    /// B1: locked signatures intact (严守 true).
    pub locked_signatures_intact: bool,
    /// B5: anchor count (严守 8).
    pub anchor_count: u8,
    /// B4: gate layers (严守 6).
    pub gate_layers: u8,
}

impl Stage5Token {
    /// 整合 #4 commit abf12243 严守状态 (safe_default).
    pub const fn safe_default() -> Self {
        Self {
            version_major: 1,
            version_minor: 2,
            baseline_index: 0,
            locked_signatures_intact: true,
            anchor_count: 8,
            gate_layers: 6,
        }
    }

    /// 工厂函数 (借鉴 Kani `Percentage::try_new` 1:1, borrowed from
    /// `tests/kani/Invariant/percentage.rs:16`).
    ///
    /// 返回 `Result<Self, &'static str>`, 错误信息是 1 行 string (Kani 1:1 风格).
    pub fn try_new(
        version_major: u8,
        version_minor: u8,
        baseline_index: u8,
        locked_signatures_intact: bool,
        anchor_count: u8,
        gate_layers: u8,
    ) -> Result<Self, &'static str> {
        if version_major != 1 {
            return Err("version_major must be 1 (B2 严守 1.x)");
        }
        if version_minor != 2 {
            return Err("version_minor must be 2 (B2 严守 1.2.0)");
        }
        if baseline_index != 0 {
            return Err("baseline_index must be 0 (A1 严守 R11)");
        }
        if !locked_signatures_intact {
            return Err("locked_signatures_intact must be true (B1 严守 0 改)");
        }
        if anchor_count != 8 {
            return Err("anchor_count must be 8 (B5 6→8 升级)");
        }
        if gate_layers != 6 {
            return Err("gate_layers must be 6 (B4 6 重 v7)");
        }
        Ok(Self {
            version_major,
            version_minor,
            baseline_index,
            locked_signatures_intact,
            anchor_count,
            gate_layers,
        })
    }
}

/// Stage5Token 的不变量 (Kani `impl Invariant for MyDate` 1:1, 借鉴
/// `library/kani/src/invariant.rs:50`).
impl Invariant for Stage5Token {
    fn is_safe(&self) -> bool {
        self.version_major == 1
            && self.version_minor == 2
            && self.baseline_index == 0
            && self.locked_signatures_intact
            && self.anchor_count == 8
            && self.gate_layers == 6
    }
}

// ============================================================================
// §8. LockedSignature — B1 24 LOCKED 1:1 POD
// ============================================================================

/// 单个 LOCKED crate 入口签名 (B1 24 LOCKED 1:1).
///
/// **设计**: 2 字段 (index, signature_intact), 代表单个 LOCKED crate 入口签名严守状态.
/// 0 装严守: 仅 POD, 0 引实际 24 LOCKED 列表 (P5-2 已 hardcode `LOCKED_CRATE_COUNT = 24`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LockedSignature {
    /// LOCKED crate 索引 (0..=23, 整合 #4 commit P2-3 verify 24/24 done).
    pub index: u8,
    /// 入口签名是否 intact (整合 #4 commit 后 P2-3 verify 全 0 改).
    pub signature_intact: bool,
}

impl LockedSignature {
    /// 安全默认: index=0, signature_intact=true.
    pub const fn safe_default() -> Self {
        Self {
            index: 0,
            signature_intact: true,
        }
    }

    /// 工厂函数: index 必须 ≤ 23, signature_intact 必须 true.
    pub fn try_new(index: u8, signature_intact: bool) -> Result<Self, &'static str> {
        if index > 23 {
            return Err("index must be 0..=23 (B1 24 LOCKED, 整合 #4 P2-3 verify)");
        }
        if !signature_intact {
            return Err("signature_intact must be true (B1 24 LOCKED 入口签名 0 改)");
        }
        Ok(Self {
            index,
            signature_intact,
        })
    }

    /// 总 LOCKED 数量 (编译期 hardcode, 跟 P5-2 consistency 同步).
    pub const TOTAL: u8 = 24;
}

impl Invariant for LockedSignature {
    fn is_safe(&self) -> bool {
        self.index < Self::TOTAL && self.signature_intact
    }
}

// ============================================================================
// §9. 8 Kani-style proof harness (cfg_attr 兜底, cargo test 模式跑)
// ============================================================================

/// 8 Kani-style proof harness (借鉴 P5-2 `verification::harnesses` 1:1, 加上
/// Stage5Token / LockedSignature 2 个新 harness).
///
/// **Kani 模式**: `#[kani::proof] fn harness_name() { assert!(invariant); }` (借鉴
/// `tests/kani/Invariant/invariant_impls.rs:20`).
///
/// **Runtime 模式 (我们)**: `#[cfg_attr(kani, kani::proof)]` 兜底, Kani 离线时退化为普通 fn
/// (cargo test 跑). 0 装严守: Kani 求解器留 R128 续扩.
pub mod harnesses {
    use super::{
        Invariant, LockedSignature, ProofHarness, ProofResult, ProofRunner, Stage5Token,
        VerificationSubject,
    };
    use crate::verification::nondet_subject;

    // -------- B2: workspace.version major (1) --------

    /// Harness B2-1: workspace.version major = 1 (Kani `check_safe_type!(u8)` 风格).
    #[cfg_attr(kani, kani::proof)]
    pub fn proof_version_major_is_one() -> ProofResult {
        let s = nondet_subject();
        if s.version_major == 1 {
            ProofResult::Success
        } else {
            ProofResult::Failure {
                harness: "proof_version_major_is_one",
                message: "version_major != 1 (B2 严守 1.x)",
            }
        }
    }

    /// Harness B2-2: workspace.version minor = 2.
    #[cfg_attr(kani, kani::proof)]
    pub fn proof_version_minor_is_two() -> ProofResult {
        let s = nondet_subject();
        if s.version_minor == 2 {
            ProofResult::Success
        } else {
            ProofResult::Failure {
                harness: "proof_version_minor_is_two",
                message: "version_minor != 2 (B2 严守 1.2.0)",
            }
        }
    }

    // -------- A1: R11 baseline (0) --------

    /// Harness A1-1: baseline index = 0 (R11).
    #[cfg_attr(kani, kani::proof)]
    pub fn proof_baseline_index_is_r11() -> ProofResult {
        let s = nondet_subject();
        if s.baseline_index == 0 {
            ProofResult::Success
        } else {
            ProofResult::Failure {
                harness: "proof_baseline_index_is_r11",
                message: "baseline_index != 0 (A1 严守 R11)",
            }
        }
    }

    // -------- B1: 24 LOCKED signatures intact --------

    /// Harness B1-1: locked signatures intact.
    #[cfg_attr(kani, kani::proof)]
    pub fn proof_locked_signatures_intact() -> ProofResult {
        let s = nondet_subject();
        if s.locked_signatures_intact {
            ProofResult::Success
        } else {
            ProofResult::Failure {
                harness: "proof_locked_signatures_intact",
                message: "locked_signatures_intact = false (B1 严守 24 LOCKED 入口签名 0 改)",
            }
        }
    }

    // -------- B5: 8 anchor count --------

    /// Harness B5-1: anchor count = 8.
    #[cfg_attr(kani, kani::proof)]
    pub fn proof_anchor_count_is_eight() -> ProofResult {
        let s = nondet_subject();
        if s.anchor_count == 8 {
            ProofResult::Success
        } else {
            ProofResult::Failure {
                harness: "proof_anchor_count_is_eight",
                message: "anchor_count != 8 (B5 6→8 哲学锚升级)",
            }
        }
    }

    // -------- B4: 6 gate layers (v7) --------

    /// Harness B4-1: gate layers = 6.
    #[cfg_attr(kani, kani::proof)]
    pub fn proof_gate_layers_is_six() -> ProofResult {
        let s = nondet_subject();
        if s.gate_layers == 6 {
            ProofResult::Success
        } else {
            ProofResult::Failure {
                harness: "proof_gate_layers_is_six",
                message: "gate_layers != 6 (B4 6 重守门 v7)",
            }
        }
    }

    // -------- Stage 5.1 专属: Stage5Token Invariant (Kani MyDate 1:1) --------

    /// Harness 5.1-1: Stage5Token::safe_default().is_safe() (Kani `check_increase_safe` 1:1,
    /// 借鉴 `tests/kani/Invariant/percentage.rs:54`).
    #[cfg_attr(kani, kani::proof)]
    pub fn proof_stage5_token_safe_default_holds() -> ProofResult {
        let token = Stage5Token::safe_default();
        ProofRunner::new().check("proof_stage5_token_safe_default_holds", token.is_safe())
    }

    // -------- Stage 5.1 专属: LockedSignature Invariant (B1 1:1) --------

    /// Harness 5.1-2: LockedSignature::safe_default().is_safe() (B1 1:1).
    #[cfg_attr(kani, kani::proof)]
    pub fn proof_locked_signature_safe_default_holds() -> ProofResult {
        let sig = LockedSignature::safe_default();
        ProofRunner::new().check("proof_locked_signature_safe_default_holds", sig.is_safe())
    }

    // -------- 8 个 harness 列表 (供 ProofRunner 跑) --------

    /// 8 个 harness 的元数据 (Kani `HarnessMetadata` 1:1 风格).
    ///
    /// **设计**: 8 个静态 `ProofHarness` 描述, 跟 [`proof_version_major_is_one`] 等 1:1 对应.
    /// 用法: `ProofRunner::new().run_with_report(&mut report, harnesses::ALL)` (R128 续扩).
    /// 当前: 单元测试 + 集成测试直接调各个 harness 函数.
    pub const ALL: [ProofHarness; 8] = [
        ProofHarness::proof(
            "proof_version_major_is_one",
            file!(),
            line!(), // 编译期 line!() 不可用, 留 0
                     // ↑ 注: 静态数组不能调 line!() (非常量), 实际 line 在测试里 override
        ),
        ProofHarness::proof("proof_version_minor_is_two", file!(), 0),
        ProofHarness::proof("proof_baseline_index_is_r11", file!(), 0),
        ProofHarness::proof("proof_locked_signatures_intact", file!(), 0),
        ProofHarness::proof("proof_anchor_count_is_eight", file!(), 0),
        ProofHarness::proof("proof_gate_layers_is_six", file!(), 0),
        ProofHarness::proof("proof_stage5_token_safe_default_holds", file!(), 0),
        ProofHarness::proof("proof_locked_signature_safe_default_holds", file!(), 0),
    ];
}

// ============================================================================
// §10. 跑全部 8 harness (1:1 跟 P5-2 `invariants::run_all`)
// ============================================================================

/// 跑全部 8 Kani-style harness, 返 8 元素的 `ProofResult` 数组.
pub fn run_all_8_harnesses() -> [ProofResult; 8] {
    [
        harnesses::proof_version_major_is_one(),
        harnesses::proof_version_minor_is_two(),
        harnesses::proof_baseline_index_is_r11(),
        harnesses::proof_locked_signatures_intact(),
        harnesses::proof_anchor_count_is_eight(),
        harnesses::proof_gate_layers_is_six(),
        harnesses::proof_stage5_token_safe_default_holds(),
        harnesses::proof_locked_signature_safe_default_holds(),
    ]
}

/// 8 harness 全过 = true (1:1 跟 P5-2 `run_all`).
pub fn run_all() -> bool {
    run_all_8_harnesses().iter().all(|r| r.is_success())
}

/// 跑全部 8 harness 并装到 ProofReport (Kani `HarnessRunner::check_all_harnesses` 1:1).
pub fn run_all_as_report() -> ProofReport {
    let mut report = ProofReport::new();
    let results = run_all_8_harnesses();
    for (harness, result) in harnesses::ALL.iter().copied().zip(results.iter().copied()) {
        report.record(harness, result);
    }
    report
}

// ============================================================================
// §11. 单元测试 (Kani `tests/kani/Invariant/invariant_impls.rs` 1:1)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formal_proof::harnesses as proof_harnesses;

    // -------- trivial_invariant! 15 类型 --------

    #[test]
    fn trivial_invariant_u8_holds() {
        let v: u8 = 42;
        assert!(v.is_safe());
        let v: u8 = 0;
        assert!(v.is_safe());
        let v: u8 = 255;
        assert!(v.is_safe());
    }

    #[test]
    fn trivial_invariant_u16_u32_u64_u128_usize_holds() {
        let v: u16 = u16::MAX;
        assert!(v.is_safe());
        let v: u32 = u32::MAX;
        assert!(v.is_safe());
        let v: u64 = u64::MAX;
        assert!(v.is_safe());
        let v: u128 = u128::MAX;
        assert!(v.is_safe());
        let v: usize = usize::MAX;
        assert!(v.is_safe());
    }

    #[test]
    fn trivial_invariant_i8_i16_i32_i64_i128_isize_holds() {
        let v: i8 = i8::MIN;
        assert!(v.is_safe());
        let v: i16 = i16::MIN;
        assert!(v.is_safe());
        let v: i32 = i32::MIN;
        assert!(v.is_safe());
        let v: i64 = i64::MIN;
        assert!(v.is_safe());
        let v: i128 = i128::MIN;
        assert!(v.is_safe());
        let v: isize = isize::MIN;
        assert!(v.is_safe());
    }

    #[test]
    fn trivial_invariant_unit_bool_char_holds() {
        let v: () = ();
        assert!(v.is_safe());
        let v: bool = true;
        assert!(v.is_safe());
        let v: bool = false;
        assert!(v.is_safe());
        let v: char = '🦀';
        assert!(v.is_safe());
        let v: char = '\0';
        assert!(v.is_safe());
    }

    // -------- impl Invariant for VerificationSubject --------

    #[test]
    fn verification_subject_safe_default_is_safe() {
        let s = VerificationSubject::safe_default();
        assert!(s.is_safe());
    }

    #[test]
    fn verification_subject_violating_version_major_not_safe() {
        let s = VerificationSubject {
            version_major: 2,
            ..VerificationSubject::safe_default()
        };
        assert!(!s.is_safe());
    }

    #[test]
    fn verification_subject_violating_baseline_not_safe() {
        let s = VerificationSubject {
            baseline_index: 1,
            ..VerificationSubject::safe_default()
        };
        assert!(!s.is_safe());
    }

    #[test]
    fn verification_subject_violating_locked_not_safe() {
        let s = VerificationSubject {
            locked_signatures_intact: false,
            ..VerificationSubject::safe_default()
        };
        assert!(!s.is_safe());
    }

    // -------- Stage5Token (Kani MyDate 1:1) --------

    #[test]
    fn stage5_token_safe_default_is_safe() {
        let t = Stage5Token::safe_default();
        assert!(t.is_safe());
    }

    #[test]
    fn stage5_token_try_new_ok() {
        let t = Stage5Token::try_new(1, 2, 0, true, 8, 6);
        assert!(t.is_ok());
        let t = t.unwrap();
        assert!(t.is_safe());
    }

    #[test]
    fn stage5_token_try_new_version_major_violates() {
        let r = Stage5Token::try_new(2, 0, 0, true, 8, 6);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), "version_major must be 1 (B2 严守 1.x)");
    }

    #[test]
    fn stage5_token_try_new_anchor_count_violates() {
        let r = Stage5Token::try_new(1, 2, 0, true, 7, 6);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), "anchor_count must be 8 (B5 6→8 升级)");
    }

    #[test]
    fn stage5_token_violating_field_not_safe() {
        let t = Stage5Token {
            anchor_count: 7,
            ..Stage5Token::safe_default()
        };
        assert!(!t.is_safe());
    }

    // -------- LockedSignature (B1 1:1) --------

    #[test]
    fn locked_signature_safe_default_is_safe() {
        let s = LockedSignature::safe_default();
        assert!(s.is_safe());
        assert_eq!(s.index, 0);
        assert!(s.signature_intact);
    }

    #[test]
    fn locked_signature_all_24_in_range() {
        for i in 0u8..24 {
            let s = LockedSignature::try_new(i, true).unwrap();
            assert!(s.is_safe());
        }
    }

    #[test]
    fn locked_signature_index_25_violates() {
        let r = LockedSignature::try_new(25, true);
        assert!(r.is_err());
        assert_eq!(
            r.unwrap_err(),
            "index must be 0..=23 (B1 24 LOCKED, 整合 #4 P2-3 verify)"
        );
    }

    #[test]
    fn locked_signature_broken_intact_violates() {
        let r = LockedSignature::try_new(0, false);
        assert!(r.is_err());
        assert_eq!(
            r.unwrap_err(),
            "signature_intact must be true (B1 24 LOCKED 入口签名 0 改)"
        );
    }

    #[test]
    fn locked_signature_total_is_24() {
        assert_eq!(LockedSignature::TOTAL, 24);
    }

    // -------- ProofKind --------

    #[test]
    fn proof_kind_as_str_matches_kani() {
        assert_eq!(ProofKind::Proof.as_str(), "#[kani::proof]");
        assert_eq!(
            ProofKind::ProofForContract.as_str(),
            "#[kani::proof_for_contract]"
        );
        assert_eq!(ProofKind::Test.as_str(), "#[test]");
    }

    // -------- ProofHarness --------

    #[test]
    fn proof_harness_proof_constructor() {
        let h = ProofHarness::proof("test", "test.rs", 42);
        assert_eq!(h.name, "test");
        assert_eq!(h.file, "test.rs");
        assert_eq!(h.line, 42);
        assert_eq!(h.kind, ProofKind::Proof);
        assert!(!h.should_panic);
    }

    #[test]
    fn proof_harness_test_constructor() {
        let h = ProofHarness::test("test", "test.rs", 10);
        assert_eq!(h.kind, ProofKind::Test);
    }

    // -------- ProofResult --------

    #[test]
    fn proof_result_is_success() {
        assert!(ProofResult::Success.is_success());
        assert!(!ProofResult::Success.is_failure());
        assert!(!ProofResult::Success.is_skipped());
    }

    #[test]
    fn proof_result_is_failure() {
        let r = ProofResult::Failure {
            harness: "h",
            message: "m",
        };
        assert!(r.is_failure());
        assert!(!r.is_success());
    }

    #[test]
    fn proof_result_is_skipped() {
        let r = ProofResult::Skipped {
            reason: "kani offline",
        };
        assert!(r.is_skipped());
        assert!(!r.is_success());
    }

    // -------- ProofRunner --------

    #[test]
    fn proof_runner_run_success() {
        let r = ProofRunner::new().run(|| ProofResult::Success);
        assert!(r.is_success());
    }

    #[test]
    fn proof_runner_run_failure() {
        let r = ProofRunner::new().run(|| ProofResult::Failure {
            harness: "x",
            message: "y",
        });
        assert!(r.is_failure());
    }

    #[test]
    fn proof_runner_check_true() {
        let r = ProofRunner::new().check("x", true);
        assert!(r.is_success());
    }

    #[test]
    fn proof_runner_check_false() {
        let r = ProofRunner::new().check("x", false);
        assert!(r.is_failure());
    }

    // -------- ProofReport --------

    #[test]
    fn proof_report_empty_is_ok() {
        let r = ProofReport::new();
        assert!(r.is_ok());
        assert_eq!(r.pass_count(), 0);
        assert_eq!(r.fail_count(), 0);
        assert_eq!(r.total(), 0);
    }

    #[test]
    fn proof_report_record_and_count() {
        let mut r = ProofReport::new();
        r.record(ProofHarness::proof("a", "f", 1), ProofResult::Success);
        r.record(ProofHarness::proof("b", "f", 2), ProofResult::Success);
        r.record(
            ProofHarness::proof("c", "f", 3),
            ProofResult::Failure {
                harness: "c",
                message: "failed",
            },
        );
        r.record(
            ProofHarness::test("d", "f", 4),
            ProofResult::Skipped { reason: "r" },
        );
        assert_eq!(r.total(), 4);
        assert_eq!(r.pass_count(), 2);
        assert_eq!(r.fail_count(), 1);
        assert_eq!(r.skipped_count(), 1);
        assert!(!r.is_ok());
    }

    // -------- 8 Kani-style proof harness --------

    #[test]
    fn proof_8_harnesses_all_pass() {
        let results = run_all_8_harnesses();
        for r in &results {
            assert!(r.is_success(), "harness failed: {:?}", r);
        }
    }

    #[test]
    fn proof_run_all_returns_true() {
        assert!(run_all());
    }

    #[test]
    fn proof_run_all_as_report_has_8_entries_all_pass() {
        let r = run_all_as_report();
        assert_eq!(r.total(), 8);
        assert_eq!(r.pass_count(), 8);
        assert_eq!(r.fail_count(), 0);
        assert!(r.is_ok());
    }

    // -------- defensive_proof! 宏 --------

    #[test]
    fn defensive_proof_macro_passes_on_true() {
        let r = defensive_proof!("h1", 5 > 0);
        assert!(r.is_success());
    }

    #[test]
    fn defensive_proof_macro_fails_on_false() {
        let r = defensive_proof!("h1", 5 < 0);
        assert!(r.is_failure());
        if let ProofResult::Failure { harness, message } = r {
            assert_eq!(harness, "h1");
            assert!(message.contains("5 < 0"), "message was: {}", message);
        } else {
            panic!("expected Failure variant");
        }
    }

    #[test]
    fn defensive_proof_macro_complex_condition() {
        let x: u8 = 10;
        let y: u8 = 20;
        let r = defensive_proof!("x_lt_y", x < y);
        assert!(r.is_success());

        let r = defensive_proof!("x_gt_y", x > y);
        assert!(r.is_failure());
    }

    // -------- 0 装严守 --------

    #[test]
    fn zero_kani_dependency_no_kani_use() {
        // 文档化保证: formal_proof 模块 0 引用 kani::*
        // (本测试本身只是占位, 实际验证在 cargo build 时由编译器检查)
        // 0 kani::any / 0 kani::proof / 0 kani::Invariant
        // 仅借鉴 Kani 模式, 0 装"已 Kani 形式化"
        let token = Stage5Token::safe_default();
        assert!(token.is_safe());
    }

    #[test]
    fn proof_harness_metadata_count_is_eight() {
        // ALL 数组应 = 8 harness (跟 run_all_8_harnesses 1:1)
        assert_eq!(proof_harnesses::ALL.len(), 8);
    }
}
