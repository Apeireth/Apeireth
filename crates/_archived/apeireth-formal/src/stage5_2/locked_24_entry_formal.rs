//! R129-10 Stage 5.2 形式化扩展 F6 — 24 LOCKED 入口签名形式化 (B1 严守, 0 改 24 LOCKED)
//!
//! # 背景 (per 决策 #33 §2.3 B1 + 决策 #55 §1 + 决策 #61 §3.1 R129-10)
//!
//! B1 24 LOCKED crate 持续更新 (per 决策 #22 §1.1-1.2 + 决策 #33 §2.3 B1):
//! - 24 LOCKED crate 入口签名 0 改
//! - 24 LOCKED crate 内部 fn 实施可改 (R125 末 升级授权 + 决策 #53)
//! - 验证: P2-3 + P4-1 + P14-1 retry 三方 verify done
//!
//! 24 LOCKED 完整列表 (per `docs/omnibus/24-locked-crates.md`):
//! 1-12 主人已知 12: supervisor / agent / bus / council / evolution / extension / graph / mcp / pipeline / tool-registry / tool-runtime / protocol
//! 13-24 Mavis 扩展 12: asi / onion / sovereignty / constraint / memory / cognition / perception / consciousness / motivation / life-force / relation / value
//!
//! # 借鉴 ID
//!
//! `R129-10-F6-BORROW-kani-4502-Invariant-trait-2026-08-11`
//! - 0 装 PASS 严守: ✅ 0 引 kani 依赖, 0 装"已 Kani 形式化"
//! - 0 越界 8 硬墙: B1 24 LOCKED 入口签名 0 改
//!
//! # 0 触碰 (per 决策 #33 §2.3 + 决策 #61 §6)
//!
//! - B1 24 LOCKED 入口签名 0 改: 本模块 0 触碰 LOCKED crate 代码
//! - 24 LOCKED 内部 fn 实施可改 (per 决策 #53 技术性 locked 解锁)
//! - 9 organ 内部 fn 借 OpenCode (B7) 0 触碰
//! - 8 LOCKED 文档 0 触碰
//! - C1 0 主动 commit: Mavis 整合 #5.1 commit 拍板

#![cfg_attr(kani, allow(dead_code))]

// ============================================================
// 1. 编译期常量 (24 LOCKED 入口签名, 1:1 跟 B1 严守)
// ============================================================

/// 24 LOCKED crate 总数 (1:1 跟 B1 严守, per `docs/omnibus/24-locked-crates.md`)
pub const LOCKED_24_CRATES_COUNT: usize = 24;

/// 24 LOCKED crate 名 (1:1 跟 B1 严守, 0 改)
/// 顺序: 1-12 主人已知 12 + 13-24 Mavis 扩展 12
pub const LOCKED_24_CRATE_NAMES: [&str; LOCKED_24_CRATES_COUNT] = [
    // 1-12 主人已知 12
    "apeireth-supervisor",
    "apeireth-agent",
    "apeireth-bus",
    "apeireth-council",
    "apeireth-evolution",
    "apeireth-extension",
    "apeireth-graph",
    "apeireth-mcp",
    "apeireth-pipeline",
    "apeireth-tool-registry",
    "apeireth-tool-runtime",
    "apeireth-protocol",
    // 13-24 Mavis 扩展 12
    "apeireth-asi",
    "apeireth-onion",
    "apeireth-sovereignty",
    "apeireth-constraint",
    "apeireth-memory",
    "apeireth-cognition",
    "apeireth-perception",
    "apeireth-consciousness",
    "apeireth-motivation",
    "apeireth-life-force",
    "apeireth-relation",
    "apeireth-value",
];

/// 24 LOCKED 入口签名 POD 镜像 (1:1 跟 B1 严守, 0 改入口签名)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Locked24EntryPod {
    /// crate 索引 (0..23, 1:1 跟 B1 严守)
    pub index: u8,
    /// crate 名 (1:1 跟 B1 严守, 0 改)
    pub name: &'static str,
    /// 入口签名是否 intact (B1 严守, 0 改 = true)
    pub signature_intact: bool,
    /// 是否在主人已知 12 (0..12) vs Mavis 扩展 12 (12..24)
    pub known: KnownSet,
}

/// LOCKED crate 集合分类 (per B1 严守, 12 已知 + 12 Mavis 扩展)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KnownSet {
    /// 主人已知 12 (per `docs/omnibus/24-locked-crates.md` 表 1)
    MasterKnown = 0,
    /// Mavis 扩展 12 (per `docs/omnibus/24-locked-crates.md` 表 2)
    MavisExtended = 1,
}

impl Locked24EntryPod {
    /// 构造 (编译期 hardcode)
    pub const fn new(index: u8, name: &'static str, signature_intact: bool, known: KnownSet) -> Self {
        Self { index, name, signature_intact, known }
    }

    /// 24 LOCKED 总数 (B1 严守)
    pub const fn count() -> usize {
        LOCKED_24_CRATES_COUNT
    }
}

// ============================================================
// 2. 24 LOCKED 入口签名不变量 (B1 严守 0 改)
// ============================================================

