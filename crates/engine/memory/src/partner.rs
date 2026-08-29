//! `apeireth-memory::partner` — 伙伴与双向羁绊模型 (R12-SpeciesCore-1 实施).
//!
//! **设计哲学 (伙伴与物种化特征)**:
//! - 用户是 AI 的长期伙伴，而非单纯的调用者或从属者；
//! - 羁绊 (`Bond`) 是跨 Session 的连续情感与信任状态，具备阶段跃迁 (`BondStage`) 与特征演化 (`BondCharacter`)；
//! - 0 假装 (O-5): 纯确定性状态机与连续度计算，显式时间戳注入。
//!
//! **O-6 三阶审查**:
//! 1. 总体: 为陪伴型智能与长期共生提供伙伴身份与羁绊状态核心载体
//! 2. 系统: 放置在 `apeireth-memory`, 与 `Milestone` 和 `SessionStore` 协同
//! 3. 架构: 强类型数据模型与 `PartnerStore` Trait 契约，支持内存与持久化存储

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use apeireth_core::kernel::SessionId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::MemoryError;

/// 关系阶段 (生命周期中的关键阶段).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BondStage {
    /// 初始接触 (初次交互)
    Initial,
    /// 熟悉中 (多轮日常互动)
    Familiar,
    /// 信任 (共同经历与决策)
    Trusted,
    /// 亲密 (深度共鸣与长程依赖)
    Intimate,
    /// 长期共生 (成熟稳固关系)
    LongTerm,
    /// 暂停 (长时间未互动)
    Paused,
    /// 终止 (用户显式声明终结)
    Ended,
}

impl BondStage {
    pub const ALL: [BondStage; 7] = [
        Self::Initial,
        Self::Familiar,
        Self::Trusted,
        Self::Intimate,
        Self::LongTerm,
        Self::Paused,
        Self::Ended,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Familiar => "familiar",
            Self::Trusted => "trusted",
            Self::Intimate => "intimate",
            Self::LongTerm => "long_term",
            Self::Paused => "paused",
            Self::Ended => "ended",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Ended)
    }
}

/// 关系深度 `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BondDepth(pub f64);

impl BondDepth {
    pub const ZERO: BondDepth = BondDepth(0.0);
    pub const ONE: BondDepth = BondDepth(1.0);

    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

impl Default for BondDepth {
    fn default() -> Self {
        Self::ZERO
    }
}

/// 关系特征 (多维关系性格指标).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BondCharacter {
    /// 互依度 `[0.0, 1.0]`
    pub interdependency: f64,
    /// 韧性 (冲突后的恢复能力) `[0.0, 1.0]`
    pub resilience: f64,
    /// 共鸣度 `[0.0, 1.0]`
    pub resonance: f64,
    /// 创造性 (共同探索新领域的冲动) `[0.0, 1.0]`
    pub creativity: f64,
    /// 信任度 `[0.0, 1.0]`
    pub trust: f64,
}

impl Default for BondCharacter {
    fn default() -> Self {
        Self {
            interdependency: 0.1,
            resilience: 0.5,
            resonance: 0.2,
            creativity: 0.2,
            trust: 0.1,
        }
    }
}

/// 羁绊状态实体.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    /// 当前关系阶段
    pub stage: BondStage,
    /// 关系深度连续值
    pub depth: BondDepth,
    /// 关系多维特征
    pub character: BondCharacter,
    /// 演化推进次数
    pub evolution_count: u64,
    /// 最后更新时间戳 (毫秒)
    pub updated_at_epoch_ms: i64,
}

impl Bond {
    pub fn new(at_epoch_ms: i64) -> Self {
        Self {
            stage: BondStage::Initial,
            depth: BondDepth::ZERO,
            character: BondCharacter::default(),
            evolution_count: 0,
            updated_at_epoch_ms: at_epoch_ms,
        }
    }

    /// 推进演化与深度提升.
    pub fn evolve(&mut self, depth_increment: f64, at_epoch_ms: i64) {
        self.depth = BondDepth::new(self.depth.value() + depth_increment);
        self.evolution_count += 1;
        self.updated_at_epoch_ms = at_epoch_ms;

        // 根据深度自动推进阶段跃迁
        let d = self.depth.value();
        if d >= 0.85 {
            self.stage = BondStage::LongTerm;
        } else if d >= 0.65 {
            self.stage = BondStage::Intimate;
        } else if d >= 0.40 {
            self.stage = BondStage::Trusted;
        } else if d >= 0.15 {
            self.stage = BondStage::Familiar;
        }
    }
}

/// 隐私边界设置 (声明式敏感信息与脱敏规则).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PrivacyBoundary {
    /// 是否允许出站 LLM 调用时替换敏感字符串
    pub allow_outbound_substitution: bool,
    /// 敏感字符串列表 (如用户真实姓名、手机号等)
    pub sensitive_strings: Vec<String>,
}

