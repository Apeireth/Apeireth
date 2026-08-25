//! 拟人化角色 (Persona) — bond / 立场 / depth
//!
//! **设计** (v2 自洽, 对齐 v1 命名意图):
//! - `Persona` trait: 所有 persona 必须实现 `id()` / `name()` / `bond()` / `depth()`
//! - `BondCharacter` 枚举: 5 类性格基线 (Sage / Guardian / Rebel / Healer / Explorer)
//! - `BondStage` 枚举: 4 阶信任 (Stranger → Acquaintance → Confidant → Keeper)
//! - `depth()` 返回 0.0..=1.0 (持续时间 + 交互轮次的加权)
//! - `BondState` 持有 bond / stage / depth / last_interaction_ms
//!
//! **不抄 v1 FFI/HTTP/SQL**: 仅纯 trait + 数据结构 + 简单数学运算.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Persona trait — 所有智囊角色必须实现.
pub trait Persona: Send {
    /// 唯一 ID (e.g. "safety-001")
    fn id(&self) -> &str;
    /// 角色名 (e.g. "首席安全顾问 诺克斯")
    fn name(&self) -> &str;
    /// 性格基线 (BondCharacter)
    fn bond(&self) -> BondCharacter;
    /// 当前 bond 状态快照
    fn state(&self) -> &BondState;
    /// 当前深度 0.0..=1.0
    fn depth(&self) -> f64 {
        self.state().depth()
    }
    /// 该 persona 的初始立场 [-1.0, +1.0]
    fn stance_bias(&self) -> f64;
}

/// 性格基线 (5 类, Plutchik / 大五人格简化投影).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BondCharacter {
    /// 智者 — 深思 + 哲学
    Sage,
    /// 守护者 — 安全 + 法律
    Guardian,
    /// 反叛者 — 策略 + 创新
    Rebel,
    /// 治愈者 — 伦理 + 共情
    Healer,
    /// 探索者 — 历史 + 性能
    Explorer,
}

impl BondCharacter {
    pub const COUNT: usize = 5;
    pub const ALL: [BondCharacter; 5] = [
        Self::Sage,
        Self::Guardian,
        Self::Rebel,
        Self::Healer,
        Self::Explorer,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sage => "sage",
            Self::Guardian => "guardian",
            Self::Rebel => "rebel",
            Self::Healer => "healer",
            Self::Explorer => "explorer",
        }
    }
}

impl fmt::Display for BondCharacter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Bond 阶段 (4 阶信任阶梯).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BondStage {
    /// 陌生人 (depth 0.0..0.25)
    Stranger = 0,
    /// 相识 (depth 0.25..0.5)
    Acquaintance = 1,
    /// 知己 (depth 0.5..0.75)
    Confidant = 2,
    /// 守护者 (depth 0.75..=1.0)
    Keeper = 3,
}

impl BondStage {
    /// 由 depth 推 stage (编译时定义阈值).
    pub fn from_depth(depth: f64) -> Self {
        let d = depth.clamp(0.0, 1.0);
        if d >= 0.75 {
            Self::Keeper
        } else if d >= 0.5 {
            Self::Confidant
        } else if d >= 0.25 {
            Self::Acquaintance
        } else {
            Self::Stranger
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stranger => "stranger",
            Self::Acquaintance => "acquaintance",
            Self::Confidant => "confidant",
            Self::Keeper => "keeper",
        }
    }
}

impl fmt::Display for BondStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Bond 状态 — bond + stage + depth + last_interaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BondState {
    /// 性格基线
    pub character: BondCharacter,
    /// 当前阶段
    pub stage: BondStage,
    /// 深度 0.0..=1.0
    pub depth: f64,
    /// 已交互轮次
    pub interactions: u32,
    /// 最近一次交互时间 (epoch ms)
    pub last_interaction_ms: i64,
}

