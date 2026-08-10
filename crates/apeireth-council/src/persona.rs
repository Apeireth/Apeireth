//! 拟人化 — 独立 session + persona + 立场 + 可辩论 3 轮
//!
//! **设计**:
//! - 每个 persona 拥有独立 [`PersonaSession`] (独立 session_id)
//! - persona 持有 `stance_bias` (初始立场偏向, 用于辩论启动)
//! - 辩论 3 轮 (`MAX_PERSONA_DEBATE_ROUNDS = 3`)
//! - 每轮 persona 可以参考 prior opinions 并产出 [`DebateRound`] 产出

use crate::advisor::{AdvisorOpinion, DeliberationOutcome, Stance, StanceKind};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Persona — 拟人化角色。
///
/// **字段**:
/// - `name` — 角色名称 (e.g. "首席安全顾问 诺克斯")
/// - `character` — 性格描述
/// - `voice` — 表达风格
/// - `stance_bias` — 初始立场偏向 (-1.0 强反对 ~ +1.0 强赞成)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Persona {
    /// 角色名称
    pub name: String,
    /// 性格描述
    pub character: String,
    /// 表达风格
    pub voice: String,
    /// 初始立场偏向 (-1.0 ~ +1.0)
    pub stance_bias: f64,
}

impl Persona {
    /// 便利构造。
    pub fn new(
        name: impl Into<String>,
        character: impl Into<String>,
        voice: impl Into<String>,
        stance_bias: f64,
    ) -> Self {
        Self {
            name: name.into(),
            character: character.into(),
            voice: voice.into(),
            stance_bias: stance_bias.clamp(-1.0, 1.0),
        }
    }

    /// 从 `stance_bias` 推出初始 [`StanceKind`].
    pub fn initial_stance_kind(&self) -> StanceKind {
        let b = self.stance_bias;
        if b >= 0.6 {
            StanceKind::StrongApprove
        } else if b >= 0.2 {
            StanceKind::Approve
        } else if b >= -0.2 {
            StanceKind::Neutral
        } else if b >= -0.6 {
            StanceKind::Disapprove
        } else {
            StanceKind::StrongDisapprove
        }
    }
}

impl fmt::Display for Persona {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.character)
    }
}

/// 辩论一轮的产出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebateRound {
    /// 轮次 (0-based)
    pub round: u8,
    /// 该轮的产出意见
    pub outcome: DeliberationOutcome,
    /// persona 自由陈述 (拟人化语气)
    pub speech: String,
}

impl DebateRound {
    /// 该轮的立场。
    pub fn stance(&self) -> &Stance {
        &self.outcome.opinion.stance
    }
}

/// Persona 会话 — 独立 session_id + 立场演化 + 辩论轮次记录。
#[derive(Debug, Clone)]
pub struct PersonaSession {
    /// 独立 session ID (与全局 Council 隔离)
    pub session_id: String,
    /// 关联 persona
    pub persona: Persona,
    /// 辩论轮次记录
    pub rounds: Vec<DebateRound>,
    /// 当前轮次 (0-based)
    pub current_round: u8,
    /// 最大轮次 (固定 = 3)
    pub max_rounds: u8,
    /// 当前立场 (辩论过程中演化)
    pub current_stance: Stance,
    /// session 开始时间 (epoch ms)
    pub started_at_ms: i64,
}

impl PersonaSession {
    /// 创建新会话 (max_rounds = 3).
    pub fn new(session_id: impl Into<String>, persona: Persona, started_at_ms: i64) -> Self {
        let initial_kind = persona.initial_stance_kind();
        let current_stance = Stance::new(
            initial_kind,
            format!("初始立场 (基于 stance_bias={:.2})", persona.stance_bias),
        );
        Self {
            session_id: session_id.into(),
            persona,
            rounds: Vec::new(),
            current_round: 0,
            max_rounds: 3,
            current_stance,
            started_at_ms,
        }
    }

    /// 是否还能辩论 (当前轮 < 最大轮).
    pub fn can_debate(&self) -> bool {
        self.current_round < self.max_rounds
    }

    /// 已辩论轮数。
    pub fn rounds_held(&self) -> usize {
        self.rounds.len()
    }

    /// 是否已完成 (current_round == max_rounds).
    pub fn is_complete(&self) -> bool {
        self.current_round >= self.max_rounds
    }

    /// 记录一轮辩论产出 + 推进轮次 + 更新当前立场.
    pub fn record_round(&mut self, round: DebateRound) {
        self.current_stance = round.outcome.opinion.stance.clone();
        self.rounds.push(round);
        self.current_round += 1;
    }

    /// 构造拟人化 speech (基于 persona.voice + 当前立场).
    pub fn craft_speech(&self, stance: &Stance) -> String {
        format!(
            "【{} · {}】 秉持「{}」之精神, 吾持 {:?} 立场: {}",
            self.persona.name,
            self.persona.voice,
            self.persona.character,
            stance.kind,
            stance.description
        )
    }
}