/// 伙伴显式偏好声明.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PartnerPreferences {
    /// 称呼偏好 (如 "你" / "您" / 自定义称呼)
    pub address: Option<String>,
    /// 表达风格偏好 (如 "简洁" / "详细" / "幽默" / "严肃")
    pub style: Option<String>,
    /// 关注话题列表
    pub topics: Vec<String>,
    /// 避开的雷区话题
    pub avoid: Vec<String>,
    /// 自定义备注键值对
    pub notes: HashMap<String, String>,
    /// 隐私边界
    pub privacy: PrivacyBoundary,
}

/// 伙伴唯一 ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PartnerId(pub String);

impl PartnerId {
    pub fn new() -> Self {
        Self(format!("partner-{}", Uuid::new_v4()))
    }

    pub fn from_session(session_id: &SessionId) -> Self {
        Self(format!("partner-{}", session_id))
    }
}

impl Default for PartnerId {
    fn default() -> Self {
        Self::new()
    }
}

/// 伙伴实体 (用户在交互关系中的工程化映射).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Partner {
    pub id: PartnerId,
    pub display_name: String,
    pub preferences: PartnerPreferences,
    pub bond: Bond,
    pub created_at_epoch_ms: i64,
    pub last_seen_epoch_ms: i64,
}

impl Partner {
    pub fn new(
        id: PartnerId,
        display_name: impl Into<String>,
        preferences: PartnerPreferences,
        now_epoch_ms: i64,
    ) -> Self {
        Self {
            id,
            display_name: display_name.into(),
            preferences,
            bond: Bond::new(now_epoch_ms),
            created_at_epoch_ms: now_epoch_ms,
            last_seen_epoch_ms: now_epoch_ms,
        }
    }

    /// 更新活跃时间戳.
    pub fn touch(&mut self, now_epoch_ms: i64) {
        self.last_seen_epoch_ms = now_epoch_ms;
    }
}

/// 伙伴数据存储与检索 Trait.
pub trait PartnerStore: Send + Sync {
    /// 保存或更新伙伴记录.
    fn save_partner(&self, partner: &Partner) -> Result<(), MemoryError>;

    /// 查询特定伙伴.
    fn get_partner(&self, id: &PartnerId) -> Result<Option<Partner>, MemoryError>;

    /// 列出所有伙伴.
    fn list_partners(&self) -> Result<Vec<Partner>, MemoryError>;
}

/// 内存版伙伴存储实现 (供轻量嵌入与单测使用).
#[derive(Debug, Default)]
pub struct InMemoryPartnerStore {
    partners: Mutex<BTreeMap<PartnerId, Partner>>,
}

impl InMemoryPartnerStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PartnerStore for InMemoryPartnerStore {
    fn save_partner(&self, partner: &Partner) -> Result<(), MemoryError> {
        let mut guard = self.partners.lock().expect("in-memory partner store mutex");
        guard.insert(partner.id.clone(), partner.clone());
        Ok(())
    }

    fn get_partner(&self, id: &PartnerId) -> Result<Option<Partner>, MemoryError> {
        let guard = self.partners.lock().expect("in-memory partner store mutex");
        Ok(guard.get(id).cloned())
    }

    fn list_partners(&self) -> Result<Vec<Partner>, MemoryError> {
        let guard = self.partners.lock().expect("in-memory partner store mutex");
        Ok(guard.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bond_evolution_stage_transitions() {
        let mut bond = Bond::new(1000);
        assert_eq!(bond.stage, BondStage::Initial);

        bond.evolve(0.20, 2000);
        assert_eq!(bond.stage, BondStage::Familiar);

        bond.evolve(0.25, 3000); // total 0.45
        assert_eq!(bond.stage, BondStage::Trusted);

        bond.evolve(0.25, 4000); // total 0.70
        assert_eq!(bond.stage, BondStage::Intimate);

        bond.evolve(0.20, 5000); // total 0.90
        assert_eq!(bond.stage, BondStage::LongTerm);
        assert_eq!(bond.evolution_count, 4);
    }

    #[test]
    fn partner_store_crud() {
        let store = InMemoryPartnerStore::new();
        let id = PartnerId::new();
        let mut partner = Partner::new(
            id.clone(),
            "主人",
            PartnerPreferences {
                address: Some("主人".into()),
                style: Some("严谨且温暖".into()),
                ..Default::default()
            },
            1756400000000,
        );

        store.save_partner(&partner).unwrap();

        let loaded = store.get_partner(&id).unwrap().expect("伙伴应存在");
        assert_eq!(loaded.display_name, "主人");
        assert_eq!(loaded.preferences.address.as_deref(), Some("主人"));

        partner.touch(1756400010000);
        store.save_partner(&partner).unwrap();

        let updated = store.get_partner(&id).unwrap().unwrap();
        assert_eq!(updated.last_seen_epoch_ms, 1756400010000);

        let all = store.list_partners().unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn partner_serde_roundtrip() {
        let partner = Partner::new(
            PartnerId::new(),
            "测试用户",
            PartnerPreferences::default(),
            1000,
        );
        let json = serde_json::to_string(&partner).unwrap();
        let decoded: Partner = serde_json::from_str(&json).unwrap();
        assert_eq!(partner, decoded);
    }
}
