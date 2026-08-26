//! `apeireth-core::onion` — 双洋葱统一体 (PrincipleOnion + PermissionOnion + HumanAuthority)
//!
//! 拆自 `lib.rs` line 33-147 (R131 架构债清理). 0 触碰公开签名 — `use apeireth_core::PrincipleOnion` 等仍可用.
//!
//! 包含: typedef 本段所有 `pub struct` / `pub enum` / `pub trait` / `pub const`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

// ============================================
