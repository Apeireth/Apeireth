//! P-arch (2026-08-27): A1 council + A2 team-lead + scene-d 例 3 Orchestrator.
//!
//! v2.0 alpha **只**提供 trait 边界 + 0 装占位数据类. 真 LLM 调
//! (council 7 advisor 并行 / orchestrator 调 subagent) 留 v2.0.0-rc 路线
//! (per `v2-unabsorbed-features.md` §A1 + §A2 + `scene-d-v2-plan.md` §3).
//!
//! 借鉴 v1 `apeireth-team-lead` (14 调度工具) + `apeireth-council` (7 advisor
//! + 按住机制) + `apeireth-orchestrator` (plan/impl/review). **v2 形态**:
//! - `Advisor` trait (per-domain LLM 评审; 7 advisor 是 7 个具体 struct)
//! - `Council::decide(proposal) -> CouncilVerdict` (按住 + 多意见加权)
//! - `Orchestrator::dispatch(spec) -> SubagentOutcome` (调 subagent)
//! - `SubagentSpec` (plan/impl/review 等 role + 隔离 LLM 实例 + JSON protocol)
//!
//! **SelfAssessmentCache** (scene-d 例 2 触发) 移到 `apeireth-plugin::self_assessment` 模块
//! (canonical 单 source of truth, per 子代理审查 1.2, 2026-08-27). 本 crate 0 重复定义.
//!
//! **0 装 PASS (v2.0 alpha)**:
//! - `Council::default()` 返 7 个 no-op advisor (全 Allow); 真 council
//!   (调 LLM 7x 并行) 在 v2.0.0-rc
//! - `Orchestrator` trait 0 装: 真正调 subagent 需 runtime 介入 (subagent
//!   是 LLM factory 独立实例, 不在本 crate 范围)
//! - 全部 `pub use` 都是 re-export; 不引入新 LLM dep
//!
//! **架构原则**:
//! - trait 在 foundation (跟 plugin / governance / credentials 同级)
//! - impl 留 v2.0.0-rc (runtime 调 LLM, 不在本 crate)
//! - 与 v1 的 14 调度工具 API 1:1 对应, 但 trait 化让 impl 可换 (CLI / TUI / HTTP 都走同 trait)
//! - 多 instance 隔离 (per scene-d §3): subagent 必须是 LLM factory 独立实例, 禁共享 prompt
//!
//! 0 触碰 LOCKED: 8 哲学锚 / 13 键 / 3 项不可变脊柱 / workspace.version / R11 baseline
//!   0 改; 新增 0 触碰现有任何 crate.

#![forbid(unsafe_code)]

use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

// ============================================
// Council (A1)
// ============================================

/// 7 个 advisor 领域 (per v1 apeireth-council + 设计意图)
/// 与 13 键原则洋葱的 S (价值层) 对应.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdvisorKind {
    /// 安全性 (PII / 注入 / 凭据泄漏 / 自我禁用)
    Safety,
    /// 性能 (延迟 / 成本 / token budget)
    Performance,
    /// 哲学对齐 (13 键原则 / 三洋葱)
    Philosophy,
    /// 历史 (类似决策的历史结果)
    History,
    /// 策略 (主人长期目标)
    Strategy,
    /// 伦理 (主人显式表达的伦理边界)
    Ethics,
    /// 法律 (合规 / 区域)
    Legal,
}

/// 单个 advisor 的输出
#[derive(Debug, Clone, PartialEq)]
pub enum AdvisorVerdict {
    /// 同意
    Allow,
    /// 反对 (附理由)
    Deny { reason: String },
    /// 弃权 (不参与投票)
    Abstain,
}

/// Advisor trait: 每个领域 1 个 LLM 调用
#[async_trait::async_trait]
pub trait Advisor: Send + Sync {
    /// 稳定名
    fn name(&self) -> &'static str;
    /// 类型
    fn kind(&self) -> AdvisorKind;
    /// 评审 (v2.0 alpha: 0 装, 返硬编码; v2.0.0-rc: 调 LLM 独立实例)
    async fn evaluate(&self, proposal: &Proposal) -> AdvisorVerdict;
}

/// 提案: 多领域评审对象 (per v1: 行动 / 提案 / 演化)
#[derive(Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// 提案 id
    pub id: String,
    /// 提交者 (subagent 或 runtime)
    pub proposer: String,
    /// 提案内容 (JSON 序列化, 0 装 不约束 schema)
    pub payload: serde_json::Value,
    /// 提交时间
    pub submitted_at: i64,
    /// 关联 session
    pub session_id: SessionId,
}

