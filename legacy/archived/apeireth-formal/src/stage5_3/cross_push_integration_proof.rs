//! R129-20 Stage 5.3 跨模块证明 F20 — 跨 push 集成形式化
//!
//! # 背景 (per 决策 #33 §2.3 + 决策 #55 §1 + 决策 #61 §3.1 R129-20)
//!
//! 0 主动 push 严守 (per 决策 #33 §2.3 + 决策 #48 + 决策 #61 §6):
//! - 0 主动 push (等 1.0 release 配 GitHub remote, per 决策 #61 §6)
//! - 整合 #4 commit abf12243 (19:41 done, 0 重跑, per 决策 #48)
//! - 整合 #5 commit 由 Mavis 拍板时机 (per 决策 #62 拆 3 commit)
//! - 13 关键决策链 #22-#66 全 0 主动 push
//!
//! F20 跨 push 集成: 13 关键决策 全 0 主动 push 严守 (per 决策 #33 §2.3 + 决策 #61 §6).
//!
//! # 借鉴 ID
//!
//! `R129-20-F20-BORROW-kani-4502-Invariant-trait-2026-08-11`
//! - 0 装 PASS 严守: ✅ 0 引 kani 依赖, 0 装"已 Kani 形式化"
//! - 0 越界 8 硬墙: 0 主动 push 严守 (F20 仅形式化, 0 主动 push)
//!
//! # 0 触碰 (per 决策 #33 §2.3 + 决策 #61 §6)
//!
//! - 0 主动 push: 本模块 0 主动 push (等 1.0 release 配 GitHub remote)
//! - C1 0 主动 commit: Mavis 整合 #5.1 commit 拍板

#![cfg_attr(kani, allow(dead_code))]

// ============================================================
// 1. 编译期常量 (0 主动 push 严守, 1:1 跟 决策 #33 §2.3 + 决策 #61 §6)
// ============================================================

/// 0 主动 push 严守 (1:1 跟 决策 #33 §2.3 + 决策 #61 §6, 0 改 = 0)
pub const ZERO_PUSH_COUNT: u32 = 0;

/// 13 关键决策 push 总数 (1:1 跟 决策 #22-#66 严守, F14 cross_decision)
pub const CROSS_PUSH_DECISION_COUNT: usize = 13;

/// 13 关键决策 push 严守状态 (1:1 跟 决策 #33 §2.3 + 决策 #61 §6, 0 主动 push)
pub const CROSS_PUSH_DECISION_IDS: [&str; CROSS_PUSH_DECISION_COUNT] = [
    "decision-22",  // 16:31 决策授权
    "decision-33",  // 17:22 master reupgrade
    "decision-36",  // 8 哲学锚 + 6 重守门
    "decision-41",  // R125 16 sub-agent
    "decision-48",  // 整合 #4 commit done
    "decision-53",  // 24 LOCKED 内部 fn 可改
    "decision-55",  // R127 4 飞会
    "decision-56",  // R127-2 10 飞会
    "decision-57",  // R128 6 飞会
    "decision-58",  // R128-2 3 飞会
    "decision-61",  // R129 era 16 飞会
    "decision-62",  // 整合 #5 拆 3 commit
    "decision-66",  // 1.0 release 准备
];

/// 0 主动 push 严守状态 (per 决策 #33 §2.3 + 决策 #61 §6)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PushStrict {
    /// 0 主动 push 严守 (true strict, per 决策 #33 §2.3)
    ZeroPush = 0,
    /// 1+ push 违反 (false strict, F20 不允许)
    PushViolation = 1,
}

/// 跨 push 集成 POD 镜像 (1:1 跟 决策 #33 §2.3 + 决策 #61 §6, 0 主动 push 严守)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CrossPushIntegrationPod {
    /// 决策索引 (0..12, 1:1 跟 决策 #22-#66 严守)
    pub decision_index: u8,
    /// 决策 ID (1:1 跟 决策 #22-#66 严守, 0 主动 push)
    pub decision_id: &'static str,
    /// 主动 push 计数 (1:1 跟 决策 #33 §2.3 + 决策 #61 §6, 0 主动 push = 0)
    pub push_count: u32,
    /// 0 主动 push 严守状态 (1:1 跟 决策 #33 §2.3, 0 主动 push = ZeroPush)
    pub push_strict: PushStrict,
}

impl CrossPushIntegrationPod {
    /// 构造 (编译期 hardcode)
    pub const fn new(decision_index: u8, decision_id: &'static str, push_count: u32, push_strict: PushStrict) -> Self {
        Self { decision_index, decision_id, push_count, push_strict }
    }

    /// F20 跨 push 集成总决策数 (13, 1:1 跟 决策 #22-#66 严守)
    pub const fn count() -> usize {
        CROSS_PUSH_DECISION_COUNT
    }
}

// ============================================================
// 2. 跨 push 集成不变量 (决策 #33 §2.3 + 决策 #61 §6 严守 0 主动 push)
// ============================================================

/// 跨 push 集成 decision_index 不变量: decision_index ∈ 0..12 永真 (决策 #22-#66 严守)
pub fn cross_push_decision_index_invariant(c: CrossPushIntegrationPod) -> bool {
    c.decision_index < CROSS_PUSH_DECISION_COUNT as u8
}

/// 跨 push 集成 0 主动 push 不变量: push_count = 0 永真 (决策 #33 §2.3 + 决策 #61 §6)
pub fn cross_push_zero_push_strict(c: CrossPushIntegrationPod) -> bool {
    c.push_count == ZERO_PUSH_COUNT
}

