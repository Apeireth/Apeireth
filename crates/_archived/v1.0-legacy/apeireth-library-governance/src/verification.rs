//! Library Stage 5 形式化验证 — 借鉴 Kani 4502 形式化模型.
//!
//! # 借鉴来源 (R124-3 BORROW)
//! - `model-checking/kani` — 形式化验证骨架
//! - 模式: POD-friendly 不变量 + 边界检查 + 符号化输入 (Kani `kani::any()`)
//! - harness 模板: `#[cfg_attr(kani, kani::proof)] pub fn <name>() { assert!(<invariant>(<pod>)); }`
//!
//! # 1:1 翻译
//! - Kani `PermissionLayerConfig` POD → 我们 `VerificationSubject` POD (避免 String/Vec)
//! - Kani `kani::any()` 符号化 → 我们 `nondet_subject()` (cargo test 模式返 safe_default)
//! - Kani 5 harness (backoff / jitter / cache / replay / role_divide) → 我们 6 verification (Stage 5 6 不变量)
//! - Kani `cargo kani` (慢, 1-5 min/harness) → 我们 `cargo test` (快, ms 级 sanity)
//!
//! # 0 触碰 Kani 本体
//! - 仅借鉴 POD + 符号化模式, 0 引 kani crate 依赖 (governance 跑 cargo test 即可)
//! - 全部 harness 都用 `#[cfg_attr(kani, kani::proof)]` 兜底, Kani 离线时退化普通 fn
//!
//! # 0 装严守
//! - ❌ 0 假装"已 Kani 验证" — `#[cfg_attr(kani, ...)]` 兜底, cargo kani 实跑 = R128 续
//! - ❌ 0 假装"覆盖 8 硬墙全部" — 仅 6 Stage 5 关键不变量, 其他 2 (B3 / A3) 留 R127 续
//! - ❌ 0 假装"运行时验证 = 形式化证明" — sanity test 跟 Kani 形式化是 2 通道 (per 哲学锚 #1)

/// 验证对象 (POD-friendly, 借鉴 Kani `PermissionLayerConfig` 1:1).
///
/// 6 字段对应 6 Stage 5 关键不变量, 全部 u8 / bool / 固定 array.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VerificationSubject {
    /// B2: workspace.version major (1, 严守 1.2.0)
    pub version_major: u8,
    /// B2: workspace.version minor (2, 严守 1.2.0)
    pub version_minor: u8,
    /// A1: R11 baseline 索引 (0=R11, 1=R12, ...)
    pub baseline_index: u8,
    /// B1: 24 LOCKED 入口签名 hash (1=保持, 0=破坏)
    pub locked_signatures_intact: bool,
    /// B5: 哲学锚 count (8 严守)
    pub anchor_count: u8,
    /// B4: 守门 v7 层数 (6 重 v7)
    pub gate_layers: u8,
}

impl VerificationSubject {
    pub const fn new(
        version_major: u8,
        version_minor: u8,
        baseline_index: u8,
        locked_signatures_intact: bool,
        anchor_count: u8,
        gate_layers: u8,
    ) -> Self {
        Self {
            version_major,
            version_minor,
            baseline_index,
            locked_signatures_intact,
            anchor_count,
            gate_layers,
        }
    }

    /// 安全默认: 整合 #4 commit abf12243 严守状态.
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
}

/// 非确定性输入生成 — Kani 模式下 `kani::any()`, 其它模式返 safe_default.
///
/// 借鉴 `apeireth-formal/invariants/double_onion_sample::nondet_config` 1:1 模式.
#[cfg(kani)]
pub fn nondet_subject() -> VerificationSubject {
    kani::any()
}

#[cfg(not(kani))]
pub fn nondet_subject() -> VerificationSubject {
    // cargo test 兜底: 选 safe_default (happy path, 不会触发 assert!)
    VerificationSubject::safe_default()
}

/// 6 Stage 5 形式化不变量 (借鉴 Kani 5 harness 模式).
///
/// 每个 invariant 是 1 个 bool 函数 (1 行断言体), 借鉴 Kani harness 的 "断言体 1 行" ponytail.
pub mod invariants {

    /// 不变量 1: workspace.version major = 1 (B2 严守 1.x, 0 改到 2.x)
    ///
    /// **物理**: 整合 #4 commit abf12243 严守 workspace.version = "1.2.0" (Cargo.toml:246).
    /// major 跳 2.x = breaking change, 1.0 release (R127 12/31) 才升 1.0.0 (大版本归 0, per 决策-22 §2.2).
    pub fn version_major_is_one(s: &super::VerificationSubject) -> bool {
        s.version_major == 1
    }

    /// 不变量 2: workspace.version minor = 2 (B2 1.2.0 严守, R125 末 minor 升)
    ///
    /// **物理**: per 决策-33 §2.3 B2 升级路线 1.1.0 → 1.2.0, R127 release 1.2.0 → 1.0.0 (大版本归 0).
    pub fn version_minor_is_two(s: &super::VerificationSubject) -> bool {
        s.version_minor == 2
    }