/// Council 多领域评审 (按住机制 + 加权投票)
pub struct Council {
    advisors: Vec<Arc<dyn Advisor>>,
}

/// Council 决策 (含按住机制, per v1 stage4-correction-v15)
#[derive(Debug, Clone, PartialEq)]
pub enum CouncilVerdict {
    /// 通过 (多数 Allow, 无强反对)
    Approved,
    /// 拒绝 (含按住: 30% Advisor 反对 / 一致反对)
    Vetoed {
        by: AdvisorKind,
        reason: String,
    },
    /// 需人工批准 (v2 治理的 Deny vs RequireApproval 区分, per ROADMAP P0)
    /// 触发条件: 60s 内达不成 consensus (v1 stage4) — 0 装: 永不触发
    DeferToHuman { reason: String },
}

impl Council {
    /// 7 个 advisor 默认 (0 装: 全 Allow, 用于 v2.0 alpha 无 LLM harness 场景)
    pub fn default_allow() -> Self {
        Self {
            advisors: vec![
                Arc::new(NoopAdvisor::new(AdvisorKind::Safety)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Performance)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Philosophy)),
                Arc::new(NoopAdvisor::new(AdvisorKind::History)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Strategy)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Ethics)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Legal)),
            ],
        }
    }

    /// 自定义 advisors
    pub fn new(advisors: Vec<Arc<dyn Advisor>>) -> Self {
        Self { advisors }
    }

    /// 多意见加权: 任一 advisor Deny (除 Abstain) 即触发 Veto
    /// (v1 30% 强反对 / 一致反对 / 60s 裁决超时的 v2 简化: 一票否决制 + Abstain 跳过)
    /// **0 装 PASS**: 真实 council 是 7 个 LLM 并行调用 + 加权 + 60s 超时, v2.0.0-rc.
    ///
    /// **v2.0.0-rc contract** (per `v2.0.0-rc-roadmap.md` §2.4 + scene-d §2):
    /// - `decide` 内部走 `tokio::time::timeout(Duration::from_secs(60), futures::future::join_all(7 LLM calls))`
    /// - 任一 advisor 失败 → 该 advisor 视为 Abstain (不算 30% 阈值)
    /// - 任一 advisor Deny → `Vetoed { by, reason }`
    /// - 全 Allow/Abstain → `Approved`
    /// - 60s 触发 → `DeferToHuman { reason: "60s no consensus" }`
    /// - runtime `DeferToHuman` 路径 → 转 `RequireApproval` (governance 已装, 直接复用 `8732857` wiring)
    ///
    /// **alpha 0 装**: 当前同步遍历 7 advisors, 没 60s timeout, 失败即 Deny (无 Abstain 路径).
    /// rc 阶段改: `tokio::spawn` 7 个并发 LLM 调用 + `futures::future::join_all` + `tokio::time::timeout`.
    pub async fn decide(&self, proposal: &Proposal) -> CouncilVerdict {
        // 0 装: 不调 LLM, 不并发 (trait 是 async 但 v0 impl 同步返 Allow)
        // 真 council 在 v2.0.0-rc 改: futures::future::join_all 调 7 个 advisor
        // + 60s timeout + 30% veto rule
        for advisor in &self.advisors {
            let verdict = advisor.evaluate(proposal).await;
            if let AdvisorVerdict::Deny { reason } = verdict {
                return CouncilVerdict::Vetoed {
                    by: advisor.kind(),
                    reason,
                };
            }
            // Allow | Abstain: continue (语义清晰: veto 优先, Abstain 不计)
        }
        CouncilVerdict::Approved
    }
}

/// Noop advisor (v2.0 alpha 0 装)
struct NoopAdvisor {
    kind: AdvisorKind,
}

impl NoopAdvisor {
    fn new(kind: AdvisorKind) -> Self {
        Self { kind }
    }
}

#[async_trait::async_trait]
impl Advisor for NoopAdvisor {
    fn name(&self) -> &'static str {
        match self.kind {
            AdvisorKind::Safety => "noop_safety",
            AdvisorKind::Performance => "noop_performance",
            AdvisorKind::Philosophy => "noop_philosophy",
            AdvisorKind::History => "noop_history",
            AdvisorKind::Strategy => "noop_strategy",
            AdvisorKind::Ethics => "noop_ethics",
            AdvisorKind::Legal => "noop_legal",
        }
    }

    fn kind(&self) -> AdvisorKind {
        self.kind
    }

    async fn evaluate(&self, _proposal: &Proposal) -> AdvisorVerdict {
        // 0 装: 全 Allow (Noop)
        AdvisorVerdict::Allow
    }
}

