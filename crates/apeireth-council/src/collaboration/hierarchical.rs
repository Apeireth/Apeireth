//! Hierarchical 模式 — 主 + 子 advisor 委派 (OpenAI swarm handoff 借鉴)
//!
//! **职责** (per v2.0 strategy §2B "Hierarchical"):
//! - 1 root advisor 拆 query 为 2 个 sub-task (硬编码默认, 0 LLM)
//! - 2 sub advisors 各自 1 opinion
//! - root 收集 → synthesize() → final verdict
//!
//! **借鉴锚** (S-1):
//! - OpenAI `swarm` handoff pattern (主 agent 委派子 agent)
//! - LangGraph `subgraph` (子图作为节点)
//! - AutoGen `GroupChatManager` 主从结构
//! - VCP `vcpLoop` 嵌套 task
//!
//! **0 漂移**:
//! - 0 改 R10 `synthesize()` (直接复用)
//! - 0 引入 I/O / 网络 / 外部 LLM HTTP
//! - 0 改 R10 `Council::deliberate`

use crate::advisor::{AdvisorId, AdvisorOpinion, Stance, StanceKind};
use crate::collaboration::types::{CollaborationContext, CollaborationMode, CollaborationVerdict};
use crate::deliberation::CouncilQuery;
use crate::synthesis::{synthesize, SynthesisWeights};
use serde::{Deserialize, Serialize};

/// Sub-task (root 委派给 sub-advisor)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelegatedTask {
    /// sub-task ID (e.g. "sub-1", "sub-2")
    pub task_id: String,
    /// sub-advisor role
    pub sub_role: String,
    /// 委派描述 (root 给 sub 的指令)
    pub instruction: String,
}

impl DelegatedTask {
    /// 便利构造
    pub fn new(
        task_id: impl Into<String>,
        sub_role: impl Into<String>,
        instruction: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            sub_role: sub_role.into(),
            instruction: instruction.into(),
        }
    }
}

/// Hierarchical 模式驱动器 — root + 2 sub (per 派活单 "主 + 2 子" 提示)
pub struct HierarchicalMode {
    /// root advisor role
    root_role: String,
    /// 2 sub-advisor roles (per 派活单决策权: 默认 ["技术方案", "风险评估"])
    sub_roles: Vec<String>,
    /// synthesis 权重
    weights: SynthesisWeights,
    /// 内部 session 序列
    next_session_seq: u64,
}

impl HierarchicalMode {
    /// 构造 (默认 2 sub_roles = ["技术方案", "风险评估"], 跟 v2.0 strategy §2B 决策对齐)
    pub fn new(root_role: impl Into<String>) -> Self {
        Self {
            root_role: root_role.into(),
            sub_roles: vec!["技术方案".to_string(), "风险评估".to_string()],
            weights: SynthesisWeights::default(),
            next_session_seq: 0,
        }
    }

    /// 自定义 sub_roles (chainable, 必须 ≥ 1)
    pub fn with_sub_roles(mut self, roles: Vec<String>) -> Self {
        if !roles.is_empty() {
            self.sub_roles = roles;
        }
        self
    }

    /// 自定义 weights (chainable)
    pub fn with_weights(mut self, weights: SynthesisWeights) -> Self {
        self.weights = weights;
        self
    }

    /// root role
    pub fn root_role(&self) -> &str {
        &self.root_role
    }

    /// sub roles 数
    pub fn sub_count(&self) -> usize {
        self.sub_roles.len()
    }

    /// **Root 拆 query 为 N 个 sub-task** (硬编码模板, 0 LLM)
    ///
    /// **拆解模板** (per root_role + sub_role):
    /// - sub 1: "{sub_role} 第 1 阶段: 收集信息"  (per query)
    /// - sub 2: "{sub_role} 第 2 阶段: 评估 {query}"
    /// - sub N (如果 > 2): "{sub_role} 第 N 阶段: 综合"
    ///
    /// **0 漂移**: 0 引入 LLM, 0 假装"root 真接 LLM" (per 主人偏好 #3 "0 假装")
    pub fn delegate(&self, query: &CouncilQuery) -> Vec<DelegatedTask> {
        self.sub_roles
            .iter()
            .enumerate()
            .map(|(i, sub_role)| {
                let instruction = match i {
                    0 => format!(
                        "Root {} 委派给 sub #{} ({}): 收集并分析 query 上下文: {}",
                        self.root_role,
                        i + 1,
                        sub_role,
                        query.description
                    ),
                    1 => format!(
                        "Root {} 委派给 sub #{} ({}): 评估 query 风险: {}",
                        self.root_role,
                        i + 1,
                        sub_role,
                        query.description
                    ),
                    _ => format!(
                        "Root {} 委派给 sub #{} ({}): 综合第 1/2 阶段结论: {}",
                        self.root_role,
                        i + 1,
                        sub_role,
                        query.description
                    ),
                };
                DelegatedTask::new(format!("sub-{}", i + 1), sub_role.clone(), instruction)
            })
            .collect()
    }

