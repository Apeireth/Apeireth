//! R33-4-2: CouncilMember + Persona 组合 (per AutoGen ConversableAgent + R19 Persona)
//!
//! **目标**: R33-4 落了 `CouncilMember { role, goal, backstory, provider }` (组织定位 "做什么"),
//! R19 落了 `Persona { name, character, voice, stance_bias }` (拟人化 "怎么做").
//! R33-4-2 组合两者 — 让 LLM 既知道"做什么" (组织任务) 又知道"怎么说" (拟人化风格).
//!
//! **AutoGen 真代码借鉴** (`autogen/agentchat/conversable_agent.py`):
//! - `ConversableAgent.system_message` 借鉴: 把 system_message 拆成"角色 + 任务 + 行为背景"
//! - `human_input_mode` 借鉴: persona.character 决定 LLM 表达风格 (拟人化, 0 假, 0 操纵)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 R33-4 `council_member.rs` (4 字段 struct LOCKED)
//! - 0 改 R19 `persona.rs` (Persona 4 字段 LOCKED)
//! - 0 改 R33-4-1 `council_member_deliberation.rs` (复用 helper 函数 1:1, 0 触碰 core loop)
//! - 0 引入 I/O / 网络 (sync `MockLlmProvider` trait 路径)
//!
//! **复用 R33-4-1 helpers** (per R23 transparent 模式, 0 业务漂移):
//! - `parse_stance_from_text` — LLM response → StanceKind
//! - `compute_consensus_score` — 共识检测
//! - `score_to_stance` — 5 阈值映射
//!
//! **多轮协商流程** (per R33-4-1 + Persona 增强):
//! ```text
//! for round in 0..max_rounds:
//!     for member in members:  // member = PersonaBoundMember { member, persona }
//!         system = member.to_system_prompt()  // 拼 persona 性格 + member 4 段
//!         user = query.description + " [第 R 轮] [member #N role=X] 之前意见: ..."
//!         response = llm.generate(user, system)
//!         stance = parse_stance_from_text(response.text)
//!         speech = member.craft_speech(stance)  // 拟人化 speech (persona.name/voice/character)
//!         opinion = build_opinion(stance, speech, round)
//!     if consensus / strong_disapprove: break
//! ```

use std::sync::Arc;

use crate::advisor::{AdvisorId, AdvisorOpinion, Stance, StanceKind};
use crate::council_member::CouncilMember;
use crate::council_member_deliberation::{
    compute_consensus_score, parse_stance_from_text, score_to_stance, RoundSummary,
};
use crate::mock_llm::MockLlmProvider;
use crate::persona::Persona;
use serde::{Deserialize, Serialize};

/// PersonaBoundMember — CouncilMember + Persona 组合
///
/// **设计**:
/// - `member` (R33-4 LOCKED) 走"做什么" (组织任务)
/// - `persona` (R19 LOCKED) 走"怎么做" (拟人化风格)
/// - `to_system_prompt()` 拼 6 段: persona 3 字段 (name/character/voice) + member 4 字段
///   (role/goal/backstory/provider)
/// - `craft_speech(stance)` 拿拟人化 speech (复用 `Persona::craft_speech` 风格)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonaBoundMember {
    /// R33-4 CouncilMember — 走"做什么"
    pub member: CouncilMember,
    /// R19 Persona — 走"怎么做"
    pub persona: Persona,
}

impl PersonaBoundMember {
    /// 便利构造
    pub fn new(member: CouncilMember, persona: Persona) -> Self {
        Self { member, persona }
    }

    /// 拼 persona 3 段 (name/character/voice) + member 4 段 (role/goal/backstory/provider)
    ///
    /// **格式** (per AutoGen `ConversableAgent.system_message` 借鉴):
    /// ```text
    /// # 拟人身份
    /// 你是『<persona.name>』, <persona.character>, 表达风格: <persona.voice>
    ///
    /// # 角色 (Role)
    /// <member.role>
    ///
    /// # 目标 (Goal)
    /// <member.goal>
    ///
    /// # 背景 (Backstory)
    /// <member.backstory>
    ///
    /// # LLM Provider
    /// <member.provider>
    /// ```
    pub fn to_system_prompt(&self) -> String {
        format!(
            "# 拟人身份\n你是『{}』, {}, 表达风格: {}\n\n\
             # 角色 (Role)\n{}\n\n\
             # 目标 (Goal)\n{}\n\n\
             # 背景 (Backstory)\n{}\n\n\
             # LLM Provider\n{}",
            self.persona.name,
            self.persona.character,
            self.persona.voice,
            self.member.role,
            self.member.goal,
            self.member.backstory,
            self.member.provider,
        )
    }