// ============================================
// Orchestrator (A2 + scene-d 例 3)
// ============================================

/// Subagent 角色 (v1 三件套 + 扩展)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubagentRole {
    /// 规划 (把长程任务拆 plan)
    Planner,
    /// 实施 (按 plan 写代码)
    Implementer,
    /// 评审 (互审核心: 独立 LLM 评审其他 agent 输出)
    Reviewer,
    /// 测试 (Plan/Impl 后跑测试)
    Tester,
    /// 文档
    Documenter,
}

/// Subagent 规格 (per v1 team-lead 14 调度工具, trait 化)
#[derive(Clone, Serialize, Deserialize)]
pub struct SubagentSpec {
    /// 唯一 id
    pub id: String,
    /// 角色
    pub role: SubagentRole,
    /// 任务标题
    pub title: String,
    /// 任务详细 (JSON 序列化)
    pub payload: serde_json::Value,
    /// 用哪个 model (per scene-d §5 决策 1: 同 provider 不同 model 隔离)
    /// None = provider 默认 model
    pub model: Option<String>,
    /// 隔离的 system prompt
    pub system_prompt: Option<String>,
    /// 最多允许的 governance 决策: RequireApproval 自动 deny
    pub require_human_approval: bool,
}

/// Subagent 输出结果
#[derive(Clone, Serialize, Deserialize)]
pub struct SubagentOutcome {
    /// 对应 spec.id
    pub spec_id: String,
    /// 角色
    pub role: SubagentRole,
    /// 输出 (JSON 序列化, 角色-specific)
    pub output: serde_json::Value,
    /// 是否成功
    pub success: bool,
    /// 错误信息 (success=false 时)
    pub error: Option<String>,
    /// 完成时间
    pub completed_at: i64,
}

/// 互审结果 (scene-d 例 3)
#[derive(Clone, Serialize, Deserialize)]
pub struct ReviewVerdict {
    /// 评审的 subagent output id
    pub reviewed_spec_id: String,
    /// 通过 / 需返工
    pub passed: bool,
    /// 评分 (0.0-1.0)
    pub score: f64,
    /// 阻塞性问题 (passed=false 时必填)
    pub blocking_issues: Vec<String>,
    /// 可选建议 (passed=true 也可填)
    pub suggestions: Vec<String>,
    /// 评审者 subagent id
    pub reviewer_id: String,
}

/// Orchestrator trait: 调度 subagent (per v1 team-lead 14 工具 trait 化)
#[async_trait::async_trait]
pub trait Orchestrator: Send + Sync {
    /// 启动 orchestrator (装配期)
    async fn start(&mut self) -> Result<(), OrchestratorError>;
    /// 停止
    async fn stop(&mut self) -> Result<(), OrchestratorError>;
    /// 分发 subagent (async 调 LLM factory 独立实例, scene-d §3 多 instance 隔离)
    async fn dispatch(&self, spec: SubagentSpec) -> Result<SubagentOutcome, OrchestratorError>;
    /// 长程任务: plan + impl + review 串联
    async fn orchestrate(
        &self,
        title: String,
        payload: serde_json::Value,
    ) -> Result<Vec<SubagentOutcome>, OrchestratorError> {
        // 默认实现: planner -> implementer -> reviewer 三步
        // clone payload 防 partial move (serde_json::Value not Copy)
        let plan = self
            .dispatch(SubagentSpec {
                id: format!("{title}-plan"),
                role: SubagentRole::Planner,
                title: format!("{title} (plan)"),
                payload: payload.clone(),
                model: None,
                system_prompt: None,
                require_human_approval: true, // 重大任务先给主人看 plan
            })
            .await?;
        let plan_output = plan.output.clone();
        let impl_outcome = self
            .dispatch(SubagentSpec {
                id: format!("{title}-impl"),
                role: SubagentRole::Implementer,
                title: format!("{title} (impl)"),
                payload: plan_output,
                model: None,
                system_prompt: None,
                require_human_approval: false,
            })
            .await?;
        let impl_output = impl_outcome.output.clone();
        let review = self
            .dispatch(SubagentSpec {
                id: format!("{title}-review"),
                role: SubagentRole::Reviewer,
                title: format!("{title} (review)"),
                payload: impl_output,
                model: None, // 关键: reviewer 必须用 **不同 model** (scene-d §5 决策 1)
                system_prompt: None,
                require_human_approval: false,
            })
            .await?;
        Ok(vec![plan, impl_outcome, review])
    }
}

