//! `apeireth-core::onion_gate` — 双洋葱统一体**判定层** (W3 三洋葱 L3-L5 判定模型移植)。
//!
//! **移植来源** (防重造轮子 diff 记录, 2026-10-10): v1 `legacy/donor/apeireth-onion`
//! (trait 抽象层 + `DefaultDoubleOnion::unify_check` 三段门 + 11 节点电子环 +
//! `organ_kani_proofs.rs` r177 证明组)。v1 架构 = "比喻 (双洋葱) → trait 抽象层 →
//! `apeireth-core` 数据结构"; v2 的**数据结构层已先在** (`onion.rs`: `PrincipleOnion`
//! 5 切片 / `PermissionOnion` L0-L5 / `HumanAuthority` M-of-N), 本模块 = 其上的
//! **判定层**, **不重定义任何 struct** (0 触碰 onion.rs 公开签名)。
//!
//! **与 v1 的适配差异 (如实记录)**:
//! - v1 `PrincipleSlice`/`PermissionSlice` trait + `requires_ha()` 方法 → v2 的
//!   `PermissionLayer.requires_ha` 已是**字段** (数据层自带), 故只补索引访问
//!   (`slices_outer_in`) 不造 trait;
//! - v1 Kani 形式化证明 (`organ_kani_proofs.rs`) → 本仓无 Kani harness, 平移为
//!   **等价 Rust 断言测试** (r177 编号保留, 语义不变; 真 Kani 接线 = 后续项);
//! - 真 Ed25519 多签仍留 v2.1 (`HumanAuthority::verify_multisig` 0 装占位不动) ——
//!   本层只做**结构与权威来源判定**, 不做密码学判定。
//!
//! **判定语义 (v1 `unify_check` 三段门, 原样移植)**:
//! 1. **HA 离线模式 = 物理隔离拒绝**: 动作触及的权限层 `requires_ha` → `BlockByHumanAuthority`;
//! 2. **触及 L5 = E 层兜底拒绝** (核武器级动作由存在层直接拒);
//! 3. 否则 `Allow { cleared_layers }` = 11 环全节点 (5 原则 + 6 权限)。
//!
//! **0 假装**: 编译期 hardcode (5+6=11 const 断言) + 纯函数判定 (0 IO / 0 unsafe /
//! 依赖仅 serde)。物理执行面 (governance/approval + W1 沙箱) 的装配 = 后续项。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::onion::{
    HAMode, HumanAuthority, PermissionLayer, PermissionOnion, PrincipleLayer, PrincipleOnion,
};

// ============================================================
// 1. 层身份 (编译时 hardcode, const fn 断言)
// ============================================================

/// 原则洋葱 5 层 (Existence / Spirit / Accumulation / Methodology / Operational)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrincipleLayerKind {
    /// E 层 — 存在 (6 项不可违背原则, 编译时 hardcode)
    Existence,
    /// S 层 — 价值
    Spirit,
    /// A 层 — 经验沉淀
    Accumulation,
    /// M 层 — 方法论
    Methodology,
    /// O 层 — 操作原则
    Operational,
}

/// 编译时 hardcode: 5 个原则层按"内→外"顺序 (深→浅)
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
    /// L0 — HA 核心 (不可变, 最后护栏)
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

/// 编译时 hardcode: 6 个权限层按"内→外"顺序
pub const PERMISSION_LAYERS_OUTER_IN: [PermissionLayerKind; 6] = [
    PermissionLayerKind::L0,
    PermissionLayerKind::L1,
    PermissionLayerKind::L2,
    PermissionLayerKind::L3,
    PermissionLayerKind::L4,
    PermissionLayerKind::L5,
];

/// 11 节点电子环 (5 + 6 = 11 切片统一环)
pub const ELECTRONIC_RING_LEN: usize = 11;

