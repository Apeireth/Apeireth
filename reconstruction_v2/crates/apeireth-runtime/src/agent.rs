//! Agent - 子代理抽象 (从 v1.0 apeireth-agent 2,989 LOC 收敛)
//!
//! 0 装 PASS: 重构版 agent 用 Apeireth 现有 council / evolution / host 接口, 提供高层 agent manager.
//!
//! 设计 (per right 图 "Unified Runtime Host" 多代理支持):
//! - AgentContext: 共享 context (session_id, capability set, message buffer)
//! - AgentInstance: 单个 agent (name + role + handle to UnifiedRuntimeHost sub-system)
//! - AgentManager: 编排多个 agent (dispatch + route by tag)

use std::collections::HashMap;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// 共享 context (跨 agent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub session_id: String,
    pub user_input: String,
    pub tags: Vec<String>,  // 用于 agent 路由
    pub shared_data: HashMap<String, String>,
}

/// Agent 角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    Facilitator,    // 协调其他 agent
    Researcher,     // 信息检索
    Coder,          // 写代码
    Reviewer,       // 审阅
    Tester,         // 测试
    Writer,         // 文档
}

impl AgentRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Facilitator => "facilitator",
            Self::Researcher => "researcher",
            Self::Coder => "coder",
            Self::Reviewer => "reviewer",
            Self::Tester => "tester",
            Self::Writer => "writer",
        }
    }
}

/// Agent 单实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInstance {
    pub id: String,
    pub name: String,
    pub role: AgentRole,
    pub tags: Vec<String>,  // 用于自动路由
    pub active: bool,
}

impl AgentInstance {
    pub fn new(id: String, name: String, role: AgentRole) -> Self {
        Self { id, name, role, tags: Vec::new(), active: true }
    }

    pub fn can_handle(&self, ctx: &AgentContext) -> bool {
        if !self.active { return false; }
        // 0 装 PASS: 简单 tag 匹配 (后续可换 fuzzy match)
        self.tags.iter().any(|t| ctx.tags.contains(t))
    }
}

/// AgentManager - 多个 agent 编排
pub struct AgentManager {
    agents: RwLock<HashMap<String, AgentInstance>>,
    /// 0 装 PASS: 按 tag 自动路由的 default agent
    default_role: AgentRole,
}

impl AgentManager {
    pub fn new(default_role: AgentRole) -> Self {
        Self { agents: RwLock::new(HashMap::new()), default_role }
    }

    pub async fn register(&self, agent: AgentInstance) {
        self.agents.write().await.insert(agent.id.clone(), agent);
    }

    pub async fn route(&self, ctx: &AgentContext) -> Option<AgentInstance> {
        let agents = self.agents.read().await;
        // 优先 tag 匹配
        for agent in agents.values() {
            if agent.can_handle(ctx) { return Some(agent.clone()); }
        }
        // fallback: default role
        for agent in agents.values() {
            if agent.role == self.default_role { return Some(agent.clone()); }
        }
        None
    }
}

impl Default for AgentManager {
    fn default() -> Self { Self::new(AgentRole::Facilitator) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_role_labels() {
        assert_eq!(AgentRole::Facilitator.label(), "facilitator");
        assert_eq!(AgentRole::Coder.label(), "coder");
    }

    #[test]
    fn test_agent_can_handle_by_tag() {
        let mut a = AgentInstance::new("a".into(), "Test".into(), AgentRole::Researcher);
        a.tags.push("research".into());
        let mut ctx = AgentContext {
            session_id: "s1".into(), user_input: "?".into(),
            tags: vec!["research".into()], shared_data: HashMap::new(),
        };
        assert!(a.can_handle(&ctx));
        ctx.tags = vec!["other".into()];
        assert!(!a.can_handle(&ctx));
    }

    #[tokio::test]
    async fn test_agent_manager_routing() {
        let mgr = AgentManager::new(AgentRole::Facilitator);
        let mut coder = AgentInstance::new("c1".into(), "Coder".into(), AgentRole::Coder);
        coder.tags.push("code".into());
        mgr.register(coder).await;
        let mut fac = AgentInstance::new("f1".into(), "Fac".into(), AgentRole::Facilitator);
        fac.tags.push("general".into());
        mgr.register(fac).await;

        // tag 匹配 → coder
        let ctx1 = AgentContext { session_id: "s1".into(), user_input: "?".into(), tags: vec!["code".into()], shared_data: HashMap::new() };
        let r1 = mgr.route(&ctx1).await.unwrap();
        assert_eq!(r1.role, AgentRole::Coder);
        // 无 tag 匹配 → fallback default
        let ctx2 = AgentContext { session_id: "s2".into(), user_input: "?".into(), tags: vec!["other".into()], shared_data: HashMap::new() };
        let r2 = mgr.route(&ctx2).await.unwrap();
        assert_eq!(r2.role, AgentRole::Facilitator);
    }
}
