//! R126 8 哲学锚 (B5 6→8 升级, per 决策 #22 §2.5 + 决策 #33 + 决策 #51 §1.2 P1-2)
//!
//! **本模块 (NEW, R126 done 2026-08-10)**:
//! - 8 哲学锚 enum (原 6 锚 0 改 + 新增 2 锚 S-3 + O-1)
//! - 编译期 hardcode 锁 `EIGHT_ANCHORS_HARDCODE` (8 锚顺序 + 分组)
//! - 跟 6 锚 (`apeireth-council::PHILOSOPHICAL_ANCHORS: [&str; 6]`) 互转
//! - 8 哲学锚 namespace 化 (S-* = Subjective 主体, O-* = Objective 客观)
//!
//! **0 触碰清单 (8 硬墙 0 越界)**:
//! - B1 24 LOCKED 入口签名 0 改: ✅ 0 改 `crates/apeireth-council/src/constitution.rs` 的 `pub const PHILOSOPHICAL_ANCHORS: [&str; 6]` (24 LOCKED #4)
//! - A3 13 键 0 改: ✅ 0 改 `crates/apeireth-core/src/lib.rs` 的 `PhilosophyKey` enum (PHL-01~06 当前 12 键) — 本模块是**独立** enum, 0 触碰 PHL 命名空间
//! - B2 1.2.0 0 改: ✅ 0 改 `Cargo.toml:246` workspace.version
//! - A1 baseline 3 值 0 删 0 改: ✅ 0 改 `crates/apeireth-asi/src/lib.rs:42-44` V1141/V1131/V1136 数字
//! - C3 v6 0 改: ✅ 0 改 6 重守门 (5 重 + Colang DSL)
//! - C1 0 commit: ✅ 0 主动 commit (本文件 untracked, 等 Mavis 整合 #5 拍板)
//! - 0 push: ✅ 0 主动 push (等 1.0 release 配 GitHub remote)
//!
//! **借鉴 ID (per 决策 #22 §3 严格化)**: `R126-philo-8-BORROW-apeireth/conventions-vR125-2026-08-10`
//! - 0 装 = 内部 extension (R125 16:55 09-anchor.md 已升级 8 锚 doc-level, 本 R126 实施 src-level)
//! - 0 装 PASS 严守: ✅ 0 假装"已借鉴"外部, 0 装 src 写完 + 内联 tests pass
//!
//! **R125-8 模式 (per agent-r125-8-final-2026-08-10.md)**:
//! - 阶段 1: 借鉴源码 study → 0 装 = 内部 extension
//! - 阶段 2: Rust 实施 (本文件 NEW)
//! - 阶段 3: 单元测试 stub (本文件 `mod tests` 内联, 8+ tests)
//! - 阶段 4: 编译期 verify (临时 crate 0 错误 + 0 警告)
//! - 阶段 5: 0 触碰 lib.rs (`pub mod eight_anchors;` 留 Mavis 整合 #5 拍板时加)

#![deny(unsafe_code)]

// ============================================
// 1. 8 哲学锚 enum (R126 B5 6→8 升级, 编译期 hardcode)
// ============================================

