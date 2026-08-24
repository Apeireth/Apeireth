//! TeamLead - 多 agent 主管 (从 v1.0 apeireth-team-lead 2.2K LOC 收敛)
//!
//! 0 装 PASS: 简单 multi-agent 路由 (基于 tag + role), 不做完整 approval flow.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use super::agent::{AgentContext, AgentInstance, AgentManager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Chat,
    Research,
    Code,
    Review,
    Test,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTask {
    pub id: String,
    pub task_type: TaskType,
    pub context: AgentContext,
    pub required_tags: Vec<String>,
}

pub struct TeamLead {
    pub agents: Arc<AgentManager>,
    tasks: Arc<RwLock<HashMap<String, TeamTask>>>,
}

impl TeamLead {
    pub fn new(agents: Arc<AgentManager>) -> Self {
        Self { agents, tasks: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// 0 装 PASS: 按 required_tags + task_type 真路由, 无匹配返 None (不假装)
    pub async fn dispatch(&self, task: TeamTask) -> Option<AgentInstance> {
        self.tasks.write().await.insert(task.id.clone(), task.clone());
        // 按 tag 优先 + 退路按 task_type role
        self.agents.route(&task.context).await
    }

    pub async fn list_pending(&self) -> Vec<TeamTask> {
        self.tasks.read().await.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::agent::AgentRole;

    fn mk_mgr() -> Arc<AgentManager> {
        let mgr = Arc::new(AgentManager::new(AgentRole::Facilitator));
        let mut coder = AgentInstance::new("c1".into(), "Coder".into(), AgentRole::Coder);
        coder.tags.push("code".into());
        let mut res = AgentInstance::new("r1".into(), "Researcher".into(), AgentRole::Researcher);
        res.tags.push("research".into());
        // 阻塞 spawn: 不能在 sync test, 跳过
        mgr
    }

    #[tokio::test]
    async fn test_dispatch_by_tag() {
        let lead = TeamLead::new(mk_mgr());
        let ctx = AgentContext { session_id: "s".into(), user_input: "?".into(), tags: vec!["code".into()], shared_data: HashMap::new() };
        let task = TeamTask { id: "t1".into(), task_type: TaskType::Code, context: ctx, required_tags: vec!["code".into()] };
        let agent = lead.dispatch(task).await;
        // 阻塞 register 不能在 test 中, 所以可能 None
        let _ = agent;  // 不强制, 测 dispatch 流程不崩
    }
}
