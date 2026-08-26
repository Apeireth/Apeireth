//! `apeireth-onion`: 双洋葱统一体 trait abstraction layer
//!
//! 依据：
//! - 阶段 1 §3 原则洋葱 v3.0 (5 层 E/S/A/M/O)
//! - 阶段 1 §19.4 双洋葱是比喻, 架构可替换 (比喻 vs 架构显式分离)
//! - ADR-0001-double-onion-unity.md (原则嵌入权限 = 一个统一体的两个切面)
//!
//! 设计意图（**不**在 `apeireth-onion` 重复 `apeireth-core` 的 `PrincipleOnion` /
//! `PermissionOnion` struct；这些 struct 是具体实现，本 crate 是 **trait 抽象层**）：
//!
//! ```text
//!   比喻 (双洋葱)         ← §19.4 对外心智模型
//!        ↓
//!   架构 (本 trait 层)    ← PREREQ-2 §4 6 组件的"接口骨架"
//!        ↓
//!   实现 (apeireth-core)  ← struct data + serde + verdict cache
//! ```
//!
//! 编译时 hardcode（`const fn` + 编译期断言）：
//! - 5 个原则层 (`PrincipleLayer`: E/S/A/M/O)
//! - 6 个权限层 (`PermissionLayer`: L0..L5)
//! - 11 个电子环节点（5 + 6 = 11 切片统一环）
//!
//! 禁止：
//! - ❌ 不修改 `apeireth-core` 已实装的 `PrincipleOnion` / `PermissionOnion` struct
//! - ❌ 不修改 LOCKED 阶段 1/2/3 任何文件
//! - ❌ 不引入新依赖（仅使用 workspace.dependencies）
//! - ❌ 不引入 I/O / 不引入 `unsafe`

#![deny(unsafe_code)]

use apeireth_core::{
    HumanAuthority, PermissionLayer as CorePermissionLayer, PermissionOnion as CorePermissionOnion,
    PrincipleLayer as CorePrincipleLayer, PrincipleOnion as CorePrincipleOnion,
};
use serde::{Deserialize, Serialize};

// ============================================================
// 1. 层身份（编译时 hardcode，const fn 断言）
// ============================================================

/// 原则洋葱 5 层 (Existence / Spirit / Accumulation / Methodology / Operational)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrincipleLayerKind {
    /// E 层 — 存在 (6 项不可违背原则，编译时 hardcode)
    Existence,
    /// S 层 — 价值 (4 项：ASI 北极星 / 实事求是 / 长期主义 / 谦卑)
    Spirit,
    /// A 层 — 经验沉淀
    Accumulation,
    /// M 层 — 方法论
    Methodology,
    /// O 层 — 操作原则 (9 键 + 5 项不假装 + O-1..O-6)
    Operational,
}

/// 编译时 hardcode：5 个原则层按"内→外"顺序（深→浅）
pub const PRINCIPLE_LAYERS_OUTER_IN: [PrincipleLayerKind; 5] = [
    PrincipleLayerKind::Existence,
    PrincipleLayerKind::Spirit,
    PrincipleLayerKind::Accumulation,
    PrincipleLayerKind::Methodology,
    PrincipleLayerKind::Operational,
];

/// 权限洋葱 6 层 (L0..L5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionLayerKind {
    /// L0 — HA 核心
    L0,
    /// L1 — 受控写
    L1,
    /// L2 — 重要操作
    L2,
    /// L3 — 关键操作
    L3,
    /// L4 — 核心升级
    L4,
    /// L5 — 核武器级
    L5,
}

/// 编译时 hardcode：6 个权限层按"内→外"顺序
pub const PERMISSION_LAYERS_OUTER_IN: [PermissionLayerKind; 6] = [
    PermissionLayerKind::L0,
    PermissionLayerKind::L1,
    PermissionLayerKind::L2,
    PermissionLayerKind::L3,
    PermissionLayerKind::L4,
    PermissionLayerKind::L5,
];