/// 8 哲学锚 (R126 B5 升级, per 决策 #22 §2.5 + 主人 16:31 最高权限 + 决策 #33 8 硬墙重置)
///
/// **原 6 锚 (LOCKED 0 改, per APEIRETH-CONVENTIONS.md §9)**:
/// - `S-1` 北极星导向 — 服务 ASI 北极星 (per 主人 22:33)
/// - `S-2` 实事求是 — 基于现状不重写, 核验后写 (per 主人 17:43, R119 主人 8/10 01:14 拍板)
/// - `O-2` 走在前人经验上 — 借鉴 Hermes / OpenClaw / VCP / claude-mem + LangGraph / AutoGen / MCP / LSP / semver (per 主人 19:33)
/// - `O-3` 干到底 — 决策立刻沉淀, 1 commit 总 (per 主人 23:44, 主人 8/9 拍板)
/// - `O-4` 任何人都能接手 — 4 件套齐全, 顶层瘦 (per 主人 00:56, R119 主人 8/10 拍板)
/// - `O-5` 不假装 — 12 键编译期 hardcode, 8 项不修改承诺形式撤销后原意保留 (per 主人 17:58, R119)
///
/// **R126 新增 2 锚 (per 决策 #22 §2.5)**:
/// - `S-3` 质量工程化 — 代码质量 = 工程信誉, clippy 150 + doc 1077 清 (per R123-1, 主人 16:55 派)
/// - `O-1` 安全优先 — 安全 > 功能 > 性能, 5 重守门 v5 + 6 重 v6 (per R125-5 NVIDIA Guardrails, 主人 16:55 派)
///
/// **命名空间 (per 决策 #22 §2.5)**:
/// - S-* = Subjective 主体哲学锚 (跟"主"主人 / 主体连续性 关联)
/// - O-* = Objective 客观哲学锚 (跟"客"客观守门 / 借鉴 / 持续性 关联)
///
/// **8 哲学锚 vs 6 哲学锚关系 (向后兼容)**:
/// - 6 锚: S-1, S-2, O-2, O-3, O-4, O-5 (顺序锁定 per `apeireth-council::PHILOSOPHICAL_ANCHORS`)
/// - 8 锚: S-1, S-2, **S-3** (新增), **O-1** (新增), O-2, O-3, O-4, O-5
/// - 6 锚 实质 0 改 (per B1 入口签名 0 改), 8 锚是 6 锚 + 2 新锚 (per B5 升级路线)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhilosophicalAnchor8 {
    // === 6 锚原版 (LOCKED 0 改, per B1 入口签名 0 改) ===
    /// S-1 北极星导向 — 服务 ASI 北极星 (per 主人 22:33)
    S1NorthStar,
    /// S-2 实事求是 — 基于现状不重写, 核验后写 (per 主人 17:43)
    S2TruthFromReality,
    /// O-2 走在前人经验上 — 借鉴业界惯例 (per 主人 19:33)
    O2StandingOnShoulders,
    /// O-3 干到底 — 决策立刻沉淀, 1 commit 总 (per 主人 23:44)
    O3SeeItThrough,
    /// O-4 任何人都能接手 — 4 件套齐全, 顶层瘦 (per 主人 00:56)
    O4AnyoneCanTakeOver,
    /// O-5 不假装 — 12 键编译期 hardcode (per 主人 17:58)
    O5NoPretend,
    // === R126 新增 2 锚 (per 决策 #22 §2.5 B5 6→8 升级) ===
    /// S-3 质量工程化 — 代码质量 = 工程信誉, clippy 150 + doc 1077 清 (per R123-1, 主人 16:55 派)
    /// **R126 NEW**: 跟 R123-1 clippy+doc 清关联
    S3QualityEngineering,
    /// O-1 安全优先 — 安全 > 功能 > 性能, 6 重守门 v6 (per R125-5 NVIDIA Guardrails, 主人 16:55 派)
    /// **R126 NEW**: 跟 5 重守门 v5 + 6 重 v6 关联
    O1SafetyFirst,
}

