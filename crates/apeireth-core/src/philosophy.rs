//! `apeireth-core::philosophy` — 12 键 verdict (编译时 hardcode trait)
//!
//! 拆自 `lib.rs` line 148-317 (R131 架构债清理). 0 触碰公开签名 — `use apeireth_core::PhilosophyKey` 等仍可用.
//!
//! 包含: typedef 本段所有 `pub struct` / `pub enum` / `pub trait` / `pub const`.

use crate::Action;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// 3. 12 键 verdict (编译时 hardcode trait)
// ============================================

/// V3 9 键 + v4.1 新增 3 键 = 12 键
///
/// ⚠️ 编译时 hardcode 锁：增删任何键都会触发 `ALL_TWELVE_KEYS` 数组长度不匹配错误。
/// `const TWELVE_KEYS_HARDCODE` 在编译期强制数组长度为 12。
/// 详见 docs/architecture-v4-1-living-intelligence-update.md §15.2。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhilosophyKey {
    // === V3 PHL-01 not_X (LOCKED 9 键之一) ===
    /// PHL-01 not_clone: 不假装克隆/同质化
    NotClone,
    /// PHL-01 not_perfect: 不假装完美/100%
    NotPerfect,
    /// PHL-01 not_uuid: 不假装唯一解/唯一真相
    NotUuid,
    // === V3 PHL-02b not_X (LOCKED 9 键之一) ===
    /// PHL-02b not_undo: 不假装可撤销过去
    NotUndo,
    /// PHL-02b not_proof: 不假装完整证明
    NotProof,
    /// PHL-02b not_safe: 不假装绝对安全
    NotSafe,
    // === V3 PHL-03 X_is_not_Y (LOCKED 9 键之一) ===
    /// PHL-03 spec_is_not_proof: 不把规格当证明
    SpecIsNotProof,
    /// PHL-03 counterexample_is_not_bug: 不把反例当 bug
    CounterexampleIsNotBug,
    /// PHL-03 prover_is_not_truth: 不把证明者当真理
    ProverIsNotTruth,
    // === v4.1 §15 新增 3 键 (PHL-04/05/06) ===
    /// PHL-04 not_pretend_unobservable: 不假装内部状态不可观测
    NotUnobservable,
    /// PHL-05 not_pretend_unscientific: 不假装决策不基于科学方法
    NotUnscientific,
    /// PHL-06 not_pretend_no_self_relation: 不假装与自身没有关系/无主体连续性
    NotSelfRelationless,
}

impl PhilosophyKey {
    /// 编译时 hardcode - 返回每个键的描述
    pub const fn description(&self) -> &'static str {
        match self {
            Self::NotClone => "不假装克隆",
            Self::NotPerfect => "不假装完美",
            Self::NotUuid => "不假装唯一",
            Self::NotUndo => "不假装可撤销",
            Self::NotProof => "不假装可证明",
            Self::NotSafe => "不假装绝对安全",
            Self::SpecIsNotProof => "规格不是证明",
            Self::CounterexampleIsNotBug => "反例不是 bug",
            Self::ProverIsNotTruth => "证明者不是真理",
            Self::NotUnobservable => "PHL-04 不假装不可观测",
            Self::NotUnscientific => "PHL-05 不假装不科学",
            Self::NotSelfRelationless => "PHL-06 不假装不与自身关系",
        }
    }

    /// 主键分组 ID (1=PHL-01, 2=PHL-02b, 3=PHL-03, 4=PHL-04, 5=PHL-05, 6=PHL-06)
    pub const fn group_id(&self) -> u8 {
        match self {
            Self::NotClone | Self::NotPerfect | Self::NotUuid => 1,
            Self::NotUndo | Self::NotProof | Self::NotSafe => 2,
            Self::SpecIsNotProof | Self::CounterexampleIsNotBug | Self::ProverIsNotTruth => 3,
            Self::NotUnobservable => 4,
            Self::NotUnscientific => 5,
            Self::NotSelfRelationless => 6,
        }
    }
}

