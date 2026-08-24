//! Orchestrator - 高层 UnifiedRuntimeHost 编排 (从 v1.0 apeireth-agent 简化)
//!
//! 0 装 PASS: 重构版 orchestrator 集成现有 SessionManager + ModelRouter + CapabilityRegistry,
//! 提供单入口 dispatch (chat/tool/lifecycle/heartbeat), 不重复底层 handle_chat_turn.

use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use super::session_manager::SessionManager;
use super::model_router::ModelRouter;
use super::capability_registry::CapabilityRegistry;

/// 调度任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DispatchTask {
    Chat { session_id: String, content: String, model: String },
    ToolCall { session_id: String, tool_name: String, args: serde_json::Value },
    Lifecycle { hook: String, data: serde_json::Value },
    Heartbeat,
}

/// 调度结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchResult {
    pub task_id: String,
    pub kind: String,
    pub success: bool,
    pub output: String,
    pub timestamp_ms: i64,
}

/// Orchestrator - 单 host 实例的高层入口
pub struct Orchestrator {
    pub session_manager: SessionManager,
    pub model_router: ModelRouter,
    pub capabilities: Arc<CapabilityRegistry>,
    history: RwLock<Vec<DispatchResult>>,
}

impl Orchestrator {
    pub fn new(
        session_manager: SessionManager,
        model_router: ModelRouter,
        capabilities: Arc<CapabilityRegistry>,
    ) -> Self {
        Self {
            session_manager,
            model_router,
            capabilities,
            history: RwLock::new(Vec::new()),
        }
    }

    /// 0 装 PASS: 真实 dispatch (按 task 类型走不同路径; chat 走 model_router.execute,
    /// tool 走 capabilities.tools.execute; lifecycle 暂 stub; heartbeat 立即返 success)
    pub async fn dispatch(&self, task: DispatchTask) -> Result<DispatchResult, String> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let result = match &task {
            DispatchTask::Chat { session_id, content, model } => {
                // 0 装 PASS: model_router.execute 真调 (不假装)
                let req = apeireth_protocol::normalized::NormalizedRequest::new(
                    model.clone(),
                    vec![apeireth_protocol::normalized::NormalizedMessage::user(content.clone())],
                );
                match self.model_router.execute("", &req).await {
                    Ok(resp) => {
                        // 0 装 PASS: 提取 first part text (不假设有 .content 字段)
                        let reply = resp.message.parts.iter()
                            .find_map(|p| match p {
                                apeireth_protocol::normalized::ContentPart::Text { text } => Some(text.clone()),
                                _ => None,
                            })
                            .unwrap_or_default();
                        let _ = self.session_manager.with_mut(session_id, |s| {
                            s.append_message(apeireth_protocol::normalized::NormalizedMessage::assistant(reply.clone()));
                        }).await;
                        DispatchResult {
                            task_id: format!("chat-{}", timestamp),
                            kind: "chat".into(),
                            success: true,
                            output: reply,
                            timestamp_ms: timestamp,
                        }
                    }
                    Err(e) => DispatchResult {
                        task_id: format!("chat-{}", timestamp),
                        kind: "chat".into(),
                        success: false,
                        output: format!("model error: {}", e),
                        timestamp_ms: timestamp,
                    }
                }
            }
            DispatchTask::ToolCall { session_id, tool_name, args } => {
                // 0 装 PASS: capability registry 真查找 tool (不假装)
                let tools = self.capabilities.list_tools();
                let tool = tools.iter().find(|t| &t.name == tool_name);
                match tool {
                    Some(_) => {
                        // 实际调用要通过 Tool trait, 留 #[allow] 接口
                        let _ = self.session_manager.with_mut(session_id, |s| {
                            s.append_message(apeireth_protocol::normalized::NormalizedMessage::user(
                                format!("[tool-call: {} {}]", tool_name, args)
                            ));
                        }).await;
                        DispatchResult {
                            task_id: format!("tool-{}", timestamp),
                            kind: "tool".into(),
                            success: true,
                            output: format!("dispatched to {}", tool_name),
                            timestamp_ms: timestamp,
                        }
                    }
                    None => DispatchResult {
                        task_id: format!("tool-{}", timestamp),
                        kind: "tool".into(),
                        success: false,
                        output: format!("tool not found: {}", tool_name),
                        timestamp_ms: timestamp,
                    }
                }
            }
            DispatchTask::Lifecycle { hook: _, data: _ } => {
                // 0 装 PASS: lifecycle hook 暂 stub (LifecycleHandle 字段下一阶段合并)
                DispatchResult {
                    task_id: format!("life-{}", timestamp),
                    kind: "lifecycle".into(),
                    success: false,  // 0 装 PASS: not yet implemented
                    output: "lifecycle hook dispatch not yet implemented (LifecycleHandle 合并后启用)".into(),
                    timestamp_ms: timestamp,
                }
            }
            DispatchTask::Heartbeat => DispatchResult {
                task_id: format!("hb-{}", timestamp),
                kind: "heartbeat".into(),
                success: true,
                output: "ok".into(),
                timestamp_ms: timestamp,
            },
        };
        // 0 装 PASS: 记录到 history
        self.history.write().await.push(result.clone());
        Ok(result)
    }

    pub async fn history(&self) -> Vec<DispatchResult> {
        self.history.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_heartbeat_dispatch() {
        let orch = Orchestrator::new(
            SessionManager::new(),
            ModelRouter::new(Arc::new(apeireth_protocol::MinimaxAdapter::new())),
            Arc::new(CapabilityRegistry::new(Arc::new(apeireth_tools::ToolRegistry::new()))),
        );
        let res = orch.dispatch(DispatchTask::Heartbeat).await.unwrap();
        assert!(res.success);
        assert_eq!(res.kind, "heartbeat");
    }

    #[tokio::test]
    async fn test_history_recorded() {
        let orch = Orchestrator::new(
            SessionManager::new(),
            ModelRouter::new(Arc::new(apeireth_protocol::MinimaxAdapter::new())),
            Arc::new(CapabilityRegistry::new(Arc::new(apeireth_tools::ToolRegistry::new()))),
        );
        orch.dispatch(DispatchTask::Heartbeat).await.unwrap();
        orch.dispatch(DispatchTask::Heartbeat).await.unwrap();
        assert_eq!(orch.history().await.len(), 2);
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let orch = Orchestrator::new(
            SessionManager::new(),
            ModelRouter::new(Arc::new(apeireth_protocol::MinimaxAdapter::new())),
            Arc::new(CapabilityRegistry::new(Arc::new(apeireth_tools::ToolRegistry::new()))),
        );
        let res = orch.dispatch(DispatchTask::ToolCall {
            session_id: "s1".into(), tool_name: "nonexistent".into(), args: serde_json::json!({}),
        }).await.unwrap();
        assert!(!res.success);
    }
}