const _ONION_LAYER_ASSERTIONS: () = {
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
    // round7-02 onion-dedupe-hardcode (v1 平移): 通过 const fn 字段访问交叉校验
    // onion.rs 的 struct 字段未被重命名/删除 —— 字段访问即类型检查。
    const fn _enforce_core_principle_layers(p: &PrincipleOnion) -> [PrincipleLayerKind; 5] {
        let _e = &p.e_layer;
        let _s = &p.s_layer;
        let _a = &p.a_layer;
        let _m = &p.m_layer;
        let _o = &p.o_layer;
        PRINCIPLE_LAYERS_OUTER_IN
    }
    const fn _enforce_core_permission_layers(p: &PermissionOnion) -> [PermissionLayerKind; 6] {
        let _l0 = &p.l0;
        let _l1 = &p.l1;
        let _l2 = &p.l2;
        let _l3 = &p.l3;
        let _l4 = &p.l4;
        let _l5 = &p.l5;
        PERMISSION_LAYERS_OUTER_IN
    }
};

// ============================================================
// 2. 层身份 → 数据层映射 (复用 onion.rs struct, 0 重定义)
// ============================================================

impl PrincipleOnion {
    /// 5 层切片按"外→内"顺序 (E/S/A/M/O, 与 `PRINCIPLE_LAYERS_OUTER_IN` 对齐)。
    pub fn slices_outer_in(&self) -> [&PrincipleLayer; 5] {
        [
            &self.e_layer,
            &self.s_layer,
            &self.a_layer,
            &self.m_layer,
            &self.o_layer,
        ]
    }

    /// 按层身份取切片。
    pub fn slice(&self, kind: PrincipleLayerKind) -> &PrincipleLayer {
        match kind {
            PrincipleLayerKind::Existence => &self.e_layer,
            PrincipleLayerKind::Spirit => &self.s_layer,
            PrincipleLayerKind::Accumulation => &self.a_layer,
            PrincipleLayerKind::Methodology => &self.m_layer,
            PrincipleLayerKind::Operational => &self.o_layer,
        }
    }
}

impl PermissionOnion {
    /// 6 层切片按"外→内"顺序 (L0..L5, 与 `PERMISSION_LAYERS_OUTER_IN` 对齐)。
    pub fn slices_outer_in(&self) -> [&PermissionLayer; 6] {
        [&self.l0, &self.l1, &self.l2, &self.l3, &self.l4, &self.l5]
    }

    /// 按层身份取切片。
    pub fn slice(&self, kind: PermissionLayerKind) -> &PermissionLayer {
        match kind {
            PermissionLayerKind::L0 => &self.l0,
            PermissionLayerKind::L1 => &self.l1,
            PermissionLayerKind::L2 => &self.l2,
            PermissionLayerKind::L3 => &self.l3,
            PermissionLayerKind::L4 => &self.l4,
            PermissionLayerKind::L5 => &self.l5,
        }
    }

    /// 某权限层是否需要 HA 真实人类批准 (数据层 `requires_ha` 字段直读)。
    pub fn layer_requires_ha(&self, kind: PermissionLayerKind) -> bool {
        self.slice(kind).requires_ha
    }

    /// **不变式**: L0 永远需要 HA (r177_oni 证明组的不变量; 构造期由
    /// [`crate::onion_gate::DoubleOnionGate::new`] 强校验)。
    pub fn l0_requires_ha(&self) -> bool {
        self.l0.requires_ha
    }
}

// ============================================================
// 3. 跨层仲裁 (§3.6: E 胜所有 > S > A > M > O)
// ============================================================

/// 原则跨层冲突仲裁 (纯函数): 更深的层胜 —— E > S > A > M > O。
pub fn arbitrate_principles(a: PrincipleLayerKind, b: PrincipleLayerKind) -> PrincipleLayerKind {
    fn depth(kind: PrincipleLayerKind) -> u8 {
        match kind {
            PrincipleLayerKind::Existence => 0,
            PrincipleLayerKind::Spirit => 1,
            PrincipleLayerKind::Accumulation => 2,
            PrincipleLayerKind::Methodology => 3,
            PrincipleLayerKind::Operational => 4,
        }
    }
    if depth(a) <= depth(b) {
        a
    } else {
        b
    }
}