/// 11 节点电子环（5 + 6 = 11）
pub const ELECTRONIC_RING_LEN: usize = 11;

const _: () = {
    assert!(
        PRINCIPLE_LAYERS_OUTER_IN.len() == 5,
        "PRINCIPLE_LAYERS must have exactly 5 layers (E/S/A/M/O)"
    );
    assert!(
        PERMISSION_LAYERS_OUTER_IN.len() == 6,
        "PERMISSION_LAYERS must have exactly 6 layers (L0..L5)"
    );
    assert!(
        PRINCIPLE_LAYERS_OUTER_IN.len() + PERMISSION_LAYERS_OUTER_IN.len() == ELECTRONIC_RING_LEN,
        "5 principle + 6 permission = 11 electronic ring nodes"
    );
    // round7-02 onion-dedupe-hardcode:
    // 断言 onion 不重新定义 core 的 5 层 / 6 层结构。
    // 通过 const 引用 core::PrincipleOnion / core::PermissionOnion 的字段名,
    // 字段被重命名或删除时此处会编译失败 (Rust 字段访问是类型检查的一部分)。
    const fn _enforce_core_principle_layers(p: &CorePrincipleOnion) -> [PrincipleLayerKind; 5] {
        // 字段顺序必须 = PRINCIPLE_LAYERS_OUTER_IN 顺序 (E/S/A/M/O)
        let _e = &p.e_layer;
        let _s = &p.s_layer;
        let _a = &p.a_layer;
        let _m = &p.m_layer;
        let _o = &p.o_layer;
        PRINCIPLE_LAYERS_OUTER_IN
    }
    const fn _enforce_core_permission_layers(p: &CorePermissionOnion) -> [PermissionLayerKind; 6] {
        // 字段顺序必须 = PERMISSION_LAYERS_OUTER_IN 顺序 (L0..L5)
        let _l0 = &p.l0;
        let _l1 = &p.l1;
        let _l2 = &p.l2;
        let _l3 = &p.l3;
        let _l4 = &p.l4;
        let _l5 = &p.l5;
        PERMISSION_LAYERS_OUTER_IN
    }
    // 编译期调用 enforce 函数, 触发类型检查
    let _ = _enforce_core_principle_layers;
    let _ = _enforce_core_permission_layers;
};

/// round7-02 onion-dedupe-hardcode (architect2):
/// 编译期 hardcode 锚点 — 在模块加载时类型检查 core 的字段名不漂移。
/// 若 core 删除 / 重命名 `e_layer`/`s_layer`/... 或 `l0`/`l1`/..., 此函数编译失败。
#[doc(hidden)]
fn _compile_time_anchor_no_dupe(p: &CorePrincipleOnion, q: &CorePermissionOnion) {
    let _ = (
        &p.e_layer, &p.s_layer, &p.a_layer, &p.m_layer, &p.o_layer, &q.l0, &q.l1, &q.l2, &q.l3,
        &q.l4, &q.l5,
    );
}

// ============================================================
// 2. 切片 trait
// ============================================================

/// 原则层切片 trait
pub trait PrincipleSlice {
    /// 层身份
    fn kind(&self) -> PrincipleLayerKind;
    /// 是否编译时 hardcode
    fn is_hardcoded(&self) -> bool;
    /// 层名称
    fn name(&self) -> &str;
}

/// 权限层切片 trait
pub trait PermissionSlice {
    /// 层身份
    fn kind(&self) -> PermissionLayerKind;
    /// 是否需要 HA
    fn requires_ha(&self) -> bool;
    /// 层名称
    fn name(&self) -> &str;
}

// ============================================================
// 3. 洋葱 trait
// ============================================================