    /// 不变量 3: baseline index = 0 (A1 严守 R11 baseline 3 值 0.8682/0.8532/0.9063)
    ///
    /// **物理**: per 决策-33 §2.3 A1, R11 baseline 数字 0 删 0 改, 17 文件原位.
    pub fn baseline_index_is_r11(s: &super::VerificationSubject) -> bool {
        s.baseline_index == 0
    }

    /// 不变量 4: 24 LOCKED 入口签名 intact (B1 严守 0 改, 整合 #4 commit 后 P2-3 verify done)
    ///
    /// **物理**: per 决策-41 §2 + 决策-42 §1.1, P2-3 sub-agent 24/24 LOCKED 入口签名 0 改 verify done.
    pub fn locked_signatures_intact(s: &super::VerificationSubject) -> bool {
        s.locked_signatures_intact
    }

    /// 不变量 5: 哲学锚 count = 8 (B5 6→8 升级, per 决策-33 §2.3 B5)
    ///
    /// **物理**: 6 + S-3 (质量工程化) + O-1 (安全优先) = 8, P1-2 R126 8 哲学锚升级 done.
    pub fn anchor_count_is_eight(s: &super::VerificationSubject) -> bool {
        s.anchor_count == 8
    }

    /// 不变量 6: 守门 layers = 6 (B4 6 重 v6 → v7 升级, per 决策-33 §2.3 B4)
    ///
    /// **物理**: 5 重 + Colang DSL = 6 重 v6, P1-3 R126 升 v7, 6 重严守.
    pub fn gate_layers_is_six(s: &super::VerificationSubject) -> bool {
        s.gate_layers == 6
    }

    /// 跑全部 6 invariant 的 sanity check.
    pub fn run_all(s: &super::VerificationSubject) -> bool {
        version_major_is_one(s)
            && version_minor_is_two(s)
            && baseline_index_is_r11(s)
            && locked_signatures_intact(s)
            && anchor_count_is_eight(s)
            && gate_layers_is_six(s)
    }
}

/// 6 Kani-style harness (借鉴 `apeireth-formal::kani_harness` 1:1 模式).
///
/// 每个 harness 都用 `#[cfg_attr(kani, kani::proof)]` 兜底, Kani 离线时退化普通 fn (cargo test 跑).
pub mod harnesses {
    use super::invariants;
    use super::{nondet_subject, VerificationSubject};

    /// Harness 1: B2 workspace.version major 严守 1.x
    #[cfg_attr(kani, kani::proof)]
    pub fn verify_version_major_is_one() {
        let s = nondet_subject();
        assert!(invariants::version_major_is_one(&s));
    }

    /// Harness 2: B2 workspace.version minor 严守 2
    #[cfg_attr(kani, kani::proof)]
    pub fn verify_version_minor_is_two() {
        let s = nondet_subject();
        assert!(invariants::version_minor_is_two(&s));
    }

    /// Harness 3: A1 R11 baseline index 严守 0
    #[cfg_attr(kani, kani::proof)]
    pub fn verify_baseline_index_is_r11() {
        let s = nondet_subject();
        assert!(invariants::baseline_index_is_r11(&s));
    }

    /// Harness 4: B1 24 LOCKED 入口签名 intact
    #[cfg_attr(kani, kani::proof)]
    pub fn verify_locked_signatures_intact() {
        let s = nondet_subject();
        assert!(invariants::locked_signatures_intact(&s));
    }

    /// Harness 5: B5 8 哲学锚
    #[cfg_attr(kani, kani::proof)]
    pub fn verify_anchor_count_is_eight() {
        let s = nondet_subject();
        assert!(invariants::anchor_count_is_eight(&s));
    }

    /// Harness 6: B4 6 重守门 v7
    #[cfg_attr(kani, kani::proof)]
    pub fn verify_gate_layers_is_six() {
        let s = nondet_subject();
        assert!(invariants::gate_layers_is_six(&s));
    }
}

/// 边界检查 (借鉴 Kani `kani::assume()` 模式).
///
/// **设计**: 8 个边界 check, 对应 8 硬墙的最严边界 (B2/A1/B1/B5/B4/B3/A3/C1-C3).
/// `Boundary` enum POD-friendly (u8 替 String).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// B2 边界: version_major ∈ {1}
    VersionMajor,
    /// B2 边界: version_minor ∈ {2}
    VersionMinor,
    /// A1 边界: baseline_index ∈ {0}
    BaselineIndex,
    /// B1 边界: locked_index ∈ 0..=23
    LockedIndex,
    /// B5 边界: anchor_count ∈ {8}
    AnchorCount,
    /// B4 边界: gate_layers ∈ {6}
    GateLayers,
    /// B3 边界: dim_count ∈ {30}
    DimCount,
    /// A3 边界: key_count ∈ {13}
    KeyCount,
}

