//! `apeireth-core::onion` — 双洋葱统一体 (PrincipleOnion + PermissionOnion + HumanAuthority)
//!
//! 拆自 `lib.rs` line 33-147 (R131 架构债清理). 0 触碰公开签名 — `use apeireth_core::PrincipleOnion` 等仍可用.
//!
//! 包含: typedef 本段所有 `pub struct` / `pub enum` / `pub trait` / `pub const`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::philosophy::{PhilosophyKey, ALL_THIRTEEN_KEYS};

// 2. 双洋葱统一体 (PrincipleOnion + PermissionOnion + HumanAuthority)
// ============================================

/// 原则洋葱 (5 切片: E/S/A/M/O, 嵌入权限每一层)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleOnion {
    /// E 存在层 (不可降级, 嵌入所有 L0-L5)
    pub e_layer: PrincipleLayer,
    /// S 价值层 (智囊团审议+物理多签, 嵌入 L5-L6)
    pub s_layer: PrincipleLayer,
    /// A 经验沉淀层 (嵌入 L4)
    pub a_layer: PrincipleLayer,
    /// M 方法论层 (嵌入 L3)
    pub m_layer: PrincipleLayer,
    /// O 操作原则层 (可自由改, 嵌入 L1-L2, 含 12 键 + 5 项不假装 + O-1..O-6)
    pub o_layer: PrincipleLayer,
}

/// 原则洋葱中任一切片
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleLayer {
    /// 层名 ("E" / "S" / "A" / "M" / "O")
    pub name: String,
    /// 层描述
    pub description: String,
    /// 是否硬编码 (true = 编译时不可变; false = 可动态 OTA)
    pub hardcoded: bool,
}

/// 权限洋葱 (6 切片: L0-L5, 承载原则)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOnion {
    /// L0 HA 核心 (不可变, 🛡️ 最后护栏)
    pub l0: PermissionLayer,
    /// L1 受控写
    pub l1: PermissionLayer,
    /// L2 重要操作
    pub l2: PermissionLayer,
    /// L3 关键操作
    pub l3: PermissionLayer,
    /// L4 核心升级
    pub l4: PermissionLayer,
    /// L5 核武器级
    pub l5: PermissionLayer,
}

/// 权限洋葱中任一切片
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionLayer {
    /// 层名 ("L0" .. "L5")
    pub name: String,
    /// 层描述
    pub description: String,
    /// 是否需要 HA 真实人类批准 (L0 永远需要)
    pub requires_ha: bool,
}

/// 人类权威 (HA) - 在权限洋葱核心 L0 (永远不变)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanAuthority {
    /// HA 部署模式 (按部署模式自适应: single / multi / offline)
    pub mode: HAMode,
    /// 注册的真实人类列表 (single=1 / multi=N)
    pub real_humans: Vec<RealHuman>,
    /// 冰冻期 (24h 内禁止 L0 变更)
    pub ice_frozen_until: Option<i64>,
}

/// HA 部署模式 (single / multi / offline)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HAMode {
    /// 单人模式: 1 个真实人类 + Windows Hello / FIDO2 / 主人密钥
    SingleHuman,
    /// 多人模式: N 个真实人类多人多签 (M-of-N)
    MultiHuman,
    /// 离线模式: 主人不在 = 安静模式 (仅允许 low / info)
    Offline,
}

/// 注册的真实人类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealHuman {
    /// 人类 ID
    pub id: String,
    /// 显示名
    pub name: String,
    /// 认证方式
    pub authentication: HAAuthentication,
    /// 生物特征数据 (抗胁迫: 生理指标)
    pub biometric_data: Option<BiometricData>,
}

/// HA 认证方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HAAuthentication {
    /// Windows Hello (生物特征)
    WindowsHello,
    /// FIDO2 安全密钥
    FIDO2,
    /// 多人多签
    MultiHuman,
    /// 离线签名
    OfflineSign,
}

/// 生物特征数据 (抗胁迫检测)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiometricData {
    /// 心率 (bpm, 抗胁迫检测)
    pub heart_rate: Option<f64>,
    /// 压力水平 (0.0 - 1.0, 胁迫检测)
    pub stress_level: Option<f64>,
}

/// 5 原则洋葱切片名（与 `PrincipleOnion.{e,s,a,m,o}_layer.name` 对应）
///
/// 用于 `VERDICT_KEYS_BY_PRINCIPLE` 的字符串标签，避开引入新 enum（3 不漂移：
/// 不新增公开类型，只新增数据）。
pub const PRINCIPLE_LAYER_E: &str = "E";
pub const PRINCIPLE_LAYER_S: &str = "S";
pub const PRINCIPLE_LAYER_A: &str = "A";
pub const PRINCIPLE_LAYER_M: &str = "M";
pub const PRINCIPLE_LAYER_O: &str = "O";