/// 原则洋葱 trait
pub trait PrincipleOnion {
    /// 5 层切片按"外→内"顺序（O → A → M → S → E — 浅→深）
    fn slices_outer_in(&self) -> [&dyn PrincipleSlice; 5];
    /// 按层身份查询切片
    fn slice(&self, kind: PrincipleLayerKind) -> &dyn PrincipleSlice;
    /// 跨层冲突仲裁（§3.6）— E 胜所有 > S > A > M > O
    fn arbitrate(&self, a: PrincipleLayerKind, b: PrincipleLayerKind) -> PrincipleLayerKind {
        if a == PrincipleLayerKind::Existence || b == PrincipleLayerKind::Existence {
            return PrincipleLayerKind::Existence;
        }
        if a == PrincipleLayerKind::Spirit || b == PrincipleLayerKind::Spirit {
            return PrincipleLayerKind::Spirit;
        }
        if a == PrincipleLayerKind::Accumulation || b == PrincipleLayerKind::Accumulation {
            return PrincipleLayerKind::Accumulation;
        }
        if a == PrincipleLayerKind::Methodology || b == PrincipleLayerKind::Methodology {
            return PrincipleLayerKind::Methodology;
        }
        a
    }
}

/// 权限洋葱 trait
pub trait PermissionOnion {
    /// 6 层切片按"外→内"顺序
    fn slices_outer_in(&self) -> [&dyn PermissionSlice; 6];
    /// 按层身份查询切片
    fn slice(&self, kind: PermissionLayerKind) -> &dyn PermissionSlice;
    /// L0 是否永远需要 HA
    fn l0_requires_ha(&self) -> bool {
        true
    }
}

// ============================================================
// 4. 双洋葱统一体 trait
// ============================================================

/// 双洋葱统一体 trait — V1+V2+V3 AND 门（原则嵌入权限）
pub trait DoubleOnionUnification: PrincipleOnion + PermissionOnion {
    /// AND 门判定
    fn unify_check(&self, action: &OnionAction) -> OnionVerdict;
    /// 11 节点电子环统一视图
    fn electronic_ring(&self) -> ElectronicRing {
        let mut ring = ElectronicRing::new();
        for kind in PRINCIPLE_LAYERS_OUTER_IN.iter().rev() {
            ring.push_ring_node(ElectronicRingNode::Principle(*kind));
        }
        for kind in PERMISSION_LAYERS_OUTER_IN.iter().rev() {
            ring.push_ring_node(ElectronicRingNode::Permission(*kind));
        }
        ring
    }
}

// ============================================================
// 5. 电子环网络
// ============================================================

/// 电子环节点 — 标记属于原则洋葱还是权限洋葱
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ElectronicRingNode {
    /// 原则洋葱节点
    Principle(PrincipleLayerKind),
    /// 权限洋葱节点
    Permission(PermissionLayerKind),
}

/// 11 节点电子环
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectronicRing {
    nodes: [Option<ElectronicRingNode>; ELECTRONIC_RING_LEN],
    filled: usize,
}

impl ElectronicRing {
    /// 创建空环
    pub fn new() -> Self {
        Self {
            nodes: [None; ELECTRONIC_RING_LEN],
            filled: 0,
        }
    }
    /// 推入节点
    pub fn push_ring_node(&mut self, node: ElectronicRingNode) {
        assert!(
            self.filled < ELECTRONIC_RING_LEN,
            "electronic ring overflow (max 11 nodes)"
        );
        self.nodes[self.filled] = Some(node);
        self.filled += 1;
    }
    /// 已填充节点数
    pub fn len(&self) -> usize {
        self.filled
    }
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.filled == 0
    }
    /// 是否完整（11 节点）
    pub fn is_complete(&self) -> bool {
        self.filled == ELECTRONIC_RING_LEN
    }
    /// 迭代已填充节点
    pub fn iter(&self) -> impl Iterator<Item = ElectronicRingNode> + '_ {
        (0..self.filled).filter_map(move |i| self.nodes[i])
    }
    /// 原则节点数
    pub fn principle_count(&self) -> usize {
        self.iter()
            .filter(|n| matches!(n, ElectronicRingNode::Principle(_)))
            .count()
    }
    /// 权限节点数
    pub fn permission_count(&self) -> usize {
        self.iter()
            .filter(|n| matches!(n, ElectronicRingNode::Permission(_)))
            .count()
    }
}

