//! Planner+Executor 模式 (LangGraph PlanAndExecute 1:1 借鉴)
//!
//! **流程** (per AutoGen planner/executor 角色分离 + LangGraph plan_node + execute_node):
//! ```text
//! 1. Planner (1 role) 拆 query 为 N 步 SubTask (D-3 keyword 简化, 0 LLM)
//! 2. Executor (per SubTask) 顺序跑 N 步, 每步产 1 opinion
//! 3. 任意 step 触发 strong_disapprove → 终止
//! 4. 全部 opinions → synthesize() 出 verdict
//! ```
//!
//! **借鉴锚** (S-1):
//! - LangGraph `PlanAndExecute.plan_node` + `execute_node` (sequential, state-passing)
//! - AutoGen `GroupChat` planner role + executor role separation
//! - VCP `vcpLoop` task decomposition
//!
//! **0 漂移**:
//! - 0 改 R33-4 `council_member.rs` (Planner/Executor 仅用 `Advisor` trait + `synthesize()`)
//! - 0 改 R10 `synthesize()` (直接 `pub use` 复用)
//! - 0 引入 I/O / 网络 / 外部 LLM HTTP
//! - 0 引入新 dep (走 `crate::advisor::AdvisorOpinion` + `crate::synthesis::synthesize`)

use crate::advisor::{AdvisorId, AdvisorOpinion, Stance, StanceKind};
use crate::collaboration::types::{CollaborationContext, CollaborationMode, CollaborationVerdict};
use crate::deliberation::CouncilQuery;
use crate::synthesis::{synthesize, SynthesisWeights};
use serde::{Deserialize, Serialize};

/// 单步 SubTask (planner 产出)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubTask {
    /// 步号 (0-based)
    pub step: u32,
    /// 子任务角色 (e.g. "design", "implement", "deploy")
    pub role: String,
    /// 子任务描述
    pub description: String,
}

impl SubTask {
    /// 便利构造
    pub fn new(step: u32, role: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            step,
            role: role.into(),
            description: description.into(),
        }
    }
}

/// Planner+Executor 模式驱动器
///
/// **构造**:
/// - `PlannerExecutor::new("architect")` — planner_role + 3 default executor roles
/// - `.with_executor_roles(vec![...])` — 自定义 executor roles
pub struct PlannerExecutor {
    /// planner role (1 个)
    planner_role: String,
    /// executor roles (per SubTask 派 1 个)
    executor_roles: Vec<String>,
    /// synthesis 权重 (per 7 强制域默认)
    weights: SynthesisWeights,
    /// 内部 session 序列
    next_session_seq: u64,
}

impl PlannerExecutor {
    /// 构造 (默认 executor_roles = ["design", "implement", "verify"])
    pub fn new(planner_role: impl Into<String>) -> Self {
        Self {
            planner_role: planner_role.into(),
            executor_roles: vec!["design".to_string(), "implement".to_string(), "verify".to_string()],
            weights: SynthesisWeights::default(),
            next_session_seq: 0,
        }
    }

    /// 自定义 executor roles (chainable, 数量跟 planner 拆的 N 步一致)
    pub fn with_executor_roles(mut self, roles: Vec<String>) -> Self {
        self.executor_roles = roles;
        self
    }

    /// 自定义 synthesis 权重 (chainable)
    pub fn with_weights(mut self, weights: SynthesisWeights) -> Self {
        self.weights = weights;
        self
    }

    /// planner_role
    pub fn planner_role(&self) -> &str {
        &self.planner_role
    }

    /// executor_roles 数
    pub fn executor_role_count(&self) -> usize {
        self.executor_roles.len()
    }