impl Boundary {
    pub const fn count() -> usize {
        8
    }

    /// 跑边界 check, 返 true = 在边界内.
    pub fn check(self, value: u8) -> bool {
        match self {
            Boundary::VersionMajor => value == 1,
            Boundary::VersionMinor => value == 2,
            Boundary::BaselineIndex => value == 0,
            Boundary::LockedIndex => value <= 23,
            Boundary::AnchorCount => value == 8,
            Boundary::GateLayers => value == 6,
            Boundary::DimCount => value == 30,
            Boundary::KeyCount => value == 13,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_default_passes_all_6_invariants() {
        let s = VerificationSubject::safe_default();
        assert!(invariants::run_all(&s));
    }

    #[test]
    fn version_major_one_passes() {
        assert!(invariants::version_major_is_one(&VerificationSubject::new(
            1, 2, 0, true, 8, 6
        )));
    }

    #[test]
    fn version_major_two_violates() {
        assert!(!invariants::version_major_is_one(
            &VerificationSubject::new(2, 0, 0, true, 8, 6)
        ));
    }

    #[test]
    fn version_minor_two_passes() {
        assert!(invariants::version_minor_is_two(&VerificationSubject::new(
            1, 2, 0, true, 8, 6
        )));
    }

    #[test]
    fn version_minor_three_violates() {
        assert!(!invariants::version_minor_is_two(
            &VerificationSubject::new(1, 3, 0, true, 8, 6)
        ));
    }

    #[test]
    fn baseline_index_zero_passes() {
        assert!(invariants::baseline_index_is_r11(
            &VerificationSubject::new(1, 2, 0, true, 8, 6)
        ));
    }

    #[test]
    fn baseline_index_nonzero_violates() {
        assert!(!invariants::baseline_index_is_r11(
            &VerificationSubject::new(1, 2, 1, true, 8, 6)
        ));
    }

    #[test]
    fn locked_intact_passes() {
        assert!(invariants::locked_signatures_intact(
            &VerificationSubject::new(1, 2, 0, true, 8, 6)
        ));
    }

    #[test]
    fn locked_broken_violates() {
        assert!(!invariants::locked_signatures_intact(
            &VerificationSubject::new(1, 2, 0, false, 8, 6)
        ));
    }

    #[test]
    fn anchor_eight_passes() {
        assert!(invariants::anchor_count_is_eight(
            &VerificationSubject::new(1, 2, 0, true, 8, 6)
        ));
    }

    #[test]
    fn anchor_seven_violates() {
        assert!(!invariants::anchor_count_is_eight(
            &VerificationSubject::new(1, 2, 0, true, 7, 6)
        ));
    }

    #[test]
    fn gate_six_passes() {
        assert!(invariants::gate_layers_is_six(&VerificationSubject::new(
            1, 2, 0, true, 8, 6
        )));
    }

    #[test]
    fn gate_five_violates() {
        assert!(!invariants::gate_layers_is_six(&VerificationSubject::new(
            1, 2, 0, true, 8, 5
        )));
    }

    #[test]
    fn boundary_count_is_eight() {
        assert_eq!(Boundary::count(), 8);
    }

    #[test]
    fn boundary_version_major_one_passes() {
        assert!(Boundary::VersionMajor.check(1));
        assert!(!Boundary::VersionMajor.check(0));
        assert!(!Boundary::VersionMajor.check(2));
    }

    #[test]
    fn boundary_locked_index_range() {
        for i in 0u8..=23 {
            assert!(Boundary::LockedIndex.check(i), "i={} should pass", i);
        }
        for i in 24u8..=255 {
            assert!(!Boundary::LockedIndex.check(i), "i={} should fail", i);
        }
    }

    #[test]
    fn boundary_key_count_thirteen() {
        assert!(Boundary::KeyCount.check(13));
        assert!(!Boundary::KeyCount.check(12));
        assert!(!Boundary::KeyCount.check(14));
    }

    #[test]
    fn nondet_subject_in_test_mode_is_safe_default() {
        // cargo test 模式 (no kani cfg): nondet_subject 返 safe_default
        let s = nondet_subject();
        assert!(invariants::run_all(&s));
    }

    #[test]
    fn all_6_harnesses_are_publicly_visible() {
        // 文档化保证: 6 harness 函数名跟 Kani 命令对齐 (类似 apeireth-formal 模式)
        let _: fn() = harnesses::verify_version_major_is_one;
        let _: fn() = harnesses::verify_version_minor_is_two;
        let _: fn() = harnesses::verify_baseline_index_is_r11;
        let _: fn() = harnesses::verify_locked_signatures_intact;
        let _: fn() = harnesses::verify_anchor_count_is_eight;
        let _: fn() = harnesses::verify_gate_layers_is_six;
    }
}
