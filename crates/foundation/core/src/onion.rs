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
    /// P-arch (2026-08-27): M-of-N 多签策略 (仅 `MultiHuman` 模式生效, single/offline 留 `None`).
    /// v2.0 是**0 装占位** (signature 是 hex string, 不是真 crypto 校验);
    /// v2.1 接 `apeireth-credentials` 的 Ed25519/实际多签实现.
    /// 详见 `v2-unabsorbed-features.md` §B4 (sovereignty 多签) + `scene-d-v2-plan.md`.
    pub multi_sign: Option<MultiSignPolicy>,
}

impl HumanAuthority {
    /// 验证多签集合: 检查 `signatures` 中**有效且来自不同 real_human** 的数量 ≥ `multi_sign.required`.
    ///
    /// **0 装 PASS**:
    /// - `mode != MultiHuman` → 必须有 `multi_sign = None`, 否则 `InvalidHA`
    /// - `mode == MultiHuman` 但 `multi_sign = None` → 用 v1 单人默认值 (要求 1 个)
    /// - signature 格式 = `hex("{signer_id}:{digest}")`; 真 crypto 校验 v2.1
    /// - 同一 signer_id 多次签名只算一次 (防止"用同一个人凑数")
    pub fn verify_multisig(&self, signatures: &[String]) -> MultiSignResult {
        match self.mode {
            HAMode::SingleHuman => {
                if self.multi_sign.is_some() {
                    return MultiSignResult::InvalidHA(
                        "single human mode must not have multi_sign policy".into(),
                    );
                }
                // 1 个有效签名 = 通过
                if signatures.iter().any(|s| Self::is_well_formed_sig(s)) {
                    MultiSignResult::Accepted
                } else {
                    MultiSignResult::Insufficient {
                        required: 1,
                        received: 0,
                    }
                }
            }
            HAMode::Offline => MultiSignResult::DeniedOffline,
            HAMode::MultiHuman => {
                let required = self.multi_sign.as_ref().map(|m| m.required).unwrap_or(1);
                let total = self.real_humans.len();
                if total == 0 {
                    return MultiSignResult::InvalidHA(
                        "MultiHuman mode requires at least one real_human".into(),
                    );
                }
                if required == 0 || required > total as u8 {
                    return MultiSignResult::InvalidHA(format!(
                        "multi_sign.required={} out of range [1, total={}]",
                        required, total
                    ));
                }
                // 收集不重复的有效 signer_id
                let mut distinct_signers: Vec<String> = Vec::new();
                for sig in signatures {
                    if !Self::is_well_formed_sig(sig) {
                        continue;
                    }
                    if let Some(signer_id) = Self::extract_signer_id(sig) {
                        if !distinct_signers.iter().any(|s| s == &signer_id)
                            && self.real_humans.iter().any(|h| h.id == signer_id)
                        {
                            distinct_signers.push(signer_id);
                        }
                    }
                }
                let received = distinct_signers.len() as u8;
                if received >= required {
                    MultiSignResult::Accepted
                } else {
                    MultiSignResult::Insufficient { required, received }
                }
            }
        }
    }

    /// 极简 signature 格式检查: 非空 + 至少含一个 `:` (signer_id:digest).
    fn is_well_formed_sig(sig: &str) -> bool {
        !sig.trim().is_empty() && sig.contains(':')
    }

    /// 从 `signer_id:digest` 形式提取 signer_id (v2.0 占位; v2.1 替换为真签名解析).
    fn extract_signer_id(sig: &str) -> Option<String> {
        sig.split(':').next().map(|s| s.trim().to_string())
    }
}

/// M-of-N 多签策略 (P-arch, 2026-08-27).
///
/// `required` 个不同 `real_human.id` 提供的有效签名即可通过 L0 HA.
/// 任何 L0 变更 (核心升降级 / 自我禁用解除 / 长期记忆迁移) 都需此验证.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultiSignPolicy {
    /// 需要的有效签名数 (1 ≤ required ≤ total).
    pub required: u8,
    /// v2.0 固定为 None; v2.1 用 `keyring` 取公钥列表 (`apeireth-credentials` 接线时填).
    pub public_keys: Option<Vec<String>>,
}

/// 多签验证结果.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiSignResult {
    /// 签名数量足够, 通过.
    Accepted,
    /// 签名不足: `received < required`.
    Insufficient { required: u8, received: u8 },
    /// 部署模式拒绝 (offline).
    DeniedOffline,
    /// 配置错误 (如 single + multi_sign, 或 required 越界).
    InvalidHA(String),
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
        &[PhilosophyKey::NotSafe, PhilosophyKey::NotUnoptimizable],
    ),
    (PRINCIPLE_LAYER_A, &[PhilosophyKey::NotUndo]),
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

#[cfg(test)]
mod ha_multisign_tests {
    use super::*;

    fn real_human(id: &str) -> RealHuman {
        RealHuman {
            id: id.to_string(),
            name: id.to_string(),
            authentication: HAAuthentication::MultiHuman,
            biometric_data: None,
        }
    }

    fn sig(signer: &str) -> String {
        format!("{signer}:fake_digest")
    }

    #[test]
    fn single_human_accepts_one_valid_signature() {
        let ha = HumanAuthority {
            mode: HAMode::SingleHuman,
            real_humans: vec![real_human("alice")],
            ice_frozen_until: None,
            multi_sign: None,
        };
        assert_eq!(
            ha.verify_multisig(&[sig("alice")]),
            MultiSignResult::Accepted
        );
    }