/// Orchestrator 错误
#[derive(Debug)]
pub enum OrchestratorError {
    /// 0 装: subagent 启动失败
    SubagentFailed(String),
    /// 0 装: 主人审批拒绝
    HumanDenied(String),
    /// governance 拒绝 (runtime 拒绝给 subagent 该 capability)
    GovernanceDenied(String),
    /// IO 错误
    Io(String),
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SubagentFailed(msg) => write!(f, "subagent failed: {msg}"),
            Self::HumanDenied(msg) => write!(f, "human denied: {msg}"),
            Self::GovernanceDenied(msg) => write!(f, "governance denied: {msg}"),
            Self::Io(msg) => write!(f, "io error: {msg}"),
        }
    }
}

impl std::error::Error for OrchestratorError {}

// Send / Sync 由编译器自动派生: 字段全 String, 满足 Send + Sync 自动推导条件.
// crate 内 #![forbid(unsafe_code)] 不允许 unsafe impl, 但自动派生无需 unsafe impl.
// RC-5 runtime 集成时跨任务传输安全.

// SelfAssessmentCache (scene-d 例 2) **0 在此 crate** (per 子代理审查 1.2):
// 之前这里有 `SelfAssessment` / `Deviation` / `SelfAssessmentCache` 同名不同定义
// (plugin::SelfAssessment 字段多, 含 id + reviewer_id, 比本处的字段全).
// **单一 source of truth**: plugin::self_assessment::SelfAssessment 是 canonical
// 类型, plugin::self_assessment::SelfAssessmentStore 是 trait.
// Orchestration 改为注入 `Arc<dyn SelfAssessmentStore>`, 0 重复定义.

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::SessionId;

    fn sample_proposal() -> Proposal {
        Proposal {
            id: "p-1".into(),
            proposer: "runtime".into(),
            payload: serde_json::json!({"action": "deploy"}),
            submitted_at: 1_700_000_000,
            session_id: SessionId::new(),
        }
    }

    #[tokio::test]
    async fn council_default_approves_everything() {
        let c = Council::default_allow();
        let v = c.decide(&sample_proposal()).await;
        assert_eq!(v, CouncilVerdict::Approved);
    }

    #[tokio::test]
    async fn council_vetoes_on_any_deny() {
        struct DenyAll;
        #[async_trait::async_trait]
        impl Advisor for DenyAll {
            fn name(&self) -> &'static str {
                "deny_all"
            }
            fn kind(&self) -> AdvisorKind {
                AdvisorKind::Safety
            }
            async fn evaluate(&self, _: &Proposal) -> AdvisorVerdict {
                AdvisorVerdict::Deny {
                    reason: "test veto".into(),
                }
            }
        }
        let c = Council::new(vec![
            Arc::new(DenyAll),
            Arc::new(NoopAdvisor::new(AdvisorKind::Safety)),
        ]);
        match c.decide(&sample_proposal()).await {
            CouncilVerdict::Vetoed { by, reason } => {
                assert_eq!(by, AdvisorKind::Safety);
                assert!(reason.contains("test veto"));
            }
            _ => panic!("expected Vetoed"),
        }
    }

    // 注: SelfAssessment / SelfAssessmentCache 测试在 `apeireth-plugin` crate 的
    // `self_assessment` 模块 (canonical 单 source of truth), 不在本 crate 重复.

    #[test]
    fn subagent_spec_roundtrip_json() {
        let spec = SubagentSpec {
            id: "spec-1".into(),
            role: SubagentRole::Planner,
            title: "design".into(),
            payload: serde_json::json!({"x": 1}),
            model: Some("anthropic/claude-3-5".into()),
            system_prompt: Some("you are planner".into()),
            require_human_approval: true,
        };
        let json = serde_json::to_string(&spec).unwrap();
        let back: SubagentSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, spec.id);
        assert_eq!(back.role, spec.role);
        assert_eq!(back.require_human_approval, true);
    }

    #[test]
    fn review_verdict_roundtrip_json() {
        let v = ReviewVerdict {
            reviewed_spec_id: "impl-1".into(),
            passed: false,
            score: 0.4,
            blocking_issues: vec!["missing test".into()],
            suggestions: vec!["add unit test".into()],
            reviewer_id: "reviewer-A".into(),
        };
        let json = serde_json::to_string(&v).unwrap();
        let back: ReviewVerdict = serde_json::from_str(&json).unwrap();
        assert_eq!(back.passed, false);
        assert_eq!(back.score, 0.4);
        assert_eq!(back.blocking_issues, vec!["missing test".to_string()]);
    }
}