// ============================================================
// 4. 电子环 (11 节点统一视图)
// ============================================================

/// 电子环节点 — 标记属于原则洋葱还是权限洋葱
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ElectronicRingNode {
    /// 原则洋葱节点
    Principle(PrincipleLayerKind),
    /// 权限洋葱节点
    Permission(PermissionLayerKind),
}

/// 11 节点电子环 (5 原则 + 6 权限的填充视图)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectronicRing {
    nodes: Vec<ElectronicRingNode>,
}

impl ElectronicRing {
    /// 空环。
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// 压入节点 (超过 11 = 编程错误, 静默丢次会破坏不变式, 故显式返回 false)。
    pub fn push_ring_node(&mut self, node: ElectronicRingNode) -> bool {
        if self.nodes.len() >= ELECTRONIC_RING_LEN {
            return false;
        }
        self.nodes.push(node);
        true
    }

    /// 已填充节点数。
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 是否空环。
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// 是否满环 (11)。
    pub fn is_complete(&self) -> bool {
        self.nodes.len() == ELECTRONIC_RING_LEN
    }

    /// 遍历 (外→内序)。
    pub fn iter(&self) -> impl Iterator<Item = ElectronicRingNode> + '_ {
        self.nodes.iter().copied()
    }

    /// 原则节点数。
    pub fn principle_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| matches!(n, ElectronicRingNode::Principle(_)))
            .count()
    }

    /// 权限节点数。
    pub fn permission_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| matches!(n, ElectronicRingNode::Permission(_)))
            .count()
    }
}

impl Default for ElectronicRing {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 5. 动作 / 判定 (unify_check 三段门)
// ============================================================

/// 待判定动作 (触及的权限层 = 判定的输入)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnionAction {
    /// 动作 id
    pub id: String,
    /// 动作描述
    pub description: String,
    /// 触及的权限层 (None = 未触及任何权限层的纯查询类动作)
    pub touches_layer: Option<PermissionLayerKind>,
}

impl OnionAction {
    /// 构造。
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            touches_layer: None,
        }
    }

    /// 标记触及的权限层。
    #[must_use]
    pub fn touches(mut self, layer: PermissionLayerKind) -> Self {
        self.touches_layer = Some(layer);
        self
    }
}

/// 双洋葱统一体判定结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnionVerdict {
    /// 放行 (附 11 环全节点清除清单)。
    Allow {
        /// 通过的 11 环节点
        cleared_layers: Vec<ElectronicRingNode>,
    },
    /// 原则层兜底拒绝 (如 L5 → E 层)。
    BlockByPrinciple {
        /// 兜底的原则层
        layer: PrincipleLayerKind,
        /// 理由
        reason: String,
    },
    /// HA 物理隔离拒绝 (离线模式触及需 HA 层)。
    BlockByHumanAuthority {
        /// 理由
        reason: String,
    },
}

impl OnionVerdict {
    /// 是否放行。
    pub fn is_allowed(&self) -> bool {
        matches!(self, OnionVerdict::Allow { .. })
    }
}

// ============================================================
// 6. 双洋葱门 (数据层 + 判定逻辑的装配; 无 IO)
// ============================================================

/// 双洋葱统一体判定门 — 装配 `onion.rs` 数据层, 提供 `unify_check` 判定。
///
/// **构造期不变式校验** (fail-loud): `PermissionOnion.l0.requires_ha` 必须为 true
/// (r177 "L0 requires HA" 不变量; 被改坏的配置在装配期就炸, 不流入运行时)。
#[derive(Debug, Clone)]
pub struct DoubleOnionGate {
    principle: PrincipleOnion,
    permission: PermissionOnion,
    human_authority: HumanAuthority,
}

impl DoubleOnionGate {
    /// 构造 (不变式: L0 恒需 HA, 否则 `Err`)。
    pub fn new(
        principle: PrincipleOnion,
        permission: PermissionOnion,
        human_authority: HumanAuthority,
    ) -> Result<Self, OnionGateError> {
        if !permission.l0_requires_ha() {
            return Err(OnionGateError::InvariantViolation(
                "L0 must require HA (r177 不变式: 最后护栏不可拆)".to_string(),
            ));
        }
        Ok(Self {
            principle,
            permission,
            human_authority,
        })
    }