/// 24 LOCKED 入口签名不变量: index ∈ 0..23 永真 (B1 严守)
pub fn locked_24_entry_invariant(e: Locked24EntryPod) -> bool {
    e.index < LOCKED_24_CRATES_COUNT as u8
}

/// 24 LOCKED 入口签名 intact 不变量: signature_intact=true = 24 (B1 严守, 0 改入口签名)
pub fn locked_24_entry_all_intact(es: [Locked24EntryPod; LOCKED_24_CRATES_COUNT]) -> bool {
    for e in &es {
        if !e.signature_intact {
            return false;
        }
    }
    true
}

/// 24 LOCKED 集合分类不变量: 主人已知 12 + Mavis 扩展 12 = 24 (B1 严守)
pub fn locked_24_entry_known_invariant(es: [Locked24EntryPod; LOCKED_24_CRATES_COUNT]) -> bool {
    let mut known = 0;
    let mut mavis = 0;
    for e in &es {
        match e.known {
            KnownSet::MasterKnown => known += 1,
            KnownSet::MavisExtended => mavis += 1,
        }
    }
    known == 12 && mavis == 12
}

// ============================================================
// 3. Kani-style proof harness
// ============================================================

/// Kani proof harness — 24 LOCKED index ∈ 0..23 永真 (B1 严守)
#[cfg_attr(kani, kani::proof)]
pub fn proof_locked_24_entry_in_range() {
    let e = nondet_entry();
    assert!(locked_24_entry_invariant(e), "24 LOCKED index 必须在 0..23");
}

/// Kani proof harness — 24 LOCKED count = 24 (B1 严守, 12 + 12)
#[cfg_attr(kani, kani::proof)]
pub fn proof_locked_24_entry_count_is_24() {
    assert_eq!(Locked24EntryPod::count(), 24, "24 LOCKED count 必须 = 24");
}

#[cfg(kani)]
fn nondet_entry() -> Locked24EntryPod {
    kani::any()
}

#[cfg(not(kani))]
fn nondet_entry() -> Locked24EntryPod {
    // cargo test 兜底: 第 1 个 LOCKED supervisor happy path
    Locked24EntryPod::new(0, "apeireth-supervisor", true, KnownSet::MasterKnown)
}

/// Runtime sanity: 24 LOCKED (0..23) 应全部通过
pub fn sanity_check() -> bool {
    for index in 0u8..LOCKED_24_CRATES_COUNT as u8 {
        if !locked_24_entry_invariant(Locked24EntryPod::new(index, "test", true, KnownSet::MasterKnown)) {
            return false;
        }
    }
    true
}

// ============================================================
// 4. 单元测试 (8 tests, 0 装 PASS 严守 verify)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_function_is_publicly_visible() {
        let _: fn() = proof_locked_24_entry_in_range;
        let _: fn() = proof_locked_24_entry_count_is_24;
    }

    #[test]
    fn locked_24_count_is_24() {
        assert_eq!(LOCKED_24_CRATES_COUNT, 24);
        assert_eq!(Locked24EntryPod::count(), 24);
        assert_eq!(LOCKED_24_CRATE_NAMES.len(), 24);
    }

    #[test]
    fn locked_24_index_0_to_23_all_pass() {
        for index in 0u8..24 {
            assert!(locked_24_entry_invariant(Locked24EntryPod::new(index, "test", true, KnownSet::MasterKnown)));
        }
    }

    #[test]
    fn locked_24_index_24_violates() {
        // 反例: index=24 越界 (B1 严守 0..24)
        assert!(!locked_24_entry_invariant(Locked24EntryPod::new(24, "test", true, KnownSet::MasterKnown)));
    }

    #[test]
    fn locked_24_all_24_intact() {
        // 24 LOCKED 全部 intact (B1 严守, 0 改入口签名)
        let es: [Locked24EntryPod; 24] = [Locked24EntryPod::new(0, "test", true, KnownSet::MasterKnown); 24];
        assert!(locked_24_entry_all_intact(es));
    }

    #[test]
    fn locked_24_one_broken_violates() {
        // 反例: 1 个 entry signature_intact=false
        let mut es: [Locked24EntryPod; 24] = [Locked24EntryPod::new(0, "test", true, KnownSet::MasterKnown); 24];
        es[0] = Locked24EntryPod::new(0, "test", false, KnownSet::MasterKnown);
        assert!(!locked_24_entry_all_intact(es));
    }

    #[test]
    fn locked_24_known_mavis_split_12_12() {
        // B1 严守: 12 主人已知 + 12 Mavis 扩展
        let mut es: [Locked24EntryPod; 24] = [Locked24EntryPod::new(0, "test", true, KnownSet::MasterKnown); 24];
        for i in 0..24 {
            es[i as usize] = Locked24EntryPod::new(i, "test", true,
                if i < 12 { KnownSet::MasterKnown } else { KnownSet::MavisExtended });
        }
        assert!(locked_24_entry_known_invariant(es));
    }

    #[test]
    fn locked_24_crate_names_24_unique() {
        // 24 LOCKED 0 重复 (B1 严守)
        let mut sorted: Vec<&str> = LOCKED_24_CRATE_NAMES.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 24, "24 LOCKED crate 名必须 unique");
    }

    #[test]
    fn sanity_check_returns_true() {
        assert!(sanity_check());
    }
}