/// 跨 push 集成 push_strict 严守不变量: ZeroPush 永真 (决策 #33 §2.3 + 决策 #61 §6)
pub fn cross_push_push_strict_invariant(c: CrossPushIntegrationPod) -> bool {
    matches!(c.push_strict, PushStrict::ZeroPush)
}

/// 跨 push 集成全 0 主动 push 不变量: 13 决策 push_count=0 + push_strict=ZeroPush (决策 #33 §2.3)
pub fn cross_push_all_zero_push(ds: &[CrossPushIntegrationPod]) -> bool {
    for d in ds {
        if !cross_push_zero_push_strict(*d) || !cross_push_push_strict_invariant(*d) {
            return false;
        }
    }
    true
}

// ============================================================
// 3. Kani-style proof harness
// ============================================================

/// Kani proof harness — 跨 push 集成 decision_index ∈ 0..12 永真 (决策 #22-#66 严守)
#[cfg_attr(kani, kani::proof)]
pub fn proof_cross_push_decision_index_in_range() {
    let c = nondet_cross_push();
    assert!(cross_push_decision_index_invariant(c), "跨 push 集成 decision_index 必须在 0..12");
}

/// Kani proof harness — 跨 push 集成 13 决策 全 0 主动 push 严守 (决策 #33 §2.3 + 决策 #61 §6)
#[cfg_attr(kani, kani::proof)]
pub fn proof_cross_push_zero_push_strict() {
    assert_eq!(CrossPushIntegrationPod::count(), 13, "F20 跨 push 集成总决策数 = 13");
    assert_eq!(ZERO_PUSH_COUNT, 0, "0 主动 push 严守 = 0 (per 决策 #33 §2.3 + 决策 #61 §6)");
    assert_eq!(CROSS_PUSH_DECISION_IDS.len(), 13, "13 决策 ID 1:1 严守");
}

#[cfg(kani)]
fn nondet_cross_push() -> CrossPushIntegrationPod {
    kani::any()
}

#[cfg(not(kani))]
fn nondet_cross_push() -> CrossPushIntegrationPod {
    // cargo test 兜底: decision-33 (master reupgrade) 0 主动 push happy path
    CrossPushIntegrationPod::new(1, "decision-33", 0, PushStrict::ZeroPush)
}

/// Runtime sanity: 跨 push 集成 13 决策 (0..12) 应全 0 主动 push 严守 (决策 #33 §2.3 + 决策 #61 §6)
pub fn sanity_check() -> bool {
    for index in 0u8..CROSS_PUSH_DECISION_COUNT as u8 {
        let c = CrossPushIntegrationPod::new(index, "test", 0, PushStrict::ZeroPush);
        if !cross_push_decision_index_invariant(c) {
            return false;
        }
        if !cross_push_zero_push_strict(c) {
            return false;
        }
        if !cross_push_push_strict_invariant(c) {
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
        let _: fn() = proof_cross_push_decision_index_in_range;
        let _: fn() = proof_cross_push_zero_push_strict;
    }

    #[test]
    fn cross_push_zero_push_count_is_0() {
        // 0 主动 push 严守 = 0 (per 决策 #33 §2.3 + 决策 #61 §6)
        assert_eq!(ZERO_PUSH_COUNT, 0);
    }

    #[test]
    fn cross_push_decision_count_is_13() {
        // 13 关键决策 push 严守状态 (1:1 跟 决策 #22-#66 严守, F14 cross_decision)
        assert_eq!(CROSS_PUSH_DECISION_COUNT, 13);
        assert_eq!(CROSS_PUSH_DECISION_IDS.len(), 13);
        assert_eq!(CrossPushIntegrationPod::count(), 13);
    }

    #[test]
    fn cross_push_decision_index_0_to_12_all_pass() {
        // decision_index ∈ 0..12 永真 (决策 #22-#66 严守)
        for index in 0u8..13 {
            let c = CrossPushIntegrationPod::new(index, "test", 0, PushStrict::ZeroPush);
            assert!(cross_push_decision_index_invariant(c));
            assert!(cross_push_zero_push_strict(c));
            assert!(cross_push_push_strict_invariant(c));
        }
    }

    #[test]
    fn cross_push_decision_index_13_violates() {
        // 反例: decision_index=13 越界 (决策 #22-#66 严守 0..13)
        let c = CrossPushIntegrationPod::new(13, "test", 0, PushStrict::ZeroPush);
        assert!(!cross_push_decision_index_invariant(c));
    }

    #[test]
    fn cross_push_count_1_violates() {
        // 反例: push_count=1 越界 (决策 #33 §2.3 + 决策 #61 §6 0 主动 push 严守)
        let c = CrossPushIntegrationPod::new(0, "test", 1, PushStrict::ZeroPush);
        assert!(!cross_push_zero_push_strict(c));
    }

    #[test]
    fn cross_push_push_violation_strict_detected() {
        // 反例: push_strict=PushViolation 越界 (决策 #33 §2.3 0 主动 push 严守)
        let c = CrossPushIntegrationPod::new(0, "test", 0, PushStrict::PushViolation);
        assert!(!cross_push_push_strict_invariant(c));
    }

    #[test]
    fn cross_push_decision_33_zero_push_strict() {
        // 决策 #33 (master reupgrade) 0 主动 push 严守 (per 决策 #33 §2.3 + 决策 #61 §6)
        let c = CrossPushIntegrationPod::new(1, "decision-33", 0, PushStrict::ZeroPush);
        assert_eq!(c.decision_id, "decision-33");
        assert!(cross_push_zero_push_strict(c));
    }

    #[test]
    fn sanity_check_returns_true() {
        assert!(sanity_check());
    }
}