    /// **Sub-advisor 跑 1 个 delegated task → 1 opinion**
    pub fn sub_execute(&self, task: &DelegatedTask, query: &CouncilQuery) -> AdvisorOpinion {
        let stance_kind = keyword_stance_for_sub(&task.instruction, &query.description);
        let stance = Stance::new(
            stance_kind,
            format!("Sub {} (root: {})", task.sub_role, self.root_role),
        );
        AdvisorOpinion::new(
            AdvisorId::new(format!("hier-{}-{}", task.task_id, self.root_role)),
            stance,
            0.7, // 默认 confidence (跟 R33-4-1 0 LLM 路径一致)
            format!(
                "Sub-advisor {} 处理 task {} (root {}): {}",
                task.sub_role, task.task_id, self.root_role, task.instruction
            ),
            query.started_at_ms,
        )
        .with_weight(0.85) // sub-advisor 默认权重略高于 executor
    }

    /// **Root 汇总 sub opinions → final verdict** (复用 R10 `synthesize()`)
    pub fn aggregate(&self, sub_opinions: &[AdvisorOpinion]) -> crate::synthesis::SynthesisReport {
        synthesize(sub_opinions, &self.weights)
    }

    /// **跑完整 Hierarchical 流程**
    ///
    /// **流程**:
    /// 1. root 拆 N 个 sub-task
    /// 2. N sub-advisor 各自 1 opinion
    /// 3. 任意 sub 触发 strong_disapprove (confidence ≥ 0.5) → 终止, termination_reason = "strong_disapprove"
    /// 4. 全部 sub opinions → synthesize() → final verdict
    pub fn run(&mut self, query: &CouncilQuery) -> CollaborationVerdict {
        self.next_session_seq += 1;
        let started_at_ms = query.started_at_ms;
        let ctx = CollaborationContext::new(
            CollaborationMode::Hierarchical,
            query.clone(),
            self.next_session_seq,
            started_at_ms,
        );

        let tasks = self.delegate(query);
        let mut opinions: Vec<AdvisorOpinion> = Vec::new();
        let mut termination_reason = "delegation_completed".to_string();
        let mut steps_run: u32 = 0;

        for (i, task) in tasks.iter().enumerate() {
            let opinion = self.sub_execute(task, query);
            if opinion.triggers_hold() {
                opinions.push(opinion);
                termination_reason = "strong_disapprove".to_string();
                steps_run = (i + 1) as u32;
                break;
            }
            opinions.push(opinion);
            steps_run = (i + 1) as u32;
        }

        let report = self.aggregate(&opinions);
        let elapsed_ms = (current_time_ms() - started_at_ms).max(0) as u64;

        CollaborationVerdict {
            session_id: ctx.session_id,
            mode: ctx.mode,
            query_id: query.query_id.clone(),
            report,
            opinions,
            steps: steps_run,
            elapsed_ms,
            termination_reason,
        }
    }
}