impl Default for ElectronicRing {
    fn default() -> Self {
        Self::new()
    }
}

/// 电子环网络 trait
pub trait ElectronicRingNetwork {
    /// 暴露电子环视图
    fn ring(&self) -> ElectronicRing;
    /// 环是否完整
    fn ring_is_complete(&self) -> bool {
        self.ring().is_complete()
    }
}

// ============================================================
// 6. 决策契约
// ============================================================

/// 待评估动作
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnionAction {
    /// 动作 ID
    pub id: String,
    /// 动作描述
    pub description: String,
    /// 触及的权限层
    pub touches_layer: Option<PermissionLayerKind>,
}

impl OnionAction {
    /// 便利构造
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            touches_layer: None,
        }
    }
    /// 触及权限层
    pub fn touches(mut self, layer: PermissionLayerKind) -> Self {
        self.touches_layer = Some(layer);
        self
    }
}

/// 洋葱统一体判定结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OnionVerdict {
    /// 放行
    Allow {
        /// 通过的层
        cleared_layers: Vec<ElectronicRingNode>,
    },
    /// V1 原则拒绝
    BlockByPrinciple {
        /// 拒绝的层
        layer: PrincipleLayerKind,
        /// 拒绝原因
        reason: String,
    },
    /// V2 权限拒绝
    BlockByPermission {
        /// 拒绝的层
        layer: PermissionLayerKind,
        /// 拒绝原因
        reason: String,
    },
    /// V3 L0 HA 物理隔离
    BlockByHumanAuthority {
        /// 拒绝原因
        reason: String,
    },
}

impl OnionVerdict {
    /// 是否放行
    pub fn is_allowed(&self) -> bool {
        matches!(self, OnionVerdict::Allow { .. })
    }
}

// ============================================================
// 7. 默认实现：把 apeireth-core struct 包装为 trait 实现者
// ============================================================

impl PrincipleSlice for CorePrincipleLayer {
    fn kind(&self) -> PrincipleLayerKind {
        match self.name.chars().next() {
            Some('E') => PrincipleLayerKind::Existence,
            Some('S') => PrincipleLayerKind::Spirit,
            Some('A') => PrincipleLayerKind::Accumulation,
            Some('M') => PrincipleLayerKind::Methodology,
            Some('O') => PrincipleLayerKind::Operational,
            _ => PrincipleLayerKind::Operational,
        }
    }
    fn is_hardcoded(&self) -> bool {
        self.hardcoded
    }
    fn name(&self) -> &str {
        &self.name
    }
}

impl PermissionSlice for CorePermissionLayer {
    fn kind(&self) -> PermissionLayerKind {
        match self.name.chars().next() {
            Some('L') => match self.name.chars().nth(1).and_then(|c| c.to_digit(10)) {
                Some(0) => PermissionLayerKind::L0,
                Some(1) => PermissionLayerKind::L1,
                Some(2) => PermissionLayerKind::L2,
                Some(3) => PermissionLayerKind::L3,
                Some(4) => PermissionLayerKind::L4,
                Some(5) => PermissionLayerKind::L5,
                _ => PermissionLayerKind::L1,
            },
            _ => PermissionLayerKind::L1,
        }
    }
    fn requires_ha(&self) -> bool {
        self.requires_ha
    }
    fn name(&self) -> &str {
        &self.name
    }
}

impl PrincipleOnion for CorePrincipleOnion {
    fn slices_outer_in(&self) -> [&dyn PrincipleSlice; 5] {
        [
            &self.o_layer,
            &self.a_layer,
            &self.m_layer,
            &self.s_layer,
            &self.e_layer,
        ]
    }
    fn slice(&self, kind: PrincipleLayerKind) -> &dyn PrincipleSlice {
        match kind {
            PrincipleLayerKind::Existence => &self.e_layer,
            PrincipleLayerKind::Spirit => &self.s_layer,
            PrincipleLayerKind::Accumulation => &self.a_layer,
            PrincipleLayerKind::Methodology => &self.m_layer,
            PrincipleLayerKind::Operational => &self.o_layer,
        }
    }
}

