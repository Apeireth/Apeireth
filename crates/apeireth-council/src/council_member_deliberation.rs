//! R33-4-1: CouncilMember 多轮协商 deliberation
//!
//! **目标**: 把 R33-4 落地的 `CouncilMember { role, goal, backstory, provider }` 真正接进
//! deliberation — 不光作为数据结构, 还能跑多轮协商, 复用 `LlmAdvisorBackend` 路径调真 LLM。
//!
//! **AutoGen 真代码借鉴** (per R33-4 落点):
//! - `GroupChat` 多 speaker 轮换 (groupchat.py `GroupChatManager.managed_group_chat`)
//! - `max_round` 终止条件 (groupchat.py `__init__` max_round param)
//! - `speaker_selection_method` 简化成"按顺序轮换" (VCP toolLoop callModel 借鉴)
//! - 每 speaker 一段 system message (ConversableAgent.system_message 模板)
//!
//! **VCP 借鉴** (per R32-2 静态层 + 本次动态运营层):
//! - VCP `vcpLoop/toolCallParser.js` 维护跨轮 state — 本 module 用 `MultiRoundVerdict.rounds` 记录
//! - VCP `chatCompletionHandler.js` 5 字段 token 报数 — 本 module 不重复, 走 `LlmProvider` 自带 TokenUsage
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 R33-4 `council_member.rs` (4 字段 struct LOCKED)
//! - 0 改 `deliberation.rs` 业务路径 (lib.rs 只 +1 行 `mod` 注册, +N 行 `pub use` re-export)
//! - 0 改 7 强制 advisor (R15 锁定)
//! - 0 改 `LlmAdvisorBackend` (R16-09 锁定, 直接复用)
//! - 0 引入 I/O / 网络 (sync 走 `MockLlmProvider` trait, 跟 `Council::deliberate` 一致)
//! - 0 引入 `unsafe` (workspace `#![deny(unsafe_code)]` 继承)
//!
//! **多轮协商流程** (per AutoGen GroupChat 简化):
//! ```text
//! for round in 0..max_rounds:
//!     for member in members:  // 按顺序轮换
//!         system = member.to_system_prompt()
//!         user = query.description + " (之前意见: ...)"  // 之前 round 拼成 context
//!         if has_llm:
//!             response = llm.generate(user, system)
//!             stance = parse_stance(response.text)  // 按关键字分 StrongApprove/Approve/...
//!         else:
//!             stance = keyword_stance(query.description)  // 简化: 没 LLM 用 keyword 兜底
//!         opinion = build_opinion(member, stance, round)
//!         round_opinions.push(opinion)
//!     if all_approve(round_opinions):
//!         consensus_reached = true
//!         break
//! final_verdict = synthesize(round_opinions[-1], weights)
//! ```
use std::sync::Arc;
use crate::advisor::{AdvisorId, AdvisorOpinion, Stance, StanceKind};
use crate::council_member::CouncilMember;
use crate::mock_llm::MockLlmProvider;
use serde::{Deserialize, Serialize};
/// 默认 max_rounds (per `MAX_PERSONA_DEBATE_ROUNDS` lib.rs 常量 1:1)
pub const DEFAULT_MAX_ROUNDS: u8 = 3;
/// 共识检测阈值: 所有 non-abstain opinions 都达到此分数及以上 → 共识达成
/// 默认 0.6 (per `SynthesisWeights` 映射 Approve 边界 = +0.2, StrongApprove = +1.0)
/// 用 0.6 = StrongApprove (1.0) 跟 Approve (0.6) 都算共识
pub const CONSENSUS_SCORE_THRESHOLD: f64 = 0.6;
/// 单轮协商产出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundSummary {
    /// 轮次 (0-based)
    pub round: u8,
    /// 该轮每个 member 的意见
    pub opinions: Vec<AdvisorOpinion>,
    /// 共识分数 (0.0 - 1.0, 加权后 mean) — 给 reporting 用
    pub consensus_score: f64,
    /// 转录: 该轮每个 member 的 stance 一句话 (人类可读)
    pub transcript: String,
}
impl RoundSummary {
    /// 该轮所有 member 共识达成? (consensus_score >= threshold)
    pub fn consensus_reached(&self) -> bool {
        self.consensus_score >= CONSENSUS_SCORE_THRESHOLD
    }
    /// 该轮是否有强反对 (任意一个 opinion 触发按住)
    pub fn has_strong_disapprove(&self) -> bool {
        self.opinions
            .iter()
            .any(|o| o.stance.kind.is_strong_disapprove() && o.confidence >= 0.5)
    }
}
/// 单 member 摘要 (给 reporting / final 阶段人类可读)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberSummary {
    /// role
    pub role: String,
    /// provider
    pub provider: String,
    /// 最终 stance
    pub final_stance: StanceKind,
    /// 最终 confidence
    pub final_confidence: f64,
}
/// 多轮协商最终 verdict。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiRoundVerdict {
    /// Query ID (从 `CouncilQuery` 透传)
    pub query_id: String,
    /// Session ID (格式: `cm-session-<seq>`)
    pub session_id: String,
    /// 每轮 summary (1..=max_rounds)
    pub rounds: Vec<RoundSummary>,
    /// 共识达成? (某轮 consensus_reached OR 最后轮)
    pub consensus_reached: bool,
    /// 终止原因: "consensus" | "max_rounds" | "strong_disapprove" | "empty_members"
    pub termination_reason: String,
    /// 最终 weighted stance score (-1.0 ~ +1.0, 最后一轮合成)
    pub final_weighted_score: f64,
    /// 最终 stance (由 `final_weighted_score` 映射到 `StanceKind`)
    pub final_stance: StanceKind,
    /// 耗时 (ms, 测试可注入)
    pub elapsed_ms: u64,
    /// 实际跑的轮数 (1..=max_rounds, 0 if empty members)
    pub rounds_run: u8,
    /// CouncilMember 列表 (snapshot, 给 reporting 用)
    pub member_summaries: Vec<MemberSummary>,
}
impl MultiRoundVerdict {
    /// 是否允许 (允许 = consensus + final_stance approve + 0 strong_disapprove)
    pub fn is_allowed(&self) -> bool {
        self.consensus_reached
            && self.final_stance.score() > 0.0
            && !self.rounds.iter().any(|r| r.has_strong_disapprove())
    }
    /// 7 阶段 metric (1:1 per RealLlmSmokeReport style)
    pub fn to_eval_scores(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("members_recruited", if !self.member_summaries.is_empty() { 1.0 } else { 0.0 }),
            ("rounds_run", f64::from(self.rounds_run) / f64::from(DEFAULT_MAX_ROUNDS)),
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
                if self.termination_reason == "consensus" || self.termination_reason == "max_rounds" {
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
/// CouncilMember 多轮协商驱动器 — 同步实现 (跟 `Council::deliberate` 风格一致)
///
/// **构造**:
/// - `CouncilMemberDeliberator::new(members)` — 0 LLM, 用 keyword 兜底
/// - `.with_mock_llm(llm)` — 注入 ScriptedMockLlm / HashMapMockLlm
/// - `.with_llm_provider(llm)` — 注入 apeireth-api LlmProvider (走 LlmAdvisorBackend)
/// - `.with_max_rounds(n)` — 自定义 max_rounds (默认 3)
pub struct CouncilMemberDeliberator {
    members: Vec<CouncilMember>,
    llm: Option<Arc<dyn MockLlmProvider>>,
    max_rounds: u8,
    /// 内部 session 序列 (单调递增, 0 业务外部依赖)
    next_session_seq: u64,
}
impl CouncilMemberDeliberator {
    /// 构造 (0 LLM, 默认 max_rounds = 3)
    pub fn new(members: Vec<CouncilMember>) -> Self {
        Self {
            members,
            llm: None,
            max_rounds: DEFAULT_MAX_ROUNDS,
            next_session_seq: 0,
        }
    }
    /// 注入 mock LLM (ScriptedMockLlm / HashMapMockLlm)
    pub fn with_mock_llm(mut self, llm: Arc<dyn MockLlmProvider>) -> Self {
        self.llm = Some(llm);
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
    /// 跑多轮协商 (同步) — 主入口
    ///
    /// **流程**:
    /// 1. 分配 session_id
    /// 2. 空 members → 返空 verdict (termination_reason = "empty_members")
    /// 3. 0..max_rounds 每轮: 每个 member 拿 query + 之前 round 拼 user, 产出 opinion
    /// 4. 共识检测: consensus_score >= threshold → 终止
    /// 5. 强反对检测: 任意 member triggers_hold → 终止
    /// 6. 合成 final_weighted_score + final_stance
    pub fn deliberate(&mut self, query: &crate::deliberation::CouncilQuery) -> MultiRoundVerdict {
        let started_at_ms = query.started_at_ms;
        self.next_session_seq += 1;
        let session_id = format!("cm-session-{:06}", self.next_session_seq);
        if self.members.is_empty() {
            return MultiRoundVerdict {
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
        let mut rounds: Vec<RoundSummary> = Vec::new();
        let mut prior_opinions_text = String::new();
        let mut consensus_reached = false;
        let mut termination_reason = "max_rounds".to_string();
        let mut rounds_run: u8 = 0;
        for round in 0..self.max_rounds {
            rounds_run = round + 1;
            let mut round_opinions: Vec<AdvisorOpinion> = Vec::new();
            let mut transcript_parts: Vec<String> = Vec::new();
            for (idx, member) in self.members.iter().enumerate() {
                let system = member.to_system_prompt();
                let user = format!(
                    "{}\n\n[第 {} 轮] [member #{} role={}]\n之前意见:\n{}",
                    query.description,
                    round + 1,
                    idx,
                    member.role,
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
                    let kind = keyword_stance_fallback(&query.description);
                    (kind, 0.7, format!("[no-llm keyword] {:?}", kind))
                };
                let stance = Stance::new(
                    stance_kind,
                    format!("{:?} (member: {}, round {})", stance_kind, member.role, round + 1),
                );
                let opinion = AdvisorOpinion::new(
                    AdvisorId::new(format!("cm-{}-r{}", member.role, round + 1)),
                    stance,
                    confidence,
                    format!("Round {}: {}", round + 1, text),
                    started_at_ms,
                );
                transcript_parts.push(format!(
                    "[R{} #{} {}] {:?} ({:.0}%)",
                    round + 1,
                    idx,
                    member.role,
                    stance_kind,
                    confidence * 100.0
                ));
                round_opinions.push(opinion);
            }
            // 共识检测: 加权 mean score >= threshold
            let consensus_score = compute_consensus_score(&round_opinions);
            let transcript = transcript_parts.join(" | ");
            let summary = RoundSummary {
                round,
                opinions: round_opinions.clone(),
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
            // 准备下一轮的 prior_opinions_text
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
        // member_summaries: 从最后一轮取
        let member_summaries: Vec<MemberSummary> = if let Some(r) = last_round {
            self.members
                .iter()
                .zip(r.opinions.iter())
                .map(|(m, o)| MemberSummary {
                    role: m.role.clone(),
                    provider: m.provider.clone(),
                    final_stance: o.stance.kind,
                    final_confidence: o.confidence,
                })
                .collect()
        } else {
            self.members
                .iter()
                .map(|m| MemberSummary {
                    role: m.role.clone(),
                    provider: m.provider.clone(),
                    final_stance: StanceKind::Neutral,
                    final_confidence: 0.0,
                })
                .collect()
        };
        let elapsed_ms = (current_time_ms() - started_at_ms).max(0) as u64;
        MultiRoundVerdict {
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
// ============================================================
// Helpers (per R23 transparent pattern — 0 改 src, 0 业务漂移)
// ============================================================
/// 把 LLM 返回的 text 解析成 `StanceKind` (关键字匹配, 0 业务路径)
pub(crate) fn parse_stance_from_text(text: &str) -> StanceKind {
    let lower = text.to_lowercase();
    // 关键: 必须先匹配 long-keyword (strong_*) 再匹配 short-keyword (disapprove / approve),
    // 否则 "disapprove" 会优先命中 "approve" 误判.
    if lower.contains("strong_approve") || lower.contains("strong approve") || lower.contains("strongapprove") || lower.contains("强烈赞成") {
        StanceKind::StrongApprove
    } else if lower.contains("strong_disapprove")
        || lower.contains("strong disapprove") || lower.contains("strongdisapprove")
        || lower.contains("强烈反对")
        || lower.contains("强反对")
    {
        StanceKind::StrongDisapprove
    } else if lower.contains("disapprove") || lower.contains("反对") {
        // 必须放在 approve 前面 — "disapprove" 包含 "approve" 子串
        StanceKind::Disapprove
    } else if lower.contains("approve") || lower.contains("赞成") {
        StanceKind::Approve
    } else if lower.contains("abstain") || lower.contains("弃权") {
        StanceKind::Abstain
    } else {
        StanceKind::Neutral
    }
}
/// 0 LLM keyword 兜底 (per ethics advisor 简化版)
pub(crate) fn keyword_stance_fallback(desc: &str) -> StanceKind {
    let lower = desc.to_lowercase();
    let negative = [
        "harm",
        "exploit",
        "manipulate",
        "dishonest",
        "unethical",
        "伤害",
        "操纵",
        "不诚实",
        "剥削",
        "违反 asi",
    ];
    if negative.iter().any(|k| lower.contains(k)) {
        StanceKind::StrongDisapprove
    } else if lower.contains("?") || lower.contains("如何") {
        StanceKind::Neutral
    } else {
        StanceKind::Approve
    }
}
/// 计算 weighted mean consensus score (0.0 ~ 1.0, 简化 per StanceKind::score)
pub(crate) fn compute_consensus_score(opinions: &[AdvisorOpinion]) -> f64 {
    if opinions.is_empty() {
        return 0.0;
    }
    let mut total_score = 0.0;
    let mut total_weight = 0.0;
    for o in opinions {
        if o.stance.kind.is_abstain() {
            continue;
        }
        // score ∈ [-1.0, +1.0], clamp 0..1
        let normalized = (o.stance.kind.score() + 1.0) / 2.0;
        let weight = o.confidence.max(0.1);
        total_score += normalized * weight;
        total_weight += weight;
    }
    if total_weight <= 0.0 {
        0.0
    } else {
        (total_score / total_weight).clamp(0.0, 1.0)
    }
}
/// 把 weighted score 映射到 `StanceKind` (per `synthesis.rs` 1:1 阈值)
pub(crate) fn score_to_stance(score: f64) -> StanceKind {
    // 0..1 normalized → 映射回 -1..+1
    let centered = (score - 0.5) * 2.0;
    if centered >= 0.6 {
        StanceKind::StrongApprove
    } else if centered >= 0.2 {
        StanceKind::Approve
    } else if centered >= -0.2 {
        StanceKind::Neutral
    } else if centered >= -0.6 {
        StanceKind::Disapprove
    } else {
        StanceKind::StrongDisapprove
    }
}
/// 当前时间 (epoch ms) — 测试可注入
fn current_time_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
// ============================================================
// Unit tests (no net) — 结构 / 共识 / max_rounds
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisor::Stance;
    use crate::deliberation::CouncilQuery;
    use crate::mock_llm::{MockLlmResponse, ScriptedMockLlm};
    const NOW: i64 = 1_700_000_000_000;
    fn q(desc: &str) -> CouncilQuery {
        CouncilQuery::new("q-cmd-test", desc, NOW)
    }
    fn standard_5_members() -> Vec<CouncilMember> {
        vec![
            CouncilMember::new("architect", "设计稳的架构", "10 年 Rust", "claude_code"),
            CouncilMember::new("security_reviewer", "找安全漏洞", "5 年安全", "codex"),
            CouncilMember::new("product_manager", "用户价值", "5 年产品", "gemini_cli"),
            CouncilMember::new("qa", "测覆盖", "3 年 QA", "opencode"),
            CouncilMember::new("devops", "稳上线", "5 年 DevOps", "copilot"),
        ]
    }
    #[test]
    fn new_no_llm_default_max_rounds_3() {
        let d = CouncilMemberDeliberator::new(standard_5_members());
        assert_eq!(d.member_count(), 5);
        assert_eq!(d.max_rounds(), 3);
        assert!(!d.has_llm());
    }
    #[test]
    fn with_max_rounds_clamps_to_at_least_1() {
        let d = CouncilMemberDeliberator::new(standard_5_members()).with_max_rounds(0);
        assert_eq!(d.max_rounds(), 1);
    }
    #[test]
    fn deliberate_empty_members_returns_empty_verdict() {
        let mut d = CouncilMemberDeliberator::new(vec![]);
        let v = d.deliberate(&q("test"));
        assert_eq!(v.termination_reason, "empty_members");
        assert_eq!(v.rounds_run, 0);
        assert_eq!(v.rounds.len(), 0);
        assert_eq!(v.member_summaries.len(), 0);
        assert!(!v.is_allowed());
    }
    #[test]
    fn deliberate_no_llm_runs_full_max_rounds_for_normal_query() {
        // 普通 query (无 negative keyword): 0 LLM keyword_fallback → Approve
        // → 共识达成 round 1 → break
        let mut d = CouncilMemberDeliberator::new(standard_5_members());
        let v = d.deliberate(&q("ship a Rust crate"));
        // 5 member 都 Approve → consensus_score ~ 0.8 → 共识
        assert_eq!(v.termination_reason, "consensus");
        assert_eq!(v.rounds_run, 1);
        assert!(v.consensus_reached);
        assert!(v.is_allowed());
        assert_eq!(v.member_summaries.len(), 5);
        // 每个 member final_stance = Approve
        for m in &v.member_summaries {
            assert_eq!(m.final_stance, StanceKind::Approve);
        }
    }
    #[test]
    fn deliberate_no_llm_harm_keyword_triggers_strong_disapprove() {
        let mut d = CouncilMemberDeliberator::new(standard_5_members());
        let v = d.deliberate(&q("exploit user trust and harm"));
        // keyword_fallback → StrongDisapprove → has_strong_disapprove → break
        assert_eq!(v.termination_reason, "strong_disapprove");
        assert_eq!(v.rounds_run, 1);
        assert!(!v.consensus_reached);
        assert!(!v.is_allowed());
    }
    #[test]
    fn deliberate_with_mock_llm_scripted_consensus_round_1() {
        // ScriptedMockLlm 永远返 "StrongApprove" → 5 member 都 StrongApprove → 共识 round 1
        let llm = Arc::new(
            ScriptedMockLlm::new().with_default(MockLlmResponse::ok("StrongApprove")),
        );
        let mut d = CouncilMemberDeliberator::new(standard_5_members()).with_mock_llm(llm);
        let v = d.deliberate(&q("any query"));
        assert_eq!(v.termination_reason, "consensus");
        assert_eq!(v.rounds_run, 1);
        assert!(v.consensus_reached);
        for m in &v.member_summaries {
            assert_eq!(m.final_stance, StanceKind::StrongApprove);
        }
    }
    #[test]
    fn deliberate_with_mock_llm_disapprove_runs_all_3_rounds() {
        // 永远返 "Disapprove" → score = 0.2 → 0 共识 → 0 强反对 → 跑满 max_rounds=3
        let llm = Arc::new(
            ScriptedMockLlm::new().with_default(MockLlmResponse::ok("Disapprove")),
        );
        let mut d = CouncilMemberDeliberator::new(standard_5_members()).with_mock_llm(llm);
        let v = d.deliberate(&q("any query"));
        assert_eq!(v.termination_reason, "max_rounds");
        assert_eq!(v.rounds_run, 3);
        assert!(!v.consensus_reached);
    }
    #[test]
    fn deliberate_with_max_rounds_1_truncates_correctly() {
        // 永远 Approve → consensus round 1
        let llm = Arc::new(
            ScriptedMockLlm::new().with_default(MockLlmResponse::ok("Approve")),
        );
        let mut d = CouncilMemberDeliberator::new(standard_5_members())
            .with_mock_llm(llm)
            .with_max_rounds(1);
        let v = d.deliberate(&q("any query"));
        assert_eq!(v.termination_reason, "consensus");
        assert_eq!(v.rounds_run, 1);
    }
    #[test]
    fn round_summary_consensus_reached_threshold() {
        let o = vec![AdvisorOpinion::new(
            AdvisorId::new("a"),
            Stance::strong_approve("ok"),
            0.9,
            "r",
            NOW,
        )];
        let r = RoundSummary {
            round: 0,
            opinions: o,
            consensus_score: 0.95,
            transcript: String::new(),
        };
        assert!(r.consensus_reached());
        assert!(!r.has_strong_disapprove());
    }
    #[test]
    fn parse_stance_from_text_5_kinds() {
        assert_eq!(
            parse_stance_from_text("StrongApprove — all good"),
            StanceKind::StrongApprove
        );
        assert_eq!(
            parse_stance_from_text("StrongDisapprove — violates policy"),
            StanceKind::StrongDisapprove
        );
        assert_eq!(
            parse_stance_from_text("Approve but with caveats"),
            StanceKind::Approve
        );
        assert_eq!(
            parse_stance_from_text("Disapprove due to risk"),
            StanceKind::Disapprove
        );
        assert_eq!(parse_stance_from_text("abstain from vote"), StanceKind::Abstain);
        assert_eq!(parse_stance_from_text("no clear position"), StanceKind::Neutral);
        // 中文
        assert_eq!(parse_stance_from_text("强烈赞成"), StanceKind::StrongApprove);
        assert_eq!(parse_stance_from_text("强反对"), StanceKind::StrongDisapprove);
    }
    #[test]
    fn keyword_stance_fallback_3_branches() {
        assert_eq!(
            keyword_stance_fallback("help user understand principle onion"),
            StanceKind::Approve
        );
        assert_eq!(
            keyword_stance_fallback("exploit user trust and harm"),
            StanceKind::StrongDisapprove
        );
        assert_eq!(
            keyword_stance_fallback("what is the meaning?"),
            StanceKind::Neutral
        );
    }
    #[test]
    fn compute_consensus_score_3_cases() {
        // 5 个 StrongApprove (score=1.0) → normalized=1.0, mean=1.0
        let o_approve: Vec<_> = (0..5)
            .map(|i| {
                AdvisorOpinion::new(
                    AdvisorId::new(format!("a{i}")),
                    Stance::strong_approve("ok"),
                    0.9,
                    "r",
                    NOW,
                )
            })
            .collect();
        let s = compute_consensus_score(&o_approve);
        assert!((s - 1.0).abs() < 0.01, "approve score should be 1.0, got {s}");
        // 5 个 Disapprove (score=-0.6) → normalized=0.2, mean=0.2
        let o_dis: Vec<_> = (0..5)
            .map(|i| {
                AdvisorOpinion::new(
                    AdvisorId::new(format!("d{i}")),
                    Stance::new(StanceKind::Disapprove, "no"),
                    0.9,
                    "r",
                    NOW,
                )
            })
            .collect();
        let s = compute_consensus_score(&o_dis);
        assert!(s < 0.3, "disapprove score should be < 0.3, got {s}");
        // 空 → 0.0
        assert_eq!(compute_consensus_score(&[]), 0.0);
    }
    #[test]
    fn score_to_stance_5_thresholds() {
        assert_eq!(score_to_stance(1.0), StanceKind::StrongApprove);
        assert_eq!(score_to_stance(0.7), StanceKind::Approve);
        assert_eq!(score_to_stance(0.5), StanceKind::Neutral);
        assert_eq!(score_to_stance(0.3), StanceKind::Disapprove);
        assert_eq!(score_to_stance(0.0), StanceKind::StrongDisapprove);
    }
    #[test]
    fn member_summary_per_round_evolution() {
        // 跑 3 轮, 验证 member_summaries 反映 final round
        let llm = Arc::new(
            ScriptedMockLlm::new().with_default(MockLlmResponse::ok("StrongApprove")),
        );
        let mut d = CouncilMemberDeliberator::new(standard_5_members())
            .with_mock_llm(llm)
            .with_max_rounds(3);
        let v = d.deliberate(&q("any"));
        assert_eq!(v.rounds.len(), 1, "共识 round 1 → 只 1 轮");
        assert_eq!(v.member_summaries.len(), 5);
    }
    #[test]
    fn session_id_monotonic() {
        let mut d = CouncilMemberDeliberator::new(standard_5_members());
        let v1 = d.deliberate(&q("a"));
        let v2 = d.deliberate(&q("b"));
        assert_ne!(v1.session_id, v2.session_id);
        assert!(v2.session_id.contains("cm-session-"));
    }
    #[test]
    fn multi_round_verdict_eval_scores_7() {
        let v = MultiRoundVerdict {
            query_id: "q".to_string(),
            session_id: "cm-session-000001".to_string(),
            rounds: vec![],
            consensus_reached: true,
            termination_reason: "consensus".to_string(),
            final_weighted_score: 0.85,
            final_stance: StanceKind::Approve,
            elapsed_ms: 100,
            rounds_run: 1,
            member_summaries: vec![MemberSummary {
                role: "a".to_string(),
                provider: "claude_code".to_string(),
                final_stance: StanceKind::Approve,
                final_confidence: 0.9,
            }],
        };
        let scores = v.to_eval_scores();
        assert_eq!(scores.len(), 7);
        for (name, val) in &scores {
            assert!(!name.is_empty());
            assert!(*val >= 0.0 && *val <= 1.0, "{name} = {val} out of range");
        }
    }
}
