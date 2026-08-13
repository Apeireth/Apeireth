//! Council deliberation — 智囊团审议驱动器
//!
//! **职责**:
//! - 召集 7 强制 advisor
//! - 跑审议 (单轮 + 多轮辩论)
//! - 触发按住评估
//! - 触发主权 hook (`SovereigntyHook::on_council_event`)
//! - 产出 [`CouncilVerdict`]

use crate::advisor::{Advisor, AdvisorOpinion, DeliberationContext, DeliberationOutcome};
use crate::hold::{HoldOutcome, HoldTrigger};
use crate::persona::{DebateRound, PersonaSession};
use crate::sovereignty::{CouncilEvent, NoopSovereigntyHook, SovereigntyHook};
use crate::synthesis::{synthesize, SynthesisReport, SynthesisWeights};
use std::fmt;

/// 默认裁决超时 (60s).
pub const DEFAULT_DELIBERATION_TIMEOUT_MS: u64 = 60_000;

/// 智囊团 query — 提交审议的请求。
#[derive(Debug, Clone)]
pub struct CouncilQuery {
    /// Query ID (全局唯一)
    pub query_id: String,
    /// Query 描述
    pub description: String,
    /// 上下文 (e.g. 动作 / 风险 / 涉及 layer)
    pub context: QueryContext,
    /// 开始时间 (epoch ms)
    pub started_at_ms: i64,
}

/// Query 上下文 (随 query 提交).
#[derive(Debug, Clone, Default)]
pub struct QueryContext {
    /// 涉及领域 (e.g. "L3 key operation")
    pub area: Option<String>,
    /// 风险等级 ("low" / "medium" / "high" / "nuclear")
    pub risk_level: Option<String>,
    /// 引用历史 ID
    pub history_refs: Vec<String>,
}

impl CouncilQuery {
    /// 便利构造.
    pub fn new(
        query_id: impl Into<String>,
        description: impl Into<String>,
        started_at_ms: i64,
    ) -> Self {
        Self {
            query_id: query_id.into(),
            description: description.into(),
            context: QueryContext::default(),
            started_at_ms,
        }
    }

    /// 设置风险等级.
    pub fn with_risk(mut self, risk: impl Into<String>) -> Self {
        self.context.risk_level = Some(risk.into());
        self
    }

    /// 设置区域.
    pub fn with_area(mut self, area: impl Into<String>) -> Self {
        self.context.area = Some(area.into());
        self
    }

    /// 添加历史引用.
    pub fn with_history_ref(mut self, ref_id: impl Into<String>) -> Self {
        self.context.history_refs.push(ref_id.into());
        self
    }
}

/// 智囊团最终裁决.
#[derive(Debug, Clone)]
pub struct CouncilVerdict {
    /// Query ID
    pub query_id: String,
    /// Session ID
    pub session_id: String,
    /// 综合报告
    pub report: SynthesisReport,
    /// 耗时 (ms)
    pub elapsed_ms: u64,
    /// 是否按住
    pub held: bool,
    /// 按住后果 (按住时)
    pub hold_outcome: Option<HoldOutcome>,
}

impl CouncilVerdict {
    /// 是否通过 (allow).
    pub fn is_allowed(&self) -> bool {
        !self.held && self.report.aggregated_stance.kind.score() > 0.0
    }

    /// 是否被拒绝.
    pub fn is_rejected(&self) -> bool {
        self.report.aggregated_stance.kind.score() < 0.0
    }
}

impl fmt::Display for CouncilVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CouncilVerdict(query={}, session={}, held={}, weighted={:.2}, stance={:?}, elapsed={}ms)",
            self.query_id,
            self.session_id,
            self.held,
            self.report.weighted_score,
            self.report.aggregated_stance.kind,
            self.elapsed_ms
        )
    }
}

/// 智囊团 — 召集 + 审议 + synthesis + 按住 + sovereignty hook 调度.
pub struct Council {
    /// 召集的 advisors
    advisors: Vec<Box<dyn Advisor>>,
    /// 自定义 synthesis 权重
    weights: SynthesisWeights,
    /// Sovereignty hooks
    hooks: Vec<Box<dyn SovereigntyHook>>,
    /// session 计数
    next_session_seq: u64,
}