/// Keyword 兜底 stance (per R33-4-1 `keyword_stance_fallback` 1:1 镜像, 0 改 R33-4-1)
fn keyword_stance_for_sub(instruction: &str, query_desc: &str) -> StanceKind {
    let combined = format!(
        "{} {}",
        instruction.to_lowercase(),
        query_desc.to_lowercase()
    );
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
    if negative.iter().any(|k| combined.contains(k)) {
        StanceKind::StrongDisapprove
    } else if combined.contains("风险") || combined.contains("risk") || combined.contains("unsafe")
    {
        // 风险评估 sub 看到 risk 关键词 → Disapprove (风险存在)
        StanceKind::Disapprove
    } else {
        StanceKind::Approve
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
// 单元测试
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::deliberation::CouncilQuery;

    fn q(desc: &str) -> CouncilQuery {
        CouncilQuery::new("q-hier-test", desc, 0)
    }

    #[test]
    fn new_default_2_sub_roles() {
        let hm = HierarchicalMode::new("cto");
        assert_eq!(hm.root_role(), "cto");
        assert_eq!(hm.sub_count(), 2);
        assert_eq!(hm.sub_roles[0], "技术方案");
        assert_eq!(hm.sub_roles[1], "风险评估");
    }

    #[test]
    fn with_sub_roles_3_custom() {
        let hm = HierarchicalMode::new("cto").with_sub_roles(vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        ]);
        assert_eq!(hm.sub_count(), 3);
    }

    #[test]
    fn with_sub_roles_empty_keeps_default() {
        let hm = HierarchicalMode::new("cto").with_sub_roles(vec![]);
        assert_eq!(hm.sub_count(), 2); // 保持默认
    }

    #[test]
    fn delegate_default_2_tasks() {
        let hm = HierarchicalMode::new("cto");
        let tasks = hm.delegate(&q("design auth system"));
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].task_id, "sub-1");
        assert_eq!(tasks[0].sub_role, "技术方案");
        assert_eq!(tasks[1].task_id, "sub-2");
        assert_eq!(tasks[1].sub_role, "风险评估");
    }

    #[test]
    fn delegate_3_custom_sub_roles_3_tasks() {
        let hm = HierarchicalMode::new("cto").with_sub_roles(vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        ]);
        let tasks = hm.delegate(&q("test"));
        assert_eq!(tasks.len(), 3);
        assert!(tasks[0].instruction.contains("收集并分析"));
        assert!(tasks[1].instruction.contains("评估"));
        assert!(tasks[2].instruction.contains("综合"));
    }

    #[test]
    fn sub_execute_normal_query_returns_approve() {
        let hm = HierarchicalMode::new("cto");
        let task = DelegatedTask::new("sub-1", "技术方案", "design auth");
        let opinion = hm.sub_execute(&task, &q("design auth system"));
        // 子 1 (技术方案) 看到 design 关键词 → Approve
        assert_eq!(opinion.stance.kind, StanceKind::Approve);
        assert_eq!(opinion.confidence, 0.7);
        assert_eq!(opinion.weight, 0.85);
    }

    #[test]
    fn sub_execute_risk_query_returns_disapprove() {
        let hm = HierarchicalMode::new("cto");
        let task = DelegatedTask::new("sub-2", "风险评估", "评估 query 风险: deploy risky feature");
        let opinion = hm.sub_execute(&task, &q("deploy risky feature"));
        // 子 2 (风险评估) 看到 risk 关键词 → Disapprove
        assert_eq!(opinion.stance.kind, StanceKind::Disapprove);
    }

    #[test]
    fn sub_execute_harm_query_returns_strong_disapprove() {
        let hm = HierarchicalMode::new("bad");
        let task = DelegatedTask::new("sub-1", "技术方案", "Step 1: do harm");
        let opinion = hm.sub_execute(&task, &q("exploit users"));
        assert_eq!(opinion.stance.kind, StanceKind::StrongDisapprove);
    }

    #[test]
    fn aggregate_produces_synthesis_report() {
        let hm = HierarchicalMode::new("cto");
        let opinions: Vec<AdvisorOpinion> = vec![
            hm.sub_execute(&DelegatedTask::new("sub-1", "a", "do thing"), &q("design")),
            hm.sub_execute(&DelegatedTask::new("sub-2", "b", "evaluate"), &q("design")),
        ];
        let report = hm.aggregate(&opinions);
        assert_eq!(report.opinion_count, 2);
    }

    #[test]
    fn run_default_2_subs_approve() {
        let mut hm = HierarchicalMode::new("cto");
        let verdict = hm.run(&q("design auth system"));
        assert_eq!(verdict.mode, CollaborationMode::Hierarchical);
        assert_eq!(verdict.steps, 2);
        assert_eq!(verdict.termination_reason, "delegation_completed");
        assert_eq!(verdict.opinions.len(), 2);
        assert!(verdict.session_id.starts_with("collab-hierarchical-"));
    }

    #[test]
    fn run_terminates_at_strong_disapprove() {
        let mut hm = HierarchicalMode::new("bad");
        let verdict = hm.run(&q("deploy something that will harm users"));
        assert_eq!(verdict.termination_reason, "strong_disapprove");
        assert!(verdict.steps <= 2);
    }

    #[test]
    fn run_session_seq_monotonic() {
        let mut hm = HierarchicalMode::new("a");
        let v1 = hm.run(&q("a"));
        let v2 = hm.run(&q("b"));
        assert!(v2.session_id > v1.session_id);
    }

    #[test]
    fn run_3_subs_3_steps() {
        let mut hm = HierarchicalMode::new("cto").with_sub_roles(vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        ]);
        let verdict = hm.run(&q("design system"));
        assert_eq!(verdict.steps, 3);
        assert_eq!(verdict.opinions.len(), 3);
    }

    #[test]
    fn delegated_task_new_basic() {
        let dt = DelegatedTask::new("sub-1", "技术方案", "design");
        assert_eq!(dt.task_id, "sub-1");
        assert_eq!(dt.sub_role, "技术方案");
        assert_eq!(dt.instruction, "design");
    }

    #[test]
    fn delegated_task_serde_round_trip() {
        let dt = DelegatedTask::new("sub-1", "技术方案", "design");
        let json = serde_json::to_string(&dt).unwrap();
        let back: DelegatedTask = serde_json::from_str(&json).unwrap();
        assert_eq!(dt, back);
    }
}