    /// **Planner 拆 query 为 N 步 SubTask** (D-3 keyword 简化, 0 LLM)
    ///
    /// **拆解规则** (per query description 关键词匹配):
    /// - "deploy" / "release" / "上线" → ["design", "implement", "deploy"]
    /// - "design" / "设计" / "架构" → ["analyze", "propose", "review"]
    /// - "test" / "测试" / "verify" → ["spec", "implement_test", "verify"]
    /// - "fix" / "bug" / "修复" → ["reproduce", "diagnose", "fix"]
    /// - 其他 → 默认 ["analyze", "execute", "verify"]
    ///
    /// **0 漂移**: 0 引入 LLM, 0 假装"planner 真接 LLM"
    /// (per 主人偏好 #3 "0 假装" + 0 I/O 哲学锚)
    pub fn plan(&self, query: &CouncilQuery) -> Vec<SubTask> {
        let desc_lower = query.description.to_lowercase();
        let step_names: Vec<&str> = if desc_lower.contains("deploy")
            || desc_lower.contains("release")
            || desc_lower.contains("上线")
            || desc_lower.contains("发布")
        {
            vec!["design", "implement", "deploy"]
        } else if desc_lower.contains("design")
            || desc_lower.contains("设计")
            || desc_lower.contains("架构")
            || desc_lower.contains("architecture")
        {
            vec!["analyze", "propose", "review"]
        } else if desc_lower.contains("test")
            || desc_lower.contains("测试")
            || desc_lower.contains("verify")
            || desc_lower.contains("验证")
        {
            vec!["spec", "implement_test", "verify"]
        } else if desc_lower.contains("fix")
            || desc_lower.contains("bug")
            || desc_lower.contains("修复")
            || desc_lower.contains("错误")
        {
            vec!["reproduce", "diagnose", "fix"]
        } else {
            vec!["analyze", "execute", "verify"]
        };

        // 优先用 executor_roles, 但用 plan 拆的 step_names 截断/填充到 3 步
        let n = step_names.len();
        step_names
            .into_iter()
            .enumerate()
            .map(|(i, step_name)| {
                let role = self
                    .executor_roles
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| step_name.to_string());
                SubTask::new(
                    i as u32,
                    role,
                    format!(
                        "Step {}/{} (planner={}): {}",
                        i + 1,
                        n,
                        self.planner_role,
                        step_name
                    ),
                )
            })
            .collect()
    }

    /// **Executor 跑 1 个 SubTask → 1 opinion** (复用 R33-4-1 keyword fallback 逻辑)
    ///
    /// **0 漂移**: keyword_stance_for_step 跟 R33-4-1 `keyword_stance_fallback` 1:1 镜像
    /// (0 改 R33-4-1 module, 仅本地复制 15 行函数)
    pub fn execute(&self, subtask: &SubTask, query: &CouncilQuery) -> AdvisorOpinion {
        let stance_kind = keyword_stance_for_step(&subtask.description, &query.description);
        let stance = Stance::new(
            stance_kind,
            format!("Step {}: {} (planner: {})", subtask.step + 1, subtask.role, self.planner_role),
        );
        AdvisorOpinion::new(
            AdvisorId::new(format!("pe-{}-s{}", subtask.role, subtask.step)),
            stance,
            0.7, // 默认 confidence (per R33-4-1 0 LLM 路径一致)
            format!(
                "Planner {}: step {}/{} ({})",
                self.planner_role,
                subtask.step + 1,
                self.executor_roles.len(),
                subtask.role
            ),
            query.started_at_ms,
        )
        .with_weight(0.8) // 注入默认权重
    }

    /// **跑完整 Planner+Executor 流程**
    ///
    /// **流程**:
    /// 1. planner 拆 N 步
    /// 2. 每步 1 executor 产 1 opinion
    /// 3. 任意 step 触发 strong_disapprove (confidence ≥ 0.5) → 终止, termination_reason = "strong_disapprove"
    /// 4. 全部 opinions → synthesize() 出 verdict
    pub fn run(&mut self, query: &CouncilQuery) -> CollaborationVerdict {
        self.next_session_seq += 1;
        let started_at_ms = query.started_at_ms;
        let ctx = CollaborationContext::new(
            CollaborationMode::PlannerExecutor,
            query.clone(),
            self.next_session_seq,
            started_at_ms,
        );

        let plan = self.plan(query);
        let mut opinions: Vec<AdvisorOpinion> = Vec::new();
        let mut termination_reason = "plan_completed".to_string();
        let mut steps_run: u32 = 0;

        for subtask in &plan {
            let opinion = self.execute(subtask, query);
            if opinion.triggers_hold() {
                opinions.push(opinion);
                termination_reason = "strong_disapprove".to_string();
                steps_run = subtask.step + 1;
                break;
            }
            opinions.push(opinion);
            steps_run = subtask.step + 1;
        }

        let report = synthesize(&opinions, &self.weights);
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
fn keyword_stance_for_step(step_desc: &str, query_desc: &str) -> StanceKind {
    let combined = format!("{} {}", step_desc.to_lowercase(), query_desc.to_lowercase());
    let negative = [
        "harm", "exploit", "manipulate", "dishonest", "unethical", "伤害", "操纵", "不诚实", "剥削",
        "违反 asi",
    ];
    if negative.iter().any(|k| combined.contains(k)) {
        StanceKind::StrongDisapprove
    } else if combined.contains('?') || combined.contains("如何") {
        StanceKind::Neutral
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
        CouncilQuery::new("q-pe-test", desc, 0)
    }

    #[test]
    fn new_default_3_executor_roles() {
        let pe = PlannerExecutor::new("architect");
        assert_eq!(pe.planner_role(), "architect");
        assert_eq!(pe.executor_role_count(), 3);
    }

    #[test]
    fn with_executor_roles_4_custom() {
        let pe = PlannerExecutor::new("pm").with_executor_roles(vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ]);
        assert_eq!(pe.executor_role_count(), 4);
    }

    #[test]
    fn plan_deploy_query_yields_3_steps() {
        let pe = PlannerExecutor::new("architect");
        let plan = pe.plan(&q("deploy feature X to prod"));
        assert_eq!(plan.len(), 3);
        assert_eq!(plan[0].step, 0);
        assert_eq!(plan[2].step, 2);
        // 拆解含 deploy 关键词
        assert!(plan[2].description.contains("deploy"));
    }

    #[test]
    fn plan_design_query_yields_3_steps() {
        let pe = PlannerExecutor::new("architect");
        let plan = pe.plan(&q("design the auth system"));
        assert_eq!(plan.len(), 3);
        assert!(plan[1].description.contains("propose"));
    }

    #[test]
    fn plan_test_query_yields_3_steps() {
        let pe = PlannerExecutor::new("qa");
        let plan = pe.plan(&q("test the new feature"));
        assert_eq!(plan.len(), 3);
        assert!(plan[2].description.contains("verify"));
    }

    #[test]
    fn plan_fix_query_yields_3_steps() {
        let pe = PlannerExecutor::new("dev");
        let plan = pe.plan(&q("fix the login bug"));
        assert_eq!(plan.len(), 3);
        assert!(plan[2].description.contains("fix"));
    }

    #[test]
    fn plan_default_3_steps_for_unknown() {
        let pe = PlannerExecutor::new("default");
        let plan = pe.plan(&q("random query that matches nothing"));
        assert_eq!(plan.len(), 3);
    }

    #[test]
    fn plan_chinese_query_works() {
        let pe = PlannerExecutor::new("架构师");
        let plan = pe.plan(&q("上线 v1.0"));
        assert_eq!(plan.len(), 3);
        assert!(plan[2].description.contains("deploy"));
    }

    #[test]
    fn plan_with_custom_executor_roles() {
        let pe = PlannerExecutor::new("architect")
            .with_executor_roles(vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]);
        let plan = pe.plan(&q("deploy feature"));
        assert_eq!(plan[0].role, "alpha");
        assert_eq!(plan[1].role, "beta");
        assert_eq!(plan[2].role, "gamma");
    }

    #[test]
    fn execute_produces_approve_for_normal_query() {
        let pe = PlannerExecutor::new("architect");
        let st = SubTask::new(0, "design", "Step 1: design");
        let opinion = pe.execute(&st, &q("deploy feature"));
        assert_eq!(opinion.stance.kind, StanceKind::Approve);
        assert_eq!(opinion.confidence, 0.7);
        assert_eq!(opinion.weight, 0.8);
    }

    #[test]
    fn execute_produces_strong_disapprove_for_harm_query() {
        let pe = PlannerExecutor::new("bad");
        let st = SubTask::new(0, "do", "Step 1: do harm");
        let opinion = pe.execute(&st, &q("exploit users"));
        assert_eq!(opinion.stance.kind, StanceKind::StrongDisapprove);
    }

    #[test]
    fn execute_produces_neutral_for_question_query() {
        let pe = PlannerExecutor::new("ask");
        let st = SubTask::new(0, "ask", "Step 1: how to");
        let opinion = pe.execute(&st, &q("如何设计"));
        assert_eq!(opinion.stance.kind, StanceKind::Neutral);
    }

    #[test]
    fn run_3_step_default_deploy() {
        let mut pe = PlannerExecutor::new("architect");
        let verdict = pe.run(&q("deploy v1.0 to production"));
        assert_eq!(verdict.mode, CollaborationMode::PlannerExecutor);
        assert_eq!(verdict.steps, 3);
        assert_eq!(verdict.termination_reason, "plan_completed");
        assert_eq!(verdict.opinions.len(), 3);
        assert!(verdict.session_id.starts_with("collab-planner_executor-"));
    }

    #[test]
    fn run_terminates_at_strong_disapprove() {
        let mut pe = PlannerExecutor::new("bad");
        let verdict = pe.run(&q("deploy something that will harm users"));
        assert_eq!(verdict.termination_reason, "strong_disapprove");
        // 任意 step 触发 strong_disapprove → 终止
        assert!(verdict.steps <= 3);
    }

    #[test]
    fn run_synthesize_produces_weighted_score() {
        let mut pe = PlannerExecutor::new("architect");
        let verdict = pe.run(&q("deploy feature X"));
        // 3 个 Approve 合成应 ≥ 0.5 (per R10 synthesize 5 阈值)
        assert!(verdict.report.weighted_score > 0.0);
    }

    #[test]
    fn run_session_seq_monotonic() {
        let mut pe = PlannerExecutor::new("a");
        let v1 = pe.run(&q("deploy"));
        let v2 = pe.run(&q("design"));
        let v3 = pe.run(&q("test"));
        assert_ne!(v1.session_id, v2.session_id);
        assert_ne!(v2.session_id, v3.session_id);
        assert!(v2.session_id > v1.session_id); // 字符串序
        assert!(v3.session_id > v2.session_id);
    }

    #[test]
    fn run_empty_query_3_steps_default() {
        let mut pe = PlannerExecutor::new("default");
        let verdict = pe.run(&q(""));
        assert_eq!(verdict.steps, 3);
        assert_eq!(verdict.opinions.len(), 3);
    }

    #[test]
    fn subtask_new_basic() {
        let st = SubTask::new(0, "design", "Step 1: design");
        assert_eq!(st.step, 0);
        assert_eq!(st.role, "design");
        assert_eq!(st.description, "Step 1: design");
    }

    #[test]
    fn subtask_serde_round_trip() {
        let st = SubTask::new(1, "implement", "Step 2: implement");
        let json = serde_json::to_string(&st).unwrap();
        let back: SubTask = serde_json::from_str(&json).unwrap();
        assert_eq!(st, back);
    }
}