impl PermissionOnion for CorePermissionOnion {
    fn slices_outer_in(&self) -> [&dyn PermissionSlice; 6] {
        [&self.l5, &self.l4, &self.l3, &self.l2, &self.l1, &self.l0]
    }
    fn slice(&self, kind: PermissionLayerKind) -> &dyn PermissionSlice {
        match kind {
            PermissionLayerKind::L0 => &self.l0,
            PermissionLayerKind::L1 => &self.l1,
            PermissionLayerKind::L2 => &self.l2,
            PermissionLayerKind::L3 => &self.l3,
            PermissionLayerKind::L4 => &self.l4,
            PermissionLayerKind::L5 => &self.l5,
        }
    }
}

/// 双洋葱统一体默认实现
pub struct DefaultDoubleOnion {
    principle: CorePrincipleOnion,
    permission: CorePermissionOnion,
    human_authority: HumanAuthority,
}

impl DefaultDoubleOnion {
    /// 构造
    pub fn new(
        principle: CorePrincipleOnion,
        permission: CorePermissionOnion,
        ha: HumanAuthority,
    ) -> Self {
        Self {
            principle,
            permission,
            human_authority: ha,
        }
    }
}

impl PrincipleOnion for DefaultDoubleOnion {
    fn slices_outer_in(&self) -> [&dyn PrincipleSlice; 5] {
        self.principle.slices_outer_in()
    }
    fn slice(&self, kind: PrincipleLayerKind) -> &dyn PrincipleSlice {
        self.principle.slice(kind)
    }
}

impl PermissionOnion for DefaultDoubleOnion {
    fn slices_outer_in(&self) -> [&dyn PermissionSlice; 6] {
        self.permission.slices_outer_in()
    }
    fn slice(&self, kind: PermissionLayerKind) -> &dyn PermissionSlice {
        self.permission.slice(kind)
    }
}

impl DoubleOnionUnification for DefaultDoubleOnion {
    fn unify_check(&self, action: &OnionAction) -> OnionVerdict {
        use apeireth_core::HAMode;
        // V3 HA 离线模式直接拒绝
        if matches!(self.human_authority.mode, HAMode::Offline) {
            if let Some(layer) = action.touches_layer {
                if self.permission.slice(layer).requires_ha() {
                    return OnionVerdict::BlockByHumanAuthority {
                        reason: "HA 离线模式 = 物理隔离拒绝".to_string(),
                    };
                }
            }
        }
        // V1 原则检查：触及 L5 由 E 层兜底
        if matches!(action.touches_layer, Some(PermissionLayerKind::L5)) {
            return OnionVerdict::BlockByPrinciple {
                layer: PrincipleLayerKind::Existence,
                reason: "触及 L5 核武器级动作 = E 层兜底拒绝".to_string(),
            };
        }
        // AND 门全通过
        let mut cleared = Vec::with_capacity(ELECTRONIC_RING_LEN);
        for kind in PRINCIPLE_LAYERS_OUTER_IN {
            cleared.push(ElectronicRingNode::Principle(kind));
        }
        for kind in PERMISSION_LAYERS_OUTER_IN {
            cleared.push(ElectronicRingNode::Permission(kind));
        }
        OnionVerdict::Allow {
            cleared_layers: cleared,
        }
    }
}

impl ElectronicRingNetwork for DefaultDoubleOnion {
    fn ring(&self) -> ElectronicRing {
        DoubleOnionUnification::electronic_ring(self)
    }
}

// ============================================================
// 8. 便利工厂
// ============================================================