    #[test]
    fn single_human_rejects_zero_signatures() {
        let ha = HumanAuthority {
            mode: HAMode::SingleHuman,
            real_humans: vec![real_human("alice")],
            ice_frozen_until: None,
            multi_sign: None,
        };
        assert_eq!(
            ha.verify_multisig(&[]),
            MultiSignResult::Insufficient {
                required: 1,
                received: 0
            }
        );
    }

    #[test]
    fn single_human_rejects_multi_sign_policy() {
        let ha = HumanAuthority {
            mode: HAMode::SingleHuman,
            real_humans: vec![real_human("alice")],
            ice_frozen_until: None,
            multi_sign: Some(MultiSignPolicy {
                required: 2,
                public_keys: None,
            }),
        };
        assert!(matches!(
            ha.verify_multisig(&[sig("alice")]),
            MultiSignResult::InvalidHA(_)
        ));
    }

    #[test]
    fn offline_mode_denies_always() {
        let ha = HumanAuthority {
            mode: HAMode::Offline,
            real_humans: vec![],
            ice_frozen_until: None,
            multi_sign: None,
        };
        assert_eq!(
            ha.verify_multisig(&[sig("alice")]),
            MultiSignResult::DeniedOffline
        );
    }

    #[test]
    fn multi_human_requires_correct_count() {
        let ha = HumanAuthority {
            mode: HAMode::MultiHuman,
            real_humans: vec![real_human("a"), real_human("b"), real_human("c")],
            ice_frozen_until: None,
            multi_sign: Some(MultiSignPolicy {
                required: 2,
                public_keys: None,
            }),
        };
        // 0 个签名
        assert_eq!(
            ha.verify_multisig(&[]),
            MultiSignResult::Insufficient {
                required: 2,
                received: 0
            }
        );
        // 1 个签名，不足
        assert_eq!(
            ha.verify_multisig(&[sig("a")]),
            MultiSignResult::Insufficient {
                required: 2,
                received: 1
            }
        );
        // 2 个签名，刚好
        assert_eq!(
            ha.verify_multisig(&[sig("a"), sig("b")]),
            MultiSignResult::Accepted
        );
        // 3 个签名，通过
        assert_eq!(
            ha.verify_multisig(&[sig("a"), sig("b"), sig("c")]),
            MultiSignResult::Accepted
        );
    }

    #[test]
    fn multi_human_dedups_same_signer() {
        // 同一人多次签名只算一次
        let ha = HumanAuthority {
            mode: HAMode::MultiHuman,
            real_humans: vec![real_human("a"), real_human("b")],
            ice_frozen_until: None,
            multi_sign: Some(MultiSignPolicy {
                required: 2,
                public_keys: None,
            }),
        };
        // 3 个签名但都是 alice，只算 1 个不同 signer
        assert_eq!(
            ha.verify_multisig(&[sig("a"), sig("a"), sig("a")]),
            MultiSignResult::Insufficient {
                required: 2,
                received: 1
            }
        );
    }

    #[test]
    fn multi_human_rejects_unknown_signer() {
        let ha = HumanAuthority {
            mode: HAMode::MultiHuman,
            real_humans: vec![real_human("alice"), real_human("bob")],
            ice_frozen_until: None,
            multi_sign: Some(MultiSignPolicy {
                required: 2,
                public_keys: None,
            }),
        };
        // charlie 不在 real_humans 里
        assert_eq!(
            ha.verify_multisig(&[sig("alice"), sig("charlie")]),
            MultiSignResult::Insufficient {
                required: 2,
                received: 1
            }
        );
    }

    #[test]
    fn multi_human_rejects_malformed_signatures() {
        let ha = HumanAuthority {
            mode: HAMode::MultiHuman,
            real_humans: vec![real_human("a"), real_human("b")],
            ice_frozen_until: None,
            multi_sign: Some(MultiSignPolicy {
                required: 2,
                public_keys: None,
            }),
        };
        // 空字符串、没冒号 → 跳过
        assert_eq!(
            ha.verify_multisig(&["".to_string(), "no_colon".to_string(), sig("a")]),
            MultiSignResult::Insufficient {
                required: 2,
                received: 1
            }
        );
    }

    #[test]
    fn multi_human_validates_required_range() {
        // required > total → InvalidHA
        let ha = HumanAuthority {
            mode: HAMode::MultiHuman,
            real_humans: vec![real_human("a"), real_human("b")],
            ice_frozen_until: None,
            multi_sign: Some(MultiSignPolicy {
                required: 5,
                public_keys: None,
            }),
        };
        assert!(matches!(
            ha.verify_multisig(&[sig("a")]),
            MultiSignResult::InvalidHA(_)
        ));
    }

    #[test]
    fn multi_human_defaults_to_required_1_when_no_policy() {
        // mode=MultiHuman 但 multi_sign=None → v1 兼容：1 个有效签名即通过
        let ha = HumanAuthority {
            mode: HAMode::MultiHuman,
            real_humans: vec![real_human("a"), real_human("b")],
            ice_frozen_until: None,
            multi_sign: None,
        };
        assert_eq!(ha.verify_multisig(&[sig("a")]), MultiSignResult::Accepted);
        assert_eq!(
            ha.verify_multisig(&[]),
            MultiSignResult::Insufficient {
                required: 1,
                received: 0
            }
        );
    }
}

// ============================================