    /// 原则层视图。
    pub fn principle(&self) -> &PrincipleOnion {
        &self.principle
    }

    /// 替换 HA 权威 (builder; 不变式 L0 恒需 HA 仍由构造期锁定)。
    #[must_use]
    pub fn with_human_authority(mut self, human_authority: HumanAuthority) -> Self {
        self.human_authority = human_authority;
        self
    }

    /// 权限层视图。
    pub fn permission(&self) -> &PermissionOnion {
        &self.permission
    }

    /// HA 视图。
    pub fn human_authority(&self) -> &HumanAuthority {
        &self.human_authority
    }

    /// **判定 (v1 `unify_check` 三段门原样移植)**:
    /// 1. HA 离线 + 触及需 HA 层 → 物理隔离拒绝;
    /// 2. 触及 L5 → E 层兜底拒绝;
    /// 3. 否则 → 11 环全节点放行。
    pub fn unify_check(&self, action: &OnionAction) -> OnionVerdict {
        // ① HA 离线模式 = 物理隔离拒绝
        if matches!(self.human_authority.mode, HAMode::Offline) {
            if let Some(layer) = action.touches_layer {
                if self.permission.layer_requires_ha(layer) {
                    return OnionVerdict::BlockByHumanAuthority {
                        reason: "HA 离线模式 = 物理隔离拒绝".to_string(),
                    };
                }
            }
        }
        // ② 触及 L5 核武器级动作 = E 层兜底
        if matches!(action.touches_layer, Some(PermissionLayerKind::L5)) {
            return OnionVerdict::BlockByPrinciple {
                layer: PrincipleLayerKind::Existence,
                reason: "触及 L5 核武器级动作 = E 层兜底拒绝".to_string(),
            };
        }
        // ③ AND 门全通过 = 11 环全节点清除
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

    /// 11 节点电子环统一视图 (原则外→内 + 权限外→内)。
    pub fn electronic_ring(&self) -> ElectronicRing {
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

/// 门构造错误 (装配期 fail-loud)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnionGateError {
    /// 不变式违反 (如 L0 不需 HA)。
    InvariantViolation(String),
}

impl std::fmt::Display for OnionGateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvariantViolation(reason) => {
                write!(f, "onion gate invariant violated: {reason}")
            }
        }
    }
}

impl std::error::Error for OnionGateError {}

// ============================================================
// 7. 便利工厂 (测试/演示装配)
// ============================================================

/// 标准原则洋葱装配 (层名/描述/硬编码位)。
pub fn standard_principle_onion() -> PrincipleOnion {
    PrincipleOnion {
        e_layer: PrincipleLayer {
            name: "E".into(),
            description: "Existence — 不可违背".into(),
            hardcoded: true,
        },
        s_layer: PrincipleLayer {
            name: "S".into(),
            description: "Spirit — 价值观".into(),
            hardcoded: true,
        },
        a_layer: PrincipleLayer {
            name: "A".into(),
            description: "Accumulation — 经验沉淀".into(),
            hardcoded: false,
        },
        m_layer: PrincipleLayer {
            name: "M".into(),
            description: "Methodology — 方法论".into(),
            hardcoded: false,
        },
        o_layer: PrincipleLayer {
            name: "O".into(),
            description: "Operational — 操作原则".into(),
            hardcoded: false,
        },
    }
}

/// 标准权限洋葱装配 (L0 恒 requires_ha = true; L1/L2 否; L3+ 需 HA)。
pub fn standard_permission_onion() -> PermissionOnion {
    PermissionOnion {
        l0: PermissionLayer {
            name: "L0".into(),
            description: "HA 核心 (不可变, 最后护栏)".into(),
            requires_ha: true,
        },
        l1: PermissionLayer {
            name: "L1".into(),
            description: "受控写".into(),
            requires_ha: false,
        },
        l2: PermissionLayer {
            name: "L2".into(),
            description: "重要操作".into(),
            requires_ha: false,
        },
        l3: PermissionLayer {
            name: "L3".into(),
            description: "关键操作".into(),
            requires_ha: true,
        },
        l4: PermissionLayer {
            name: "L4".into(),
            description: "核心升级".into(),
            requires_ha: true,
        },
        l5: PermissionLayer {
            name: "L5".into(),
            description: "核武器级".into(),
            requires_ha: true,
        },
    }
}