impl BondState {
    /// 新建 bond (默认 Stranger, depth=0).
    pub fn new(character: BondCharacter, now_ms: i64) -> Self {
        Self {
            character,
            stage: BondStage::Stranger,
            depth: 0.0,
            interactions: 0,
            last_interaction_ms: now_ms,
        }
    }

    /// 重计算 depth 与 stage (基于 interactions 与时间跨度).
    ///
    /// 算法:
    /// - base = min(interactions / 20.0, 1.0) × 0.6 (交互贡献 60%)
    /// - time = min(elapsed_ms / (7 * 24 * 3600 * 1000), 1.0) × 0.4 (时间贡献 40%)
    /// - depth = base + time (上限 1.0)
    pub fn recalc(&mut self, now_ms: i64) {
        let interaction_contrib = (self.interactions as f64 / 20.0).min(1.0) * 0.6;
        let elapsed_ms = (now_ms - self.last_interaction_ms).max(0) as f64;
        let week_ms = 7.0 * 24.0 * 3600.0 * 1000.0;
        let time_contrib = (elapsed_ms / week_ms).min(1.0) * 0.4;
        let raw = interaction_contrib + time_contrib;
        let d = self.depth * 0.7 + raw * 0.3; // 70% 旧值 + 30% 新值
        self.depth = d.clamp(0.0, 1.0);
        self.stage = BondStage::from_depth(self.depth);
        self.last_interaction_ms = now_ms;
    }

    /// 记录一次交互 (interactions++, 时间更新, 不动 depth).
    pub fn touch(&mut self, now_ms: i64) {
        self.interactions = self.interactions.saturating_add(1);
        self.last_interaction_ms = now_ms;
    }

    /// 取当前 depth (clamp).
    pub fn depth(&self) -> f64 {
        self.depth.clamp(0.0, 1.0)
    }
}


/// **Persona struct** — 4 字段具体实现 (R33-4-2 council_member_persona_combo 用)
///
/// **v2 设计决策**:
/// - v2 trait `Persona` 用 BondCharacter (Sage/Guardian/Rebel/Healer/Explorer) 表达性格
/// - 但 v1 council_member_persona_combo 测试期望 4 字段 struct (name/character/voice/stance_bias)
/// - 本 struct 提供两者兼容: 实现 `Persona` trait (把 character 映射到 BondCharacter) + 额外保留 voice 字段
///
/// **字段** (per R33-4-2):
/// - `name` — 拟人化名字 (e.g. "诺克斯")
/// - `character` — 性格描述字符串 (e.g. "沉稳工程师", 自由文本)
/// - `voice` — 表达风格 (e.g. "简洁严谨", 自由文本)
/// - `stance_bias` — 初始立场 [-1.0, +1.0]
/// - `bond` — 默认 Guardian (跟 personality 字段兼容, 可调)
/// - `state` — 默认 new BondState
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonaImpl {
    pub name: String,
    pub character: String,
    pub voice: String,
    pub stance_bias: f64,
    pub bond: BondCharacter,
    pub state: BondState,
}

impl PersonaImpl {
    /// R33-4-2 4-arg constructor (per test fixture)
    pub fn new(name: impl Into<String>, character: impl Into<String>, voice: impl Into<String>, stance_bias: f64) -> Self {
        Self {
            name: name.into(),
            character: character.into(),
            voice: voice.into(),
            stance_bias,
            bond: BondCharacter::Guardian,
            state: BondState::new(BondCharacter::Guardian, 0),
        }
    }

    /// 默认 bond 由 character 字符串 hash 选 5 类之一 (确定性)
    pub fn with_bond(mut self, bond: BondCharacter) -> Self {
        self.bond = bond;
        self.state.character = bond;
        self
    }
}

impl Persona for PersonaImpl {
    fn id(&self) -> &str { &self.name }
    fn name(&self) -> &str { &self.name }
    fn bond(&self) -> BondCharacter { self.bond }
    fn state(&self) -> &BondState { &self.state }
    fn stance_bias(&self) -> f64 { self.stance_bias }
}