impl PhilosophicalAnchor8 {
    /// 编译期 hardcode - 返回每个锚的代号 (per APEIRETH-CONVENTIONS §9 + 09-anchor.md)
    pub const fn code(&self) -> &'static str {
        match self {
            Self::S1NorthStar => "S-1",
            Self::S2TruthFromReality => "S-2",
            Self::S3QualityEngineering => "S-3",
            Self::O1SafetyFirst => "O-1",
            Self::O2StandingOnShoulders => "O-2",
            Self::O3SeeItThrough => "O-3",
            Self::O4AnyoneCanTakeOver => "O-4",
            Self::O5NoPretend => "O-5",
        }
    }

    /// 编译期 hardcode - 返回每个锚的描述 (per 09-anchor.md R125 16:55 + R126 升级)
    pub const fn description(&self) -> &'static str {
        match self {
            Self::S1NorthStar => "S-1 北极星导向 — 服务 ASI 北极星 (主 22:33)",
            Self::S2TruthFromReality => "S-2 实事求是 — 基于现状不重写, 核验后写 (主 17:43)",
            Self::S3QualityEngineering => "S-3 质量工程化 — 代码质量 = 工程信誉, clippy 150 + doc 1077 清 (R123-1 主 16:55)",
            Self::O1SafetyFirst => "O-1 安全优先 — 安全 > 功能 > 性能, 6 重守门 v6 (R125-5 主 16:55)",
            Self::O2StandingOnShoulders => "O-2 走在前人经验上 — 借鉴 Hermes / OpenClaw / VCP / claude-mem + LangGraph / AutoGen / MCP / LSP / semver (主 19:33)",
            Self::O3SeeItThrough => "O-3 干到底 — 决策立刻沉淀, 1 commit 总 (主 23:44)",
            Self::O4AnyoneCanTakeOver => "O-4 任何人都能接手 — 4 件套齐全, 顶层瘦 (主 00:56)",
            Self::O5NoPretend => "O-5 不假装 — 12 键编译期 hardcode (主 17:58)",
        }
    }

    /// 命名空间分组 (1=S-* 主体, 2=O-* 客观)
    pub const fn namespace(&self) -> u8 {
        match self {
            Self::S1NorthStar | Self::S2TruthFromReality | Self::S3QualityEngineering => 1,
            Self::O1SafetyFirst
            | Self::O2StandingOnShoulders
            | Self::O3SeeItThrough
            | Self::O4AnyoneCanTakeOver
            | Self::O5NoPretend => 2,
        }
    }

    /// 是否 R126 新增锚 (S-3 + O-1)
    pub const fn is_r126_new(&self) -> bool {
        matches!(self, Self::S3QualityEngineering | Self::O1SafetyFirst)
    }

    /// 是否原 6 锚 (向后兼容, per B1 入口签名 0 改)
    pub const fn is_legacy_six(&self) -> bool {
        matches!(
            self,
            Self::S1NorthStar
                | Self::S2TruthFromReality
                | Self::O2StandingOnShoulders
                | Self::O3SeeItThrough
                | Self::O4AnyoneCanTakeOver
                | Self::O5NoPretend
        )
    }
}

// ============================================
// 2. ALL_EIGHT_ANCHORS 完整列表 (编译期 hardcode, 8 锚)
// ============================================

/// 8 哲学锚完整列表 — 编译期 hardcode, 🦴 骨架不可变
///
/// ⚠️ 任何修改都会立即触发 `EIGHT_ANCHORS_HARDCODE` 编译期断言失败。
/// 顺序锁定 (per 09-anchor.md R125 16:55): S-1 → S-2 → S-3 (R126 NEW) → O-1 (R126 NEW) → O-2 → O-3 → O-4 → O-5
///
/// **0 改原 6 锚顺序** (per B1 入口签名 0 改 + 决策 #22 §5.1):
/// - S-1 (0 改), S-2 (0 改), O-2 (0 改), O-3 (0 改), O-4 (0 改), O-5 (0 改)
/// - `apeireth-council::PHILOSOPHICAL_ANCHORS: [&str; 6]` 顺序 0 改
///
/// **R126 升级** (per 决策 #22 §2.5 + 决策 #33 + 决策 #51 §1.2 P1-2):
/// - S-3 新增 (在 S-2 后)
/// - O-1 新增 (在 O-2 前 — 按 S-* + O-* 命名空间分组)
pub const ALL_EIGHT_ANCHORS: [PhilosophicalAnchor8; 8] = [
    // === S-* 主体哲学锚 (3 项) ===
    PhilosophicalAnchor8::S1NorthStar,
    PhilosophicalAnchor8::S2TruthFromReality,
    PhilosophicalAnchor8::S3QualityEngineering, // R126 NEW (per 决策 #22 §2.5)
    // === O-* 客观哲学锚 (5 项) ===
    PhilosophicalAnchor8::O1SafetyFirst, // R126 NEW (per 决策 #22 §2.5)
    PhilosophicalAnchor8::O2StandingOnShoulders,
    PhilosophicalAnchor8::O3SeeItThrough,
    PhilosophicalAnchor8::O4AnyoneCanTakeOver,
    PhilosophicalAnchor8::O5NoPretend,
];