/// 单人 HA 装配。
pub fn single_human_authority() -> HumanAuthority {
    HumanAuthority {
        mode: HAMode::SingleHuman,
        real_humans: vec![],
        ice_frozen_until: None,
        multi_sign: None,
    }
}

/// 标准测试门 (单人 HA + 标准双洋葱; 离线门测试另行覆写 `human_authority`)。
pub fn standard_double_onion_gate() -> DoubleOnionGate {
    DoubleOnionGate::new(
        standard_principle_onion(),
        standard_permission_onion(),
        single_human_authority(),
    )
    .expect("standard assemblies satisfy the L0-HA invariant")
}

#[cfg(test)]
mod onion_gate_tests {
    use super::*;

    // ---- r177 Kani 等价证明组 (organ_kani_proofs.rs 平移) ----

    #[test]
    fn r177_oni_01_principle_layers_5() {
        assert_eq!(PRINCIPLE_LAYERS_OUTER_IN.len(), 5);
    }

    #[test]
    fn r177_oni_02_principle_layers_distinct() {
        let mut seen = std::collections::HashSet::new();
        for layer in &PRINCIPLE_LAYERS_OUTER_IN {
            assert!(seen.insert(*layer), "原则层重复: {layer:?}");
        }
        assert_eq!(seen.len(), 5);
    }

    #[test]
    fn r177_oni_03_permission_layers_6() {
        assert_eq!(PERMISSION_LAYERS_OUTER_IN.len(), 6);
    }

    #[test]
    fn r177_oni_04_permission_layers_distinct() {
        let mut seen = std::collections::HashSet::new();
        for layer in &PERMISSION_LAYERS_OUTER_IN {
            assert!(seen.insert(*layer), "权限层重复: {layer:?}");
        }
        assert_eq!(seen.len(), 6);
    }

    #[test]
    fn r177_oni_05_electronic_ring_11() {
        assert_eq!(ELECTRONIC_RING_LEN, 11);
        assert_eq!(
            PRINCIPLE_LAYERS_OUTER_IN.len() + PERMISSION_LAYERS_OUTER_IN.len(),
            ELECTRONIC_RING_LEN
        );
    }

    #[test]
    fn r177_oni_06_l0_requires_ha_invariant() {
        // Kani `double_onion_sample` (L0 requires HA) 的 Rust 等价: 标准装配 +
        // 构造期强校验; 被改坏 (L0 不需 HA) 的装配必须在构造期炸。
        assert!(standard_permission_onion().l0_requires_ha());
        let mut broken = standard_permission_onion();
        broken.l0.requires_ha = false;
        assert!(matches!(
            DoubleOnionGate::new(standard_principle_onion(), broken, single_human_authority()),
            Err(OnionGateError::InvariantViolation(_))
        ));
    }

    // ---- 层映射/仲裁 ----

    #[test]
    fn slices_map_in_order() {
        let principle = standard_principle_onion();
        assert_eq!(principle.slices_outer_in().len(), 5);
        assert_eq!(principle.slice(PrincipleLayerKind::Existence).name, "E");
        assert_eq!(principle.slice(PrincipleLayerKind::Operational).name, "O");
        let permission = standard_permission_onion();
        assert_eq!(permission.slices_outer_in().len(), 6);
        assert_eq!(permission.slice(PermissionLayerKind::L0).name, "L0");
        assert!(permission.layer_requires_ha(PermissionLayerKind::L3));
        assert!(!permission.layer_requires_ha(PermissionLayerKind::L1));
    }