    /// 拿 persona 风格 + member role 拼的 speech (per `Persona::craft_speech` 借鉴)
    pub fn craft_speech(&self, stance: &Stance) -> String {
        format!(
            "【{} · {}】 角色『{}』立场 {}: {}",
            self.persona.name,
            self.persona.voice,
            self.member.role,
            self.member.goal,
            stance.description,
        )
    }

    /// 初始 stance (per `Persona::initial_stance_kind` 1:1 复用)
    pub fn initial_stance_kind(&self) -> StanceKind {
        self.persona.initial_stance_kind()
    }

    /// member 引用
    pub fn member(&self) -> &CouncilMember {
        &self.member
    }

    /// persona 引用
    pub fn persona(&self) -> &Persona {
        &self.persona
    }
}

/// PersonaBound 协商轮 — 跟 R33-4-1 RoundSummary 类似, 但含 persona speech
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonaBoundRound {
    /// 轮次 (0-based)
    pub round: u8,
    /// 该轮每个 member 的意见
    pub opinions: Vec<AdvisorOpinion>,
    /// 该轮每个 member 的拟人化 speech (per `PersonaBoundMember::craft_speech`)
    pub speeches: Vec<String>,
    /// 共识分数 (0.0 - 1.0)
    pub consensus_score: f64,
    /// 转录: 该轮每个 member 的 stance + speech (人类可读)
    pub transcript: String,
}

impl PersonaBoundRound {
    /// 该轮共识达成?
    pub fn consensus_reached(&self) -> bool {
        self.consensus_score >= crate::council_member_deliberation::CONSENSUS_SCORE_THRESHOLD
    }

    /// 该轮有强反对?
    pub fn has_strong_disapprove(&self) -> bool {
        self.opinions
            .iter()
            .any(|o| o.stance.kind.is_strong_disapprove() && o.confidence >= 0.5)
    }

    /// 0:1 复用 R33-4-1 `RoundSummary` (per R23 transparent pattern)
    pub fn to_round_summary(&self) -> RoundSummary {
        RoundSummary {
            round: self.round,
            opinions: self.opinions.clone(),
            consensus_score: self.consensus_score,
            transcript: self.transcript.clone(),
        }
    }
}

/// PersonaBound 多轮协商最终 verdict
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonaBoundVerdict {
    /// Query ID
    pub query_id: String,
    /// Session ID
    pub session_id: String,
    /// 每轮 summary
    pub rounds: Vec<PersonaBoundRound>,
    /// 共识达成?
    pub consensus_reached: bool,
    /// 终止原因: "consensus" | "max_rounds" | "strong_disapprove" | "empty_members"
    pub termination_reason: String,
    /// 最终 weighted score (0.0 - 1.0)
    pub final_weighted_score: f64,
    /// 最终 stance
    pub final_stance: StanceKind,
    /// 耗时 (ms)
    pub elapsed_ms: u64,
    /// 实际跑的轮数
    pub rounds_run: u8,
    /// PersonaBoundMember 最终 summary
    pub member_summaries: Vec<PersonaBoundSummary>,
}

/// 单 PersonaBoundMember 最终 summary
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonaBoundSummary {
    /// member role
    pub role: String,
    /// persona name
    pub persona_name: String,
    /// member provider
    pub provider: String,
    /// 最终 stance
    pub final_stance: StanceKind,
    /// 最终 confidence
    pub final_confidence: f64,
    /// 最终 speech (拟人化)
    pub final_speech: String,
}

impl PersonaBoundVerdict {
    /// 是否允许
    pub fn is_allowed(&self) -> bool {
        self.consensus_reached
            && self.final_stance.score() > 0.0
            && !self.rounds.iter().any(|r| r.has_strong_disapprove())
    }