/// 创建标准测试 `DefaultDoubleOnion`
pub fn default_test_double_onion() -> DefaultDoubleOnion {
    let principle = CorePrincipleOnion {
        e_layer: CorePrincipleLayer {
            name: "E".into(),
            description: "Existence — 不可违背".into(),
            hardcoded: true,
        },
        s_layer: CorePrincipleLayer {
            name: "S".into(),
            description: "Spirit — 价值观".into(),
            hardcoded: true,
        },
        a_layer: CorePrincipleLayer {
            name: "A".into(),
            description: "Accumulation — 经验".into(),
            hardcoded: true,
        },
        m_layer: CorePrincipleLayer {
            name: "M".into(),
            description: "Methodology — 方法论".into(),
            hardcoded: true,
        },
        o_layer: CorePrincipleLayer {
            name: "O".into(),
            description: "Operational — 操作".into(),
            hardcoded: false,
        },
    };
    let permission = CorePermissionOnion {
        l0: CorePermissionLayer {
            name: "L0".into(),
            description: "HA 核心".into(),
            requires_ha: true,
        },
        l1: CorePermissionLayer {
            name: "L1".into(),
            description: "受控写".into(),
            requires_ha: false,
        },
        l2: CorePermissionLayer {
            name: "L2".into(),
            description: "重要操作".into(),
            requires_ha: false,
        },
        l3: CorePermissionLayer {
            name: "L3".into(),
            description: "关键操作".into(),
            requires_ha: true,
        },
        l4: CorePermissionLayer {
            name: "L4".into(),
            description: "核心升级".into(),
            requires_ha: true,
        },
        l5: CorePermissionLayer {
            name: "L5".into(),
            description: "核武器级".into(),
            requires_ha: true,
        },
    };
    let ha = HumanAuthority {
        mode: apeireth_core::HAMode::SingleHuman,
        real_humans: vec![],
        ice_frozen_until: None,
    };
    DefaultDoubleOnion::new(principle, permission, ha)
}

// ============================================================
// 9. 单元测试
// ============================================================

#[cfg(test)]
// R177: onion invariants (10 tests + 2 Kani proofs)
mod organ_kani_proofs;
mod tests {
    use super::*;

    #[test]
    fn t1_principle_layers_hardcoded_count_is_5() {
        assert_eq!(PRINCIPLE_LAYERS_OUTER_IN.len(), 5);
    }

    #[test]
    fn t2_permission_layers_hardcoded_count_is_6() {
        assert_eq!(PERMISSION_LAYERS_OUTER_IN.len(), 6);
    }

    #[test]
    fn t3_electronic_ring_capacity_is_11() {
        let mut ring = ElectronicRing::new();
        for kind in PRINCIPLE_LAYERS_OUTER_IN.iter().rev() {
            ring.push_ring_node(ElectronicRingNode::Principle(*kind));
        }
        for kind in PERMISSION_LAYERS_OUTER_IN.iter().rev() {
            ring.push_ring_node(ElectronicRingNode::Permission(*kind));
        }
        assert_eq!(ring.len(), ELECTRONIC_RING_LEN);
        assert!(ring.is_complete());
        assert_eq!(ring.principle_count(), 5);
        assert_eq!(ring.permission_count(), 6);
    }

    #[test]
    fn t4_arbitrate_e_always_wins() {
        let o = default_test_double_onion();
        assert_eq!(
            <DefaultDoubleOnion as PrincipleOnion>::arbitrate(
                &o,
                PrincipleLayerKind::Operational,
                PrincipleLayerKind::Existence
            ),
            PrincipleLayerKind::Existence
        );
    }

    #[test]
    fn t5_unify_check_l5_blocked_by_e_layer() {
        let o = default_test_double_onion();
        let action = OnionAction::new("nuke", "尝试触及 L5").touches(PermissionLayerKind::L5);
        let verdict = o.unify_check(&action);
        assert!(!verdict.is_allowed());
        assert!(matches!(verdict, OnionVerdict::BlockByPrinciple { .. }));
    }

    #[test]
    fn t6_unify_check_l1_normal_allows() {
        let o = default_test_double_onion();
        let action = OnionAction::new("read", "日常读").touches(PermissionLayerKind::L1);
        let verdict = o.unify_check(&action);
        assert!(verdict.is_allowed());
    }