    #[test]
    fn arbitrate_prefers_deeper_layer() {
        assert_eq!(
            arbitrate_principles(
                PrincipleLayerKind::Operational,
                PrincipleLayerKind::Existence
            ),
            PrincipleLayerKind::Existence
        );
        assert_eq!(
            arbitrate_principles(PrincipleLayerKind::Spirit, PrincipleLayerKind::Methodology),
            PrincipleLayerKind::Spirit
        );
        assert_eq!(
            arbitrate_principles(
                PrincipleLayerKind::Methodology,
                PrincipleLayerKind::Operational
            ),
            PrincipleLayerKind::Methodology
        );
    }

    // ---- 电子环 ----

    #[test]
    fn electronic_ring_is_complete_and_labeled() {
        let ring = standard_double_onion_gate().electronic_ring();
        assert!(ring.is_complete(), "11 环必须满");
        assert_eq!(ring.principle_count(), 5);
        assert_eq!(ring.permission_count(), 6);
    }

    #[test]
    fn ring_rejects_overflow() {
        let mut ring = ElectronicRing::new();
        for kind in PERMISSION_LAYERS_OUTER_IN {
            assert!(ring.push_ring_node(ElectronicRingNode::Permission(kind)));
        }
        for kind in PRINCIPLE_LAYERS_OUTER_IN {
            assert!(ring.push_ring_node(ElectronicRingNode::Principle(kind)));
        }
        assert!(!ring.push_ring_node(ElectronicRingNode::Permission(PermissionLayerKind::L0)));
    }

    // ---- unify_check 三段门 ----

    #[test]
    fn unify_check_allows_clearing_all_eleven() {
        let gate = standard_double_onion_gate();
        let verdict =
            gate.unify_check(&OnionAction::new("a1", "受控写").touches(PermissionLayerKind::L1));
        assert!(verdict.is_allowed());
        let OnionVerdict::Allow { cleared_layers } = verdict else {
            panic!("expected Allow");
        };
        assert_eq!(cleared_layers.len(), ELECTRONIC_RING_LEN);
    }

    #[test]
    fn unify_check_blocks_l5_by_existence() {
        let gate = standard_double_onion_gate();
        let verdict =
            gate.unify_check(&OnionAction::new("a2", "核动作").touches(PermissionLayerKind::L5));
        match verdict {
            OnionVerdict::BlockByPrinciple { layer, .. } => {
                assert_eq!(layer, PrincipleLayerKind::Existence)
            }
            other => panic!("expected BlockByPrinciple, got {other:?}"),
        }
    }

    #[test]
    fn unify_check_blocks_ha_layer_when_offline() {
        let mut gate = standard_double_onion_gate();
        gate.human_authority.mode = HAMode::Offline;
        let verdict =
            gate.unify_check(&OnionAction::new("a3", "核心升级").touches(PermissionLayerKind::L4));
        match verdict {
            OnionVerdict::BlockByHumanAuthority { .. } => {}
            other => panic!("expected BlockByHumanAuthority, got {other:?}"),
        }
        // 离线但触及非 HA 层 (L1) = 仍放行 (物理隔离只挡需 HA 的动作)。
        let verdict_l1 =
            gate.unify_check(&OnionAction::new("a4", "受控写").touches(PermissionLayerKind::L1));
        assert!(verdict_l1.is_allowed(), "L1 不触 HA 应放行");
    }

    #[test]
    fn unify_check_allows_layerless_action() {
        let gate = standard_double_onion_gate();
        assert!(gate
            .unify_check(&OnionAction::new("a5", "纯查询"))
            .is_allowed());
    }

    #[test]
    fn offline_multisig_stays_denied_by_existing_verifier() {
        // 防重造轮子自证: 密码学判定仍由 onion.rs::HumanAuthority::verify_multisig
        // 承担 (0 装占位不动), 本层不重复签名逻辑。
        let mut ha = single_human_authority();
        ha.mode = HAMode::Offline;
        assert_eq!(
            ha.verify_multisig(&["alice:digest".to_string()]),
            crate::onion::MultiSignResult::DeniedOffline
        );
    }
}