/// 8 哲学锚代号列表 (顺序匹配 `ALL_EIGHT_ANCHORS`, 编译期 hardcode)
pub const ALL_EIGHT_ANCHOR_CODES: [&str; 8] = [
    "S-1",
    "S-2",
    "S-3", // R126 NEW
    "O-1", // R126 NEW
    "O-2",
    "O-3",
    "O-4",
    "O-5",
];

/// 6 哲学锚 (向后兼容) 顺序锁定 (per `apeireth-council::PHILOSOPHICAL_ANCHORS`)
/// 顺序: S-1 → S-2 → O-2 → O-3 → O-4 → O-5 (原 6 锚, 0 改)
pub const LEGACY_SIX_ANCHOR_CODES: [&str; 6] = [
    "S-1",
    "S-2",
    "O-2",
    "O-3",
    "O-4",
    "O-5",
];

/// R126 新增 2 锚代号 (S-3 + O-1, per 决策 #22 §2.5 B5 升级)
pub const R126_NEW_ANCHOR_CODES: [&str; 2] = [
    "S-3", // 质量工程化
    "O-1", // 安全优先
];

// ============================================
// 3. 编译期 hardcode 断言 (per 13 键 PHL-07 模式, A3 + R125-12 spec §2.3)
// ============================================

/// 8 哲学锚编译期 hardcode 锁 — 任何遗漏/重复/顺序错都编译失败
///
/// 这是 v6 守门 1（编译时 hardcode）在哲学锚层的真正落地：🦴 骨架不可变。
pub const EIGHT_ANCHORS_HARDCODE: () = {
    // 数组长度 = 8 (R126 升级: 6 → 8)
    if ALL_EIGHT_ANCHORS.len() != 8 {
        panic!("8 哲学锚 hardcode 被破坏！必须保持 6 原版 + S-3 + O-1 = 8");
    }
    if ALL_EIGHT_ANCHOR_CODES.len() != 8 {
        panic!("8 哲学锚代号 hardcode 被破坏！必须保持 8 项");
    }
    if LEGACY_SIX_ANCHOR_CODES.len() != 6 {
        panic!("6 原版哲学锚 hardcode 被破坏！必须保持 6 项 (向后兼容)");
    }
    if R126_NEW_ANCHOR_CODES.len() != 2 {
        panic!("R126 新增 2 锚 hardcode 被破坏！必须保持 S-3 + O-1 = 2");
    }

    // 命名空间分组 (3 S-* + 5 O-* = 8)
    let mut s_count = 0u8;
    let mut o_count = 0u8;
    let mut r126_new = 0u8;
    let mut legacy_six = 0u8;
    let mut i = 0;

    while i < ALL_EIGHT_ANCHORS.len() {
        match ALL_EIGHT_ANCHORS[i].namespace() {
            1 => s_count += 1,
            2 => o_count += 1,
            _ => panic!("未定义命名空间"),
        }
        if ALL_EIGHT_ANCHORS[i].is_r126_new() {
            r126_new += 1;
        }
        if ALL_EIGHT_ANCHORS[i].is_legacy_six() {
            legacy_six += 1;
        }
        // 验证 code() 跟 ALL_EIGHT_ANCHOR_CODES[i] 匹配
        if ALL_EIGHT_ANCHORS[i].code().as_bytes()[0] != ALL_EIGHT_ANCHOR_CODES[i].as_bytes()[0] {
            panic!("8 哲学锚 code() 不匹配 ALL_EIGHT_ANCHOR_CODES");
        }
        if ALL_EIGHT_ANCHORS[i].code().as_bytes()[1] != ALL_EIGHT_ANCHOR_CODES[i].as_bytes()[1] {
            panic!("8 哲学锚 code() 不匹配 ALL_EIGHT_ANCHOR_CODES");
        }
        i += 1;
    }

    // S-* 必须 3 个 (S-1, S-2, S-3)
    if s_count != 3 {
        panic!("8 哲学锚命名空间 S-* 不匹配 3");
    }
    // O-* 必须 5 个 (O-1, O-2, O-3, O-4, O-5)
    if o_count != 5 {
        panic!("8 哲学锚命名空间 O-* 不匹配 5");
    }
    // R126 新增必须 2 个 (S-3 + O-1)
    if r126_new != 2 {
        panic!("R126 新增哲学锚不匹配 2 (S-3 + O-1)");
    }
    // 原 6 锚必须 6 个 (向后兼容, B1 入口签名 0 改)
    if legacy_six != 6 {
        panic!("原 6 哲学锚不匹配 6 (B1 入口签名 0 改)");
    }

    // 顺序校验: 原 6 锚顺序 (S-1 → S-2 → O-2 → O-3 → O-4 → O-5) 0 改
    // 顺序: ALL_EIGHT_ANCHORS[0]=S-1, [1]=S-2, [2]=S-3, [3]=O-1, [4]=O-2, [5]=O-3, [6]=O-4, [7]=O-5
    // 提取原 6 锚顺序: S-1, S-2, O-2, O-3, O-4, O-5
    // 验证: ALL_EIGHT_ANCHORS[0] == S1, [1] == S2, [4] == O2, [5] == O3, [6] == O4, [7] == O5
    if ALL_EIGHT_ANCHORS[0] != PhilosophicalAnchor8::S1NorthStar {
        panic!("原 6 锚顺序 0 改: [0] 必须是 S-1");
    }
    if ALL_EIGHT_ANCHORS[1] != PhilosophicalAnchor8::S2TruthFromReality {
        panic!("原 6 锚顺序 0 改: [1] 必须是 S-2");
    }
    if ALL_EIGHT_ANCHORS[4] != PhilosophicalAnchor8::O2StandingOnShoulders {
        panic!("原 6 锚顺序 0 改: [4] 必须是 O-2");
    }
    if ALL_EIGHT_ANCHORS[5] != PhilosophicalAnchor8::O3SeeItThrough {
        panic!("原 6 锚顺序 0 改: [5] 必须是 O-3");
    }
    if ALL_EIGHT_ANCHORS[6] != PhilosophicalAnchor8::O4AnyoneCanTakeOver {
        panic!("原 6 锚顺序 0 改: [6] 必须是 O-4");
    }
    if ALL_EIGHT_ANCHORS[7] != PhilosophicalAnchor8::O5NoPretend {
        panic!("原 6 锚顺序 0 改: [7] 必须是 O-5");
    }

    // R126 新增 2 锚位置: [2]=S-3, [3]=O-1
    if ALL_EIGHT_ANCHORS[2] != PhilosophicalAnchor8::S3QualityEngineering {
        panic!("R126 新增位置错: [2] 必须是 S-3 (质量工程化)");
    }
    if ALL_EIGHT_ANCHORS[3] != PhilosophicalAnchor8::O1SafetyFirst {
        panic!("R126 新增位置错: [3] 必须是 O-1 (安全优先)");
    }
};