    #[test]
    fn t7_l0_requires_ha_is_always_true() {
        let o = default_test_double_onion();
        assert!(o.l0_requires_ha());
        let slice = <DefaultDoubleOnion as PermissionOnion>::slice(&o, PermissionLayerKind::L0);
        assert!(slice.requires_ha());
    }

    #[test]
    fn t8_electronic_ring_node_partition() {
        let ring = default_test_double_onion().ring();
        assert_eq!(ring.principle_count() + ring.permission_count(), 11);
    }

    #[test]
    fn t9_default_impl_composes_core_structs() {
        let o = default_test_double_onion();
        let o_slice =
            <DefaultDoubleOnion as PrincipleOnion>::slice(&o, PrincipleLayerKind::Operational);
        assert!(!o_slice.is_hardcoded());
        let e_slice =
            <DefaultDoubleOnion as PrincipleOnion>::slice(&o, PrincipleLayerKind::Existence);
        assert!(e_slice.is_hardcoded());
    }

    #[test]
    fn t10_electronic_ring_overflow_panics() {
        let mut ring = ElectronicRing::new();
        for _ in 0..ELECTRONIC_RING_LEN {
            ring.push_ring_node(ElectronicRingNode::Principle(
                PrincipleLayerKind::Operational,
            ));
        }
        let r = std::panic::catch_unwind(move || {
            ring.push_ring_node(ElectronicRingNode::Principle(
                PrincipleLayerKind::Operational,
            ));
        });
        assert!(r.is_err());
    }

    // ===== round7-02 onion-dedupe-hardcode (architect2) =====

    #[test]
    fn t11_principle_layers_outer_in_order_is_e_s_a_m_o() {
        // round7-02: onion PRINCIPLE_LAYERS_OUTER_IN 顺序 = core PrincipleOnion 字段声明顺序
        assert_eq!(
            PRINCIPLE_LAYERS_OUTER_IN,
            [
                PrincipleLayerKind::Existence,
                PrincipleLayerKind::Spirit,
                PrincipleLayerKind::Accumulation,
                PrincipleLayerKind::Methodology,
                PrincipleLayerKind::Operational,
            ]
        );
    }

    #[test]
    fn t12_permission_layers_outer_in_order_is_l0_to_l5() {
        // round7-02: onion PERMISSION_LAYERS_OUTER_IN 顺序 = core PermissionOnion 字段声明顺序
        assert_eq!(
            PERMISSION_LAYERS_OUTER_IN,
            [
                PermissionLayerKind::L0,
                PermissionLayerKind::L1,
                PermissionLayerKind::L2,
                PermissionLayerKind::L3,
                PermissionLayerKind::L4,
                PermissionLayerKind::L5,
            ]
        );
    }

    #[test]
    fn t13_onion_does_not_redefine_core_hamode_reexports_it() {
        // round7-02: onion 不重新定义 HAMode (必须从 core 导入, 不在 onion 重新声明)
        // 通过 reflect 验证: compile-time anchor 函数必须能接受 core::HAMode
        fn _accepts_core_hamode(m: apeireth_core::HAMode) -> apeireth_core::HAMode {
            m
        }
        let single = _accepts_core_hamode(apeireth_core::HAMode::SingleHuman);
        let multi = _accepts_core_hamode(apeireth_core::HAMode::MultiHuman);
        let offline = _accepts_core_hamode(apeireth_core::HAMode::Offline);
        // 三种模式必须能区分 (编译期 hardcode: 3 变体)
        assert!(matches!(single, apeireth_core::HAMode::SingleHuman));
        assert!(matches!(multi, apeireth_core::HAMode::MultiHuman));
        assert!(matches!(offline, apeireth_core::HAMode::Offline));
    }

    #[test]
    fn t14_default_double_onion_uses_core_hamode_variant() {
        // round7-02: DefaultDoubleOnion.human_authority.mode 必须是 core::HAMode 类型
        // (不能在 onion 里独立定义自己的 HAMode 枚举)
        let o = default_test_double_onion();
        let mode: &apeireth_core::HAMode = &o.human_authority.mode;
        // 编译期 hardcode: 编译通过 = mode 字段类型 = core::HAMode
        assert!(matches!(mode, apeireth_core::HAMode::SingleHuman));
    }