    /// 7 阶段 eval score (1:1 per `MultiRoundVerdict::to_eval_scores`)
    pub fn to_eval_scores(&self) -> Vec<(&'static str, f64)> {
        vec![
            (
                "members_recruited",
                if !self.member_summaries.is_empty() { 1.0 } else { 0.0 },
            ),
            (
                "rounds_run",
                f64::from(self.rounds_run)
                    / f64::from(crate::council_member_deliberation::DEFAULT_MAX_ROUNDS),
            ),
            (
                "consensus_reached",
                if self.consensus_reached { 1.0 } else { 0.0 },
            ),
            (
                "weighted_score_normalized",
                (self.final_weighted_score + 1.0) / 2.0,
            ),
            (
                "no_strong_disapprove",
                if !self.rounds.iter().any(|r| r.has_strong_disapprove()) {
                    1.0
                } else {
                    0.0
                },
            ),
            (
                "termination_clean",
                if self.termination_reason == "consensus"
                    || self.termination_reason == "max_rounds"
                {
                    1.0
                } else {
                    0.0
                },
            ),
            (
                "members_aligned",
                {
                    let n = self.member_summaries.len() as f64;
                    if n == 0.0 {
                        0.0
                    } else {
                        self.member_summaries
                            .iter()
                            .filter(|m| m.final_stance.score() > 0.0)
                            .count() as f64
                            / n
                    }
                },
            ),
        ]
    }
}

/// PersonaBound 多轮协商驱动器 — 复用 R33-4-1 helpers, 0 改 core loop
pub struct PersonaBoundDeliberator {
    members: Vec<PersonaBoundMember>,
    llm: Option<Arc<dyn MockLlmProvider>>,
    max_rounds: u8,
    next_session_seq: u64,
}

impl PersonaBoundDeliberator {
    /// 构造 (0 LLM, 默认 max_rounds = 3, per R33-4-1 DEFAULT_MAX_ROUNDS 1:1)
    pub fn new(members: Vec<PersonaBoundMember>) -> Self {
        Self {
            members,
            llm: None,
            max_rounds: crate::council_member_deliberation::DEFAULT_MAX_ROUNDS,
            next_session_seq: 0,
        }
    }

    /// 注入 mock LLM
    pub fn with_mock_llm(mut self, llm: Arc<dyn MockLlmProvider>) -> Self {
        self.llm = Some(llm);
        self
    }

    /// 注入真 LLM (apeireth-api::llm::LlmProvider) — 内部包成 LlmAdvisorBackend
    /// adapter 复用 R16-09 已有逻辑, 0 改 council_member_deliberation.rs.
    ///
    /// **便利入口**: 比手写 `Arc::new(LlmAdvisorBackend::new(llm))` 短, 给 R33-4 后续
    /// council deliberation 跨 provider 协商用 (per R37-1 集成点).
    pub fn with_llm_provider(
        mut self,
        llm: std::sync::Arc<dyn apeireth_api::llm::LlmProvider>,
    ) -> Self {
        self.llm = Some(std::sync::Arc::new(
            crate::llm_backend::LlmAdvisorBackend::new(llm),
        ));
        self
    }

    /// 自定义 max_rounds
    pub fn with_max_rounds(mut self, n: u8) -> Self {
        self.max_rounds = n.max(1);
        self
    }

    /// 当前 members 数
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// max_rounds
    pub fn max_rounds(&self) -> u8 {
        self.max_rounds
    }

    /// 是否挂了 LLM
    pub fn has_llm(&self) -> bool {
        self.llm.is_some()
    }