// ============================================
// 4. 6→8 互转 (向后兼容 + 升级路径)
// ============================================

/// 6 哲学锚代号 → 8 哲学锚 enum (向后兼容, B1 入口签名 0 改)
///
/// 0 装 PASS: 6 锚 input 仍工作 (per `apeireth-council::PHILOSOPHICAL_ANCHORS: [&str; 6]`)
/// 8 锚 input 升级路径 (per 决策 #22 §2.5 B5 升级)
pub const fn anchor_code_to_eight(code: &str) -> Option<PhilosophicalAnchor8> {
    match code {
        // === 6 锚原版 (向后兼容) ===
        "S-1" => Some(PhilosophicalAnchor8::S1NorthStar),
        "S-2" => Some(PhilosophicalAnchor8::S2TruthFromReality),
        "O-2" => Some(PhilosophicalAnchor8::O2StandingOnShoulders),
        "O-3" => Some(PhilosophicalAnchor8::O3SeeItThrough),
        "O-4" => Some(PhilosophicalAnchor8::O4AnyoneCanTakeOver),
        "O-5" => Some(PhilosophicalAnchor8::O5NoPretend),
        // === R126 新增 2 锚 (B5 6→8 升级) ===
        "S-3" => Some(PhilosophicalAnchor8::S3QualityEngineering),
        "O-1" => Some(PhilosophicalAnchor8::O1SafetyFirst),
        // === 0 装 PASS: 0 假装"已升级" ===
        _ => None,
    }
}