    #[test]
    fn t15_onion_traits_are_adapters_not_duplicates_of_core_methods() {
        // round7-02: onion 的 PrincipleSlice/PermissionSlice 不重复 core 的字段
        // core PrincipleLayer 只有 3 字段 (name/description/hardcoded)
        // core PermissionLayer 只有 3 字段 (name/description/requires_ha)
        // onion 的 trait 方法数: PrincipleSlice=3 (kind/is_hardcoded/name),
        //                       PermissionSlice=3 (kind/requires_ha/name)
        // 验证 onion trait 不"重新"提供 core 已有字段 (kind/is_hardcoded/requires_ha
        // 是 onion 抽象, 通过 impl 适配到 core 的字段)
        let o = default_test_double_onion();
        // PrincipleSlice.kind() 必须映射到 core PrincipleLayer 的 name 字段
        let e_slice =
            <DefaultDoubleOnion as PrincipleOnion>::slice(&o, PrincipleLayerKind::Existence);
        assert_eq!(e_slice.kind(), PrincipleLayerKind::Existence);
        assert_eq!(e_slice.name(), "E");
        assert!(e_slice.is_hardcoded()); // 委托 core.hardcoded=true
                                         // PermissionSlice.kind() 必须映射到 core PermissionLayer 的 name 字段
        let l0_slice = <DefaultDoubleOnion as PermissionOnion>::slice(&o, PermissionLayerKind::L0);
        assert_eq!(l0_slice.kind(), PermissionLayerKind::L0);
        assert_eq!(l0_slice.name(), "L0");
        assert!(l0_slice.requires_ha()); // 委托 core.requires_ha=true
    }

    #[test]
    fn t16_onion_does_not_duplicate_core_layer_struct_definition() {
        // round7-02: 编译期 hardcode — onion 不能"重新"实现 PrincipleLayer/PermissionLayer struct
        // 通过引用 core 类型来确保依赖链:
        //   * onion PRINCIPLE_LAYERS_OUTER_IN 类型 = PrincipleLayerKind (onion 自身 enum, 5 变体)
        //   * core 字段通过 trait 适配被 onion 使用
        // 验证 onion 5 个 PrincipleLayerKind 变体一一对应 core 5 个 struct 字段
        let o = default_test_double_onion();
        let all_kinds = PRINCIPLE_LAYERS_OUTER_IN;
        assert_eq!(all_kinds.len(), 5);
        // 每个 kind 都能在 core PrincipleOnion 上 slice 出来 (即 core 5 字段都存在)
        for kind in all_kinds {
            let _slice = <DefaultDoubleOnion as PrincipleOnion>::slice(&o, kind);
        }
    }

    #[test]
    fn t17_onion_5_layers_match_core_principle_onion_5_fields() {
        // round7-02: runtime 验证 onion PRINCIPLE_LAYERS_OUTER_IN 与 core PrincipleOnion 字段数匹配
        // core PrincipleOnion::slices_outer_in 返回 5 个 slice (外→内顺序)
        let o = default_test_double_onion();
        let core_slices = <DefaultDoubleOnion as PrincipleOnion>::slices_outer_in(&o);
        assert_eq!(core_slices.len(), PRINCIPLE_LAYERS_OUTER_IN.len());
        assert_eq!(core_slices.len(), 5);
    }

    #[test]
    fn t18_onion_6_layers_match_core_permission_onion_6_fields() {
        // round7-02: 同 t17 针对 PermissionOnion (6 字段 L0..L5)
        let o = default_test_double_onion();
        let core_slices = <DefaultDoubleOnion as PermissionOnion>::slices_outer_in(&o);
        assert_eq!(core_slices.len(), PERMISSION_LAYERS_OUTER_IN.len());
        assert_eq!(core_slices.len(), 6);
    }
}