impl Council {
    /// 创建新智囊团 (无 advisor, 需调用 [`Council::recruit`] 召集).
    pub fn new() -> Self {
        Self {
            advisors: Vec::new(),
            weights: SynthesisWeights::default(),
            hooks: Vec::new(),
            next_session_seq: 0,
        }
    }

    /// 召集 advisor.
    pub fn recruit(&mut self, advisor: Box<dyn Advisor>) {
        self.advisors.push(advisor);
    }

    /// 批量召集.
    pub fn recruit_many(&mut self, advisors: Vec<Box<dyn Advisor>>) {
        self.advisors.extend(advisors);
    }

    /// 注册主权 hook.
    pub fn register_hook(&mut self, hook: Box<dyn SovereigntyHook>) {
        self.hooks.push(hook);
    }

    /// 自定义 synthesis 权重.
    pub fn set_weights(&mut self, weights: SynthesisWeights) {
        self.weights = weights;
    }

    /// 当前 advisor 数.
    pub fn advisor_count(&self) -> usize {
        self.advisors.len()
    }

    /// 迭代 advisors (按召集顺序). R218 followup 用: checkpoint 集成.
    pub fn advisors_iter(&self) -> std::slice::Iter<'_, Box<dyn Advisor>> {
        self.advisors.iter()
    }

    /// 取指定 domain 的 synthesis 权重. R218 followup 用.
    pub fn weights_for(&self, domain: crate::advisor::AdvisorDomain) -> f64 {
        self.weights.for_domain(domain)
    }

    /// 复制当前 weights. R218 followup 用: checkpoint 集成需要 synthesis.
    pub fn weights_clone(&self) -> crate::synthesis::SynthesisWeights {
        self.weights.clone()
    }

    /// 审议入口 — 单轮 (非拟人化).
    ///
    /// **流程**:
    /// 1. 生成 session_id
    /// 2. 触发 DeliberationStarted 事件
    /// 3. 每个 advisor 调用 `deliberate`
    /// 4. 收集 opinions
    /// 5. 触发 OpinionIssued 事件 (每个 opinion)
    /// 6. 评估按住 + 触发 HoldTriggered 事件 (若按住)
    /// 7. synthesis 出报告
    /// 8. 触发 DeliberationCompleted 事件
    pub fn deliberate(&mut self, query: CouncilQuery) -> CouncilVerdict {
        let session_id = self.alloc_session_id();
        let started_at_ms = query.started_at_ms;

        // 1. DeliberationStarted event
        self.emit_event(&CouncilEvent::DeliberationStarted {
            session_id: session_id.clone(),
            query_id: query.query_id.clone(),
            started_at_ms,
        });

        // 2. Collect opinions
        let mut opinions = Vec::new();
        let mut ctx = DeliberationContext::new(started_at_ms);

        for advisor in &self.advisors {
            match advisor.deliberate(&query, &mut ctx) {
                Ok(outcome) => {
                    let opinion = outcome.opinion.clone();
                    // 注入权重
                    let weighted = opinion.with_weight(self.weights.for_domain(advisor.domain()));
                    self.emit_event(&CouncilEvent::OpinionIssued {
                        session_id: session_id.clone(),
                        opinion: weighted.clone(),
                    });
                    opinions.push(weighted);
                }
                Err(err) => {
                    // advisor 错误 → 跳过 (模拟 LLM 失败, 但不假装)
                    eprintln!("advisor {} error: {}", advisor.id(), err);
                }
            }
        }

        // 3. Evaluate hold
        let hold_trigger = HoldTrigger::evaluate(&opinions);
        if let Some(trigger) = &hold_trigger {
            self.emit_event(&CouncilEvent::HoldTriggered {
                session_id: session_id.clone(),
                trigger: trigger.clone(),
            });
        }

        // 4. Synthesize
        let report = synthesize(&opinions, &self.weights);

        // 5. Hold outcome
        let held = report.is_held();
        let hold_outcome = if held {
            Some(HoldOutcome::ReflectionStarted {
                reason: format!("按住触发: {:?}", hold_trigger.unwrap().threshold),
                started_at_ms,
            })
        } else {
            None
        };

        // 6. Elapsed time (synthetic — based on started_at_ms vs now())
        let elapsed_ms = (current_time_ms() - started_at_ms).max(0) as u64;

        // 7. DeliberationCompleted event
        self.emit_event(&CouncilEvent::DeliberationCompleted {
            session_id: session_id.clone(),
            report: report.clone(),
            elapsed_ms,
        });

        CouncilVerdict {
            query_id: query.query_id,
            session_id,
            report,
            elapsed_ms,
            held,
            hold_outcome,
        }
    }

    /// 拟人化辩论 — 多轮 persona 辩论.
    ///
    /// **流程**: 同 [`Council::deliberate`], 但每个 advisor 拥有独立 persona session + 3 轮辩论.
    pub fn deliberate_persona(
        &mut self,
        query: CouncilQuery,
        personas: &mut [PersonaSession],
    ) -> CouncilVerdict {
        let session_id = self.alloc_session_id();
        let started_at_ms = query.started_at_ms;

        self.emit_event(&CouncilEvent::DeliberationStarted {
            session_id: session_id.clone(),
            query_id: query.query_id.clone(),
            started_at_ms,
        });

        let mut opinions = Vec::new();
        let mut ctx = DeliberationContext::new(started_at_ms);

        // 每个 advisor 配对一个 persona session
        for (advisor, persona_session) in self.advisors.iter().zip(personas.iter_mut()) {
            // 多轮辩论
            while persona_session.can_debate() {
                let outcome = match advisor.deliberate(&query, &mut ctx) {
                    Ok(o) => o,
                    Err(_) => continue,
                };

                // Craft persona speech
                let speech = persona_session.craft_speech(&outcome.opinion.stance);
                let round = DebateRound {
                    round: persona_session.current_round,
                    outcome,
                    speech,
                };

                // 注入权重
                let mut weighted = round.outcome.opinion.clone();
                weighted = weighted.with_weight(self.weights.for_domain(advisor.domain()));

                self.emit_event(&CouncilEvent::OpinionIssued {
                    session_id: session_id.clone(),
                    opinion: weighted.clone(),
                });

                persona_session.record_round(DebateRound {
                    round: round.round,
                    outcome: DeliberationOutcome {
                        opinion: weighted.clone(),
                        needs_rebuttal: persona_session.can_debate(),
                    },
                    speech: round.speech,
                });
                opinions.push(weighted);
            }
        }

        let hold_trigger = HoldTrigger::evaluate(&opinions);
        if let Some(trigger) = &hold_trigger {
            self.emit_event(&CouncilEvent::HoldTriggered {
                session_id: session_id.clone(),
                trigger: trigger.clone(),
            });
        }

        let report = synthesize(&opinions, &self.weights);
        let held = report.is_held();
        let hold_outcome = if held {
            Some(HoldOutcome::ReflectionStarted {
                reason: format!("按住触发: {:?}", hold_trigger.unwrap().threshold),
                started_at_ms,
            })
        } else {
            None
        };

        let elapsed_ms = (current_time_ms() - started_at_ms).max(0) as u64;

        self.emit_event(&CouncilEvent::DeliberationCompleted {
            session_id: session_id.clone(),
            report: report.clone(),
            elapsed_ms,
        });

        CouncilVerdict {
            query_id: query.query_id,
            session_id,
            report,
            elapsed_ms,
            held,
            hold_outcome,
        }
    }

    /// 触发主权 hook 事件 (供外部主权主动调用).
    pub fn emit_event(&self, event: &CouncilEvent) {
        for hook in &self.hooks {
            hook.on_council_event(event);
        }
    }

    /// 当前注册 hook 数.
    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }

    /// 分配 session ID.
    fn alloc_session_id(&mut self) -> String {
        self.next_session_seq += 1;
        format!("session-{:06}", self.next_session_seq)
    }

    /// 注册默认空主权 hook (用于测试).
    pub fn register_default_hook(&mut self) {
        self.register_hook(Box::new(NoopSovereigntyHook));
    }
}

impl Default for Council {
    fn default() -> Self {
        Self::new()
    }
}

/// 当前时间 (epoch ms) — 测试可注入.
fn current_time_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