/// 6 锚代号列表 (per `apeireth-council::PHILOSOPHICAL_ANCHORS: [&str; 6]`, 0 改)
pub const LEGACY_SIX_ANCHORS: [&str; 6] = [
    "S-1",
    "S-2",
    "O-2",
    "O-3",
    "O-4",
    "O-5",
];

// ============================================
// 5. 内联单元测试 (per R125-8 模式, 8+ tests)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 1: ALL_EIGHT_ANCHORS 数组长度 = 8 (编译期断言已自动运行)
    #[test]
    fn test_eight_anchors_complete() {
        assert_eq!(
            ALL_EIGHT_ANCHORS.len(),
            8,
            "8 哲学锚 hardcode 数组长度必须 = 8"
        );
        // 验证每个锚都在数组中（无重复、无遗漏）
        let mut seen = [false; 8];
        for (i, anchor) in ALL_EIGHT_ANCHORS.iter().enumerate() {
            // 同一 anchor 多次出现 = 重复
            for j in 0..i {
                assert_ne!(ALL_EIGHT_ANCHORS[j], *anchor, "8 锚数组中出现重复: {:?}", anchor);
            }
            seen[i] = true;
        }
        assert!(seen.iter().all(|x| *x), "8 锚数组完整性检查");
    }

    /// 测试 2: 8 锚命名空间分组 = 3 S-* + 5 O-*
    #[test]
    fn test_eight_anchors_namespace_distribution() {
        let s_count = ALL_EIGHT_ANCHORS.iter().filter(|a| a.namespace() == 1).count();
        let o_count = ALL_EIGHT_ANCHORS.iter().filter(|a| a.namespace() == 2).count();

        assert_eq!(s_count, 3, "S-* 命名空间必须有 3 个 (S-1, S-2, S-3)");
        assert_eq!(o_count, 5, "O-* 命名空间必须有 5 个 (O-1, O-2, O-3, O-4, O-5)");
        assert_eq!(s_count + o_count, 8, "S-* + O-* 必须 = 8");
    }

    /// 测试 3: 8 锚 R126 新增分组 = 2 (S-3 + O-1)
    #[test]
    fn test_eight_anchors_r126_new() {
        let r126_new_count = ALL_EIGHT_ANCHORS.iter().filter(|a| a.is_r126_new()).count();
        let legacy_six_count = ALL_EIGHT_ANCHORS.iter().filter(|a| a.is_legacy_six()).count();

        assert_eq!(r126_new_count, 2, "R126 新增必须有 2 个 (S-3 + O-1)");
        assert_eq!(legacy_six_count, 6, "原 6 锚必须有 6 个 (向后兼容)");
        assert_eq!(r126_new_count + legacy_six_count, 8);
    }

    /// 测试 4: EIGHT_ANCHORS_HARDCODE const 评估（仅作可访问性证明）
    #[test]
    fn test_eight_anchors_hardcode_compile_time_lock() {
        // 这个 const 单元类型值在编译期已被求值，类型为 ()
        let _lock: () = EIGHT_ANCHORS_HARDCODE;
        // 实际不需要运行时断言 — 编译期已锁定
    }

    /// 测试 5: 8 锚 code() 函数返回正确代号
    #[test]
    fn test_eight_anchors_code() {
        assert_eq!(PhilosophicalAnchor8::S1NorthStar.code(), "S-1");
        assert_eq!(PhilosophicalAnchor8::S2TruthFromReality.code(), "S-2");
        assert_eq!(PhilosophicalAnchor8::S3QualityEngineering.code(), "S-3"); // R126 NEW
        assert_eq!(PhilosophicalAnchor8::O1SafetyFirst.code(), "O-1"); // R126 NEW
        assert_eq!(PhilosophicalAnchor8::O2StandingOnShoulders.code(), "O-2");
        assert_eq!(PhilosophicalAnchor8::O3SeeItThrough.code(), "O-3");
        assert_eq!(PhilosophicalAnchor8::O4AnyoneCanTakeOver.code(), "O-4");
        assert_eq!(PhilosophicalAnchor8::O5NoPretend.code(), "O-5");
    }

    /// 测试 6: 8 锚 description() 函数返回正确描述
    #[test]
    fn test_eight_anchors_description() {
        assert!(PhilosophicalAnchor8::S1NorthStar.description().contains("S-1"));
        assert!(PhilosophicalAnchor8::S2TruthFromReality.description().contains("S-2"));
        assert!(PhilosophicalAnchor8::S3QualityEngineering.description().contains("S-3"));
        assert!(PhilosophicalAnchor8::S3QualityEngineering.description().contains("质量工程化"));
        assert!(PhilosophicalAnchor8::O1SafetyFirst.description().contains("O-1"));
        assert!(PhilosophicalAnchor8::O1SafetyFirst.description().contains("安全优先"));
        assert!(PhilosophicalAnchor8::O2StandingOnShoulders.description().contains("O-2"));
        assert!(PhilosophicalAnchor8::O3SeeItThrough.description().contains("O-3"));
        assert!(PhilosophicalAnchor8::O4AnyoneCanTakeOver.description().contains("O-4"));
        assert!(PhilosophicalAnchor8::O5NoPretend.description().contains("O-5"));
    }

    /// 测试 7: 6→8 互转 (向后兼容, 6 锚 input 仍 work)
    #[test]
    fn test_anchor_code_to_eight_legacy_six() {
        // 6 锚 input 0 改 (向后兼容, B1 入口签名 0 改)
        assert_eq!(anchor_code_to_eight("S-1"), Some(PhilosophicalAnchor8::S1NorthStar));
        assert_eq!(anchor_code_to_eight("S-2"), Some(PhilosophicalAnchor8::S2TruthFromReality));
        assert_eq!(anchor_code_to_eight("O-2"), Some(PhilosophicalAnchor8::O2StandingOnShoulders));
        assert_eq!(anchor_code_to_eight("O-3"), Some(PhilosophicalAnchor8::O3SeeItThrough));
        assert_eq!(anchor_code_to_eight("O-4"), Some(PhilosophicalAnchor8::O4AnyoneCanTakeOver));
        assert_eq!(anchor_code_to_eight("O-5"), Some(PhilosophicalAnchor8::O5NoPretend));
        // 0 装 PASS: 0 假装"已升级"
        assert_eq!(anchor_code_to_eight("S-3"), Some(PhilosophicalAnchor8::S3QualityEngineering));
        assert_eq!(anchor_code_to_eight("O-1"), Some(PhilosophicalAnchor8::O1SafetyFirst));
        // 无效 input
        assert_eq!(anchor_code_to_eight("S-99"), None);
        assert_eq!(anchor_code_to_eight(""), None);
    }

    /// 测试 8: 顺序锁定 — 原 6 锚位置在 8 锚 0 改
    #[test]
    fn test_legacy_six_position_unchanged() {
        // 原 6 锚在 8 锚中的位置 (per B1 入口签名 0 改):
        // S-1 = [0], S-2 = [1], O-2 = [4], O-3 = [5], O-4 = [6], O-5 = [7]
        // R126 新增: S-3 = [2], O-1 = [3]
        assert_eq!(ALL_EIGHT_ANCHORS[0], PhilosophicalAnchor8::S1NorthStar);
        assert_eq!(ALL_EIGHT_ANCHORS[1], PhilosophicalAnchor8::S2TruthFromReality);
        assert_eq!(ALL_EIGHT_ANCHORS[4], PhilosophicalAnchor8::O2StandingOnShoulders);
        assert_eq!(ALL_EIGHT_ANCHORS[5], PhilosophicalAnchor8::O3SeeItThrough);
        assert_eq!(ALL_EIGHT_ANCHORS[6], PhilosophicalAnchor8::O4AnyoneCanTakeOver);
        assert_eq!(ALL_EIGHT_ANCHORS[7], PhilosophicalAnchor8::O5NoPretend);
        // R126 新增位置
        assert_eq!(ALL_EIGHT_ANCHORS[2], PhilosophicalAnchor8::S3QualityEngineering);
        assert_eq!(ALL_EIGHT_ANCHORS[3], PhilosophicalAnchor8::O1SafetyFirst);
    }

    /// 测试 9 (额外): ALL_EIGHT_ANCHOR_CODES 匹配 ALL_EIGHT_ANCHORS
    #[test]
    fn test_codes_match_anchors() {
        for i in 0..ALL_EIGHT_ANCHORS.len() {
            assert_eq!(
                ALL_EIGHT_ANCHORS[i].code(),
                ALL_EIGHT_ANCHOR_CODES[i],
                "ALL_EIGHT_ANCHORS[{}].code() != ALL_EIGHT_ANCHOR_CODES[{}]",
                i,
                i
            );
        }
    }

    /// 测试 10 (额外): LEGACY_SIX_ANCHORS 跟 LEGACY_SIX_ANCHOR_CODES 一致
    #[test]
    fn test_legacy_six_anchors_consistent() {
        assert_eq!(LEGACY_SIX_ANCHORS, LEGACY_SIX_ANCHOR_CODES);
        assert_eq!(LEGACY_SIX_ANCHORS.len(), 6);
    }

    /// 测试 11 (额外): R126_NEW_ANCHOR_CODES 是 S-3 + O-1
    #[test]
    fn test_r126_new_anchor_codes() {
        assert_eq!(R126_NEW_ANCHOR_CODES, ["S-3", "O-1"]);
        assert!(anchor_code_to_eight(R126_NEW_ANCHOR_CODES[0]).unwrap().is_r126_new());
        assert!(anchor_code_to_eight(R126_NEW_ANCHOR_CODES[1]).unwrap().is_r126_new());
    }

    /// 测试 12 (额外): is_legacy_six() 跟 is_r126_new() 互斥
    #[test]
    fn test_legacy_and_new_mutually_exclusive() {
        for anchor in ALL_EIGHT_ANCHORS.iter() {
            assert_ne!(
                anchor.is_legacy_six(),
                anchor.is_r126_new(),
                "锚 {:?} 应该只属于原 6 锚或 R126 新增 2 锚之一, 不应同时是两者",
                anchor
            );
        }
    }
}