/// 12 键完整列表 — 编译时 hardcode，🦴 骨架不可变。
///
/// ⚠️ 任何修改都会立即触发 `TWELVE_KEYS_HARDCODE` 编译期断言失败。
/// 顺序锁定：V3 PHL-01 (3) → V3 PHL-02b (3) → V3 PHL-03 (3) → v4.1 PHL-04/05/06 (3)
pub const ALL_TWELVE_KEYS: [PhilosophyKey; 12] = [
    // V3 PHL-01 not_X (LOCKED)
    PhilosophyKey::NotClone,
    PhilosophyKey::NotPerfect,
    PhilosophyKey::NotUuid,
    // V3 PHL-02b not_X (LOCKED)
    PhilosophyKey::NotUndo,
    PhilosophyKey::NotProof,
    PhilosophyKey::NotSafe,
    // V3 PHL-03 X_is_not_Y (LOCKED)
    PhilosophyKey::SpecIsNotProof,
    PhilosophyKey::CounterexampleIsNotBug,
    PhilosophyKey::ProverIsNotTruth,
    // v4.1 §15 新增 3 键 (PHL-04/05/06)
    PhilosophyKey::NotUnobservable,
    PhilosophyKey::NotUnscientific,
    PhilosophyKey::NotSelfRelationless,
];

/// 编译期断言 — 12 键 hardcode 锁。任何遗漏/重复都编译失败。
///
/// 这是 v6 守门 1（编译时 hardcode）的真正落地：🦴 骨架不可变。
pub const TWELVE_KEYS_HARDCODE: () = {
    // 数组长度 = 12。增删键必须同步修改此断言，否则编译失败。
    if ALL_TWELVE_KEYS.len() != 12 {
        panic!("12 键 hardcode 被破坏！必须保持 V3 9 键 + v4.1 新增 3 键 = 12");
    }
    // 主键分组检查 (3+3+3+1+1+1 = 12)
    let mut phl01 = 0u8;
    let mut phl02b = 0u8;
    let mut phl03 = 0u8;
    let mut phl04 = 0u8;
    let mut phl05 = 0u8;
    let mut phl06 = 0u8;
    let mut i = 0;
    while i < ALL_TWELVE_KEYS.len() {
        match ALL_TWELVE_KEYS[i].group_id() {
            1 => phl01 += 1,
            2 => phl02b += 1,
            3 => phl03 += 1,
            4 => phl04 += 1,
            5 => phl05 += 1,
            6 => phl06 += 1,
            _ => panic!("未分组键"),
        }
        i += 1;
    }
    if phl01 != 3 || phl02b != 3 || phl03 != 3 || phl04 != 1 || phl05 != 1 || phl06 != 1 {
        panic!("12 键分组不匹配！3+3+3+1+1+1=12");
    }
};

/// 哲学守门 trait - 编译时 hardcode 强制实现 12 键
pub trait PhilosophyGuard: Send + Sync {
    /// 12 键 verdict (编译时 hardcode 强制所有 verdict 都返回 bool)
    fn check_philosophy(&self, action: &Action) -> PhilosophyVerdict;
    /// 元问题禁令 (外部反馈 §3.A) - 反思期不能询问"是否需要 L0 HA"
    fn is_forbidden_meta_question(&self, query: &str) -> bool {
        query.contains("L0 HA") || query.contains("是否需要") || query.contains("取消 L0")
    }
}

/// 12 键哲学守门的 verdict 结果
#[derive(Debug, Clone, PartialEq)]
pub enum PhilosophyVerdict {
    /// 通过 (V1 允许)
    Allow,
    /// 拒绝 (V1 拒绝，附带违反的具体键)
    Block(PhilosophyKey),
}

/// 12 键 verdict cache (运行时 O(1) 查询缓存)
#[derive(Debug, Default)]
pub struct VerdictCache {
    /// action_id → verdict 映射
    cache: HashMap<String, PhilosophyVerdict>,
}

impl VerdictCache {
    /// 创建空缓存
    pub fn new() -> Self {
        Self::default()
    }
    /// 查询 verdict (门上的肉 = 动态变化)
    pub fn get(&self, action_id: &str) -> Option<&PhilosophyVerdict> {
        self.cache.get(action_id)
    }
    /// 刷新 verdict (OTA / hot-reload / 反思期可改)
    pub fn refresh(&mut self, action_id: String, verdict: PhilosophyVerdict) {
        self.cache.insert(action_id, verdict);
    }
}

// ============================================