/// 13 键 → 5 原则洋葱映射（v2 工程重构，2026-08-27）。
///
/// v1 的 13 键没有显式的原则分组，group_id() 只返回 1-7 的 "PHL-01..07" 编号。
/// v2 把 13 键挂到 5 原则洋葱（E 存在 / S 价值 / A 经验 / M 方法论 / O 操作），
/// **作为哲学标准的"判别词汇表"**——不作为 runtime 强制机制
/// （runtime 强制 = external hook 闸 + 未来场景 D 长程 AI 判断，per `philosophy::RUNTIME_ENFORCED = false`）。
///
/// 映射原则（按 key 语义，非机械分组）:
/// - **E 存在层**: NotClone / NotPerfect / NotUuid / NotUnobservable / NotSelfRelationless
///   (本体论可观测性、身份连续性、不假装克隆/完美/唯一 → 5 键)
/// - **S 价值层**: NotSafe / NotUnoptimizable
///   (安全 + 最优价值判断 → 2 键)
/// - **A 经验层**: NotUndo
///   (过去经验不可撤销 → 1 键)
/// - **M 方法论层**: NotProof / SpecIsNotProof / CounterexampleIsNotBug / ProverIsNotTruth / NotUnscientific
///   (证明 / 规格 / 反例 / 证明者 / 科学方法 → 5 键)
/// - **O 操作层**: (v2 当前无键显式归此层；预留扩展位)
///   (含义：日常操作层。当前 13 键在 E/S/A/M 各层已覆盖 O 的语义)
///
/// 总数: 5 + 2 + 1 + 5 = 13 ✓ (与 ALL_THIRTEEN_KEYS.len() 一致)
/// 0 触碰 ALL_THIRTEEN_KEYS / group_id() / THIRTEEN_KEYS_HARDCODE——纯新增映射表。
pub const VERDICT_KEYS_BY_PRINCIPLE: &[(&str, &[PhilosophyKey])] = &[
    (
        PRINCIPLE_LAYER_E,
        &[
            PhilosophyKey::NotClone,
            PhilosophyKey::NotPerfect,
            PhilosophyKey::NotUuid,
            PhilosophyKey::NotUnobservable,
            PhilosophyKey::NotSelfRelationless,
        ],
    ),
    (
        PRINCIPLE_LAYER_S,
        &[
            PhilosophyKey::NotSafe,
            PhilosophyKey::NotUnoptimizable,
        ],
    ),
    (
        PRINCIPLE_LAYER_A,
        &[PhilosophyKey::NotUndo],
    ),
    (
        PRINCIPLE_LAYER_M,
        &[
            PhilosophyKey::NotProof,
            PhilosophyKey::SpecIsNotProof,
            PhilosophyKey::CounterexampleIsNotBug,
            PhilosophyKey::ProverIsNotTruth,
            PhilosophyKey::NotUnscientific,
        ],
    ),
    // O 操作层: 当前 13 键未显式归此层 (E/S/A/M 已覆盖 O 语义)
    // 扩展位: 未来如新增 O 层 键，加在下面。
];

/// 编译期断言 — VERDICT_KEYS_BY_PRINCIPLE 总键数 = ALL_THIRTEEN_KEYS.len()（0 漂移）。
///
/// 防止新增/删 13 键后忘记同步原则映射。Rust const 不允许格式化宏，所以用长度求和做断言：
/// sum(len of each slice) 必须 = 13。
const _VERDICT_KEYS_BY_PRINCIPLE_LEN_MATCHES: () = {
    let mut total: usize = 0;
    let mut idx = 0;
    while idx < VERDICT_KEYS_BY_PRINCIPLE.len() {
        total += VERDICT_KEYS_BY_PRINCIPLE[idx].1.len();
        idx += 1;
    }
    if total != ALL_THIRTEEN_KEYS.len() {
        panic!("13 键映射完整性: 映射表总数与 ALL_THIRTEEN_KEYS.len() 不匹配");
    }
};

/// 按原则名取 13 键子集。未知原则名返回空切片。
///
/// 用于 governance hook 在 deny 时附加原则归属 reason（v2 治理事实落地点）。
pub fn verdict_keys_for_principle(principle: &str) -> &'static [PhilosophyKey] {
    for (name, keys) in VERDICT_KEYS_BY_PRINCIPLE {
        if *name == principle {
            return keys;
        }
    }
    &[]
}

// ============================================