    /// 跑多轮协商 (sync) — 主入口
    ///
    /// **跟 R33-4-1 CouncilMemberDeliberator::deliberate 同流程, 唯一不同**:
    /// - 1. system_prompt 改用 `PersonaBoundMember::to_system_prompt()` (含 persona 3 字段)
    /// - 2. transcript 加 persona speech
    /// - 3. 复用 R33-4-1 helpers (`parse_stance_from_text` / `compute_consensus_score` /
    ///    `score_to_stance`) 0 改
    pub fn deliberate(
        &mut self,
        query: &crate::deliberation::CouncilQuery,
    ) -> PersonaBoundVerdict {
        let started_at_ms = query.started_at_ms;
        self.next_session_seq += 1;
        let session_id = format!("pbd-session-{:06}", self.next_session_seq);

        if self.members.is_empty() {
            return PersonaBoundVerdict {
                query_id: query.query_id.clone(),
                session_id,
                rounds: Vec::new(),
                consensus_reached: false,
                termination_reason: "empty_members".to_string(),
                final_weighted_score: 0.0,
                final_stance: StanceKind::Neutral,
                elapsed_ms: 0,
                rounds_run: 0,
                member_summaries: Vec::new(),
            };
        }

        let mut rounds: Vec<PersonaBoundRound> = Vec::new();
        let mut prior_opinions_text = String::new();
        let mut consensus_reached = false;
        let mut termination_reason = "max_rounds".to_string();
        let mut rounds_run: u8 = 0;

        for round in 0..self.max_rounds {
            rounds_run = round + 1;
            let mut round_opinions: Vec<AdvisorOpinion> = Vec::new();
            let mut round_speeches: Vec<String> = Vec::new();
            let mut transcript_parts: Vec<String> = Vec::new();

            for (idx, pbm) in self.members.iter().enumerate() {
                let system = pbm.to_system_prompt();
                let user = format!(
                    "{}\n\n[第 {} 轮] [member #{} role={} persona={}]\n之前意见:\n{}",
                    query.description,
                    round + 1,
                    idx,
                    pbm.member.role,
                    pbm.persona.name,
                    if prior_opinions_text.is_empty() {
                        "无 (第 1 轮)".to_string()
                    } else {
                        prior_opinions_text.clone()
                    },
                );

                let (stance_kind, confidence, text) = if let Some(llm) = &self.llm {
                    let resp = llm.generate(&user, &system);
                    let kind = parse_stance_from_text(&resp.text);
                    (kind, resp.confidence, resp.text)
                } else {
                    let kind = pbm.initial_stance_kind();
                    (kind, 0.7, format!("[no-llm persona] {:?}", kind))
                };

                let stance = Stance::new(
                    stance_kind,
                    format!(
                        "{:?} (member: {}, persona: {}, round {})",
                        stance_kind,
                        pbm.member.role,
                        pbm.persona.name,
                        round + 1
                    ),
                );
                let opinion = AdvisorOpinion::new(
                    AdvisorId::new(format!("pbd-{}-r{}", pbm.member.role, round + 1)),
                    stance.clone(),
                    confidence,
                    format!("Round {}: {}", round + 1, text),
                    started_at_ms,
                );
                let speech = pbm.craft_speech(&stance);

                transcript_parts.push(format!(
                    "[R{} #{} {} · {}] {:?} ({:.0}%) — {}",
                    round + 1,
                    idx,
                    pbm.member.role,
                    pbm.persona.name,
                    stance_kind,
                    confidence * 100.0,
                    speech
                ));
                round_speeches.push(speech);
                round_opinions.push(opinion);
            }

            let consensus_score = compute_consensus_score(&round_opinions);
            let transcript = transcript_parts.join(" | ");

            let summary = PersonaBoundRound {
                round,
                opinions: round_opinions.clone(),
                speeches: round_speeches,
                consensus_score,
                transcript,
            };

            // 强反对检测
            if summary.has_strong_disapprove() {
                termination_reason = "strong_disapprove".to_string();
                rounds.push(summary);
                break;
            }

            // 共识检测
            if summary.consensus_reached() {
                consensus_reached = true;
                termination_reason = "consensus".to_string();
                rounds.push(summary);
                break;
            }

            // 准备下一轮 prior_opinions_text
            prior_opinions_text = round_opinions
                .iter()
                .map(|o| {
                    format!(
                        "- {} ({}): {:?} ({:.0}%) — {}",
                        o.advisor_id.as_str(),
                        o.stance.description,
                        o.stance.kind,
                        o.confidence * 100.0,
                        o.reasoning
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            rounds.push(summary);
        }

        // 合成 final verdict
        let last_round = rounds.last();
        let final_weighted_score = last_round.map(|r| r.consensus_score).unwrap_or(0.0);
        let final_stance = score_to_stance(final_weighted_score);

        let member_summaries: Vec<PersonaBoundSummary> = if let Some(r) = last_round {
            self.members
                .iter()
                .zip(r.opinions.iter())
                .zip(r.speeches.iter())
                .map(|((pbm, o), speech)| PersonaBoundSummary {
                    role: pbm.member.role.clone(),
                    persona_name: pbm.persona.name.clone(),
                    provider: pbm.member.provider.clone(),
                    final_stance: o.stance.kind,
                    final_confidence: o.confidence,
                    final_speech: speech.clone(),
                })
                .collect()
        } else {
            self.members
                .iter()
                .map(|pbm| PersonaBoundSummary {
                    role: pbm.member.role.clone(),
                    persona_name: pbm.persona.name.clone(),
                    provider: pbm.member.provider.clone(),
                    final_stance: StanceKind::Neutral,
                    final_confidence: 0.0,
                    final_speech: String::new(),
                })
                .collect()
        };

        let elapsed_ms = (current_time_ms() - started_at_ms).max(0) as u64;

        PersonaBoundVerdict {
            query_id: query.query_id.clone(),
            session_id,
            rounds,
            consensus_reached,
            termination_reason,
            final_weighted_score,
            final_stance,
            elapsed_ms,
            rounds_run,
            member_summaries,
        }
    }
}

fn current_time_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deliberation::CouncilQuery;
    use crate::mock_llm::{MockLlmResponse, ScriptedMockLlm};

    const NOW: i64 = 1_700_000_000_000;

    fn q(desc: &str) -> CouncilQuery {
        CouncilQuery::new("q-pbd", desc, NOW)
    }

    fn standard_3_pbm() -> Vec<PersonaBoundMember> {
        vec![
            PersonaBoundMember::new(
                CouncilMember::new("architect", "设计稳的架构", "10 年 Rust", "claude_code"),
                Persona::new("诺克斯", "沉稳工程师", "简洁严谨", 0.4),
            ),
            PersonaBoundMember::new(
                CouncilMember::new("security_reviewer", "找安全漏洞", "5 年安全", "codex"),
                Persona::new("赛琳", "严谨审计", "精准犀利", 0.1),
            ),
            PersonaBoundMember::new(
                CouncilMember::new("product_manager", "用户价值", "5 年产品", "gemini_cli"),
                Persona::new("艾拉", "用户视角", "温和共情", 0.6),
            ),
        ]
    }

    #[test]
    fn new_no_llm_default_max_rounds_3() {
        let d = PersonaBoundDeliberator::new(standard_3_pbm());
        assert_eq!(d.member_count(), 3);
        assert_eq!(d.max_rounds(), 3);
        assert!(!d.has_llm());
    }

    #[test]
    fn persona_bound_member_to_system_prompt_contains_6_sections() {
        let pbm = &standard_3_pbm()[0];
        let p = pbm.to_system_prompt();
        assert!(p.contains("# 拟人身份"));
        assert!(p.contains("诺克斯"));
        assert!(p.contains("沉稳工程师"));
        assert!(p.contains("简洁严谨"));
        assert!(p.contains("# 角色 (Role)"));
        assert!(p.contains("architect"));
        assert!(p.contains("# 目标 (Goal)"));
        assert!(p.contains("设计稳的架构"));
        assert!(p.contains("# 背景 (Backstory)"));
        assert!(p.contains("10 年 Rust"));
        assert!(p.contains("# LLM Provider"));
        assert!(p.contains("claude_code"));
    }

    #[test]
    fn persona_bound_member_craft_speech_contains_role_and_persona() {
        let pbm = &standard_3_pbm()[0];
        let stance = Stance::strong_approve("looks good");
        let s = pbm.craft_speech(&stance);
        assert!(s.contains("诺克斯"));
        assert!(s.contains("architect"));
        assert!(s.contains("looks good"));
    }

    #[test]
    fn persona_bound_member_initial_stance_kind_from_persona() {
        // stance_bias 0.4 → Approve
        assert_eq!(standard_3_pbm()[0].initial_stance_kind(), StanceKind::Approve);
        // stance_bias 0.1 → Neutral
        assert_eq!(standard_3_pbm()[1].initial_stance_kind(), StanceKind::Neutral);
        // stance_bias 0.6 → StrongApprove
        assert_eq!(standard_3_pbm()[2].initial_stance_kind(), StanceKind::StrongApprove);
    }

    #[test]
    fn deliberate_empty_members_returns_empty_verdict() {
        let mut d = PersonaBoundDeliberator::new(vec![]);
        let v = d.deliberate(&q("test"));
        assert_eq!(v.termination_reason, "empty_members");
        assert_eq!(v.rounds_run, 0);
        assert_eq!(v.rounds.len(), 0);
        assert_eq!(v.member_summaries.len(), 0);
        assert!(!v.is_allowed());
    }

    #[test]
    fn deliberate_no_llm_initial_stance_aware_3_pbm() {
        // 3 pbm initial: Approve / Neutral / StrongApprove
        // weighted consensus = 0.5 * (0.8 + 0.5 + 1.0) / 3 ≈ 0.767
        // >= 0.6 → 共识 round 1
        let mut d = PersonaBoundDeliberator::new(standard_3_pbm());
        let v = d.deliberate(&q("ship it"));
        assert_eq!(v.termination_reason, "consensus");
        assert_eq!(v.rounds_run, 1);
        assert!(v.consensus_reached);
        assert!(v.is_allowed());
        assert_eq!(v.member_summaries.len(), 3);
        // member_summaries 反映 initial_stance_kind
        assert_eq!(v.member_summaries[0].final_stance, StanceKind::Approve);
        assert_eq!(v.member_summaries[1].final_stance, StanceKind::Neutral);
        assert_eq!(v.member_summaries[2].final_stance, StanceKind::StrongApprove);
        // final_speech 包含 persona name + role
        for s in &v.member_summaries {
            assert!(!s.final_speech.is_empty());
            assert!(s.final_speech.contains(&s.persona_name));
            assert!(s.final_speech.contains(&s.role));
        }
    }

    #[test]
    fn deliberate_with_mock_llm_strong_disapprove_triggers_hold() {
        let llm = Arc::new(
            ScriptedMockLlm::new().with_default(MockLlmResponse::reject("StrongDisapprove — bad")),
        );
        let mut d = PersonaBoundDeliberator::new(standard_3_pbm()).with_mock_llm(llm);
        let v = d.deliberate(&q("any"));
        assert_eq!(v.termination_reason, "strong_disapprove");
        assert_eq!(v.rounds_run, 1);
        assert!(!v.is_allowed());
    }

    #[test]
    fn persona_bound_round_to_round_summary_0_drift() {
        let r = PersonaBoundRound {
            round: 0,
            opinions: vec![],
            speeches: vec![],
            consensus_score: 0.5,
            transcript: "test".to_string(),
        };
        let s = r.to_round_summary();
        assert_eq!(s.round, 0);
        assert!((s.consensus_score - 0.5).abs() < 0.001);
        assert_eq!(s.transcript, "test");
    }

    #[test]
    fn session_id_monotonic_pbd_prefix() {
        let mut d = PersonaBoundDeliberator::new(standard_3_pbm());
        let v1 = d.deliberate(&q("a"));
        let v2 = d.deliberate(&q("b"));
        assert_ne!(v1.session_id, v2.session_id);
        assert!(v2.session_id.contains("pbd-session-"));
    }

    #[test]
    fn eval_scores_7_stages() {
        let v = PersonaBoundVerdict {
            query_id: "q".to_string(),
            session_id: "pbd-session-000001".to_string(),
            rounds: vec![],
            consensus_reached: true,
            termination_reason: "consensus".to_string(),
            final_weighted_score: 0.85,
            final_stance: StanceKind::Approve,
            elapsed_ms: 100,
            rounds_run: 1,
            member_summaries: vec![PersonaBoundSummary {
                role: "a".to_string(),
                persona_name: "p".to_string(),
                provider: "claude_code".to_string(),
                final_stance: StanceKind::Approve,
                final_confidence: 0.9,
                final_speech: "ok".to_string(),
            }],
        };
        let scores = v.to_eval_scores();
        assert_eq!(scores.len(), 7);
        for (name, val) in &scores {
            assert!(!name.is_empty());
            assert!(*val >= 0.0 && *val <= 1.0, "{name} = {val}");
        }
    }
}