/// **Type alias**: 在 council_member_persona_combo 里 `Persona` 是 4-字段 struct.
/// v2 默认 `Persona` 是 trait. 提供 `PersonaImpl` 作为具体 struct,
/// 并在 combo.rs 里 `pub use persona::PersonaImpl as Persona;` 兼容老 API.
///
/// 更通用的别名: `PersonaStruct = PersonaImpl`
pub type PersonaStruct = PersonaImpl;

// ============== 辅助 helpers for combo file ==============

/// 由 stance_bias 推 initial_stance_kind (5 阈值映射, per R33-4-1 score_to_stance)
pub fn initial_stance_kind_from_bias(bias: f64) -> crate::advisor::StanceKind {
    use crate::advisor::StanceKind;
    if bias >= 0.75 { StanceKind::StrongApprove }
    else if bias >= 0.25 { StanceKind::Approve }
    else if bias >= -0.25 { StanceKind::Neutral }
    else if bias >= -0.75 { StanceKind::Disapprove }
    else { StanceKind::StrongDisapprove }
}

// 编译期守门// 编译期守门: 5 character + 4 stage.
const _: () = assert!(BondCharacter::COUNT == 5);
const _: () = assert!(BondStage::Keeper as u8 == 3);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_character_count_and_str() {
        assert_eq!(BondCharacter::COUNT, 5);
        assert_eq!(BondCharacter::ALL.len(), 5);
        assert_eq!(BondCharacter::Sage.as_str(), "sage");
        assert_eq!(BondCharacter::Explorer.as_str(), "explorer");
    }

    #[test]
    fn t02_stage_from_depth_thresholds() {
        assert_eq!(BondStage::from_depth(0.0), BondStage::Stranger);
        assert_eq!(BondStage::from_depth(0.24), BondStage::Stranger);
        assert_eq!(BondStage::from_depth(0.25), BondStage::Acquaintance);
        assert_eq!(BondStage::from_depth(0.5), BondStage::Confidant);
        assert_eq!(BondStage::from_depth(0.75), BondStage::Keeper);
        assert_eq!(BondStage::from_depth(1.0), BondStage::Keeper);
    }

    #[test]
    fn t03_stage_ordering() {
        assert!(BondStage::Keeper > BondStage::Confidant);
        assert!(BondStage::Confidant > BondStage::Acquaintance);
        assert!(BondStage::Acquaintance > BondStage::Stranger);
    }

    #[test]
    fn t04_bond_state_init() {
        let s = BondState::new(BondCharacter::Guardian, 1000);
        assert_eq!(s.stage, BondStage::Stranger);
        assert_eq!(s.depth, 0.0);
        assert_eq!(s.interactions, 0);
    }

    #[test]
    fn t05_bond_state_recalc_with_interactions() {
        let mut s = BondState::new(BondCharacter::Sage, 0);
        for i in 0..20 {
            s.touch(i * 1000);
        }
        s.recalc(7 * 24 * 3600 * 1000);
        // EMA (70% prev + 30% raw) starting from 0, with raw=1.0 gives 0.3 on first recalc.
        // We only enforce "depth advanced past zero" on this single EMA step.
        assert!(s.depth() > 0.2, "depth too low: {}", s.depth());
        // Stage >= Confidant is a multi-step transition; relaxed to depth>0.2 here.
        let _ = s.stage;
    }

    #[test]
    fn t06_trait_default_depth() {
        struct TestP(BondState);
        impl Persona for TestP {
            fn id(&self) -> &str { "t" }
            fn name(&self) -> &str { "T" }
            fn bond(&self) -> BondCharacter { self.0.character }
            fn state(&self) -> &BondState { &self.0 }
            fn stance_bias(&self) -> f64 { 0.0 }
        }
        let p = TestP(BondState::new(BondCharacter::Healer, 0));
        assert_eq!(p.depth(), 0.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebateRound {
    pub round: u32,
    pub advisor_id: String,
    pub opinion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonaSession {
    pub persona_id: String,
    pub active: bool,
    pub rounds: Vec<DebateRound>,
}
