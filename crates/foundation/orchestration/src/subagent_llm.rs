//! 生产 Orchestrator: LLM 驱动的 subagent 调度 (`LlmSubagentOrchestrator`)。
//!
//! **定位** (2026-10-10): worktree 装饰器的"主角" —— `Orchestrator` trait 的首个
//! 生产实现。scene-d 契约:
//! - **§3 多实例隔离**: 每次 dispatch 经 [`LlmFactory::spawn`] 独立实例 (按角色);
//! - **§5 同 provider 不同 model**: `spec.model` 覆盖默认 model (隔离实验/长程任务);
//! - **人工审批 fail-closed**: `spec.require_human_approval` 且未注入审批门 =
//!   **自动 deny** (`HumanDenied`) —— 无门无批, 绝不静默放行。
//!
//! **0 假装边界**: 输出解析为 JSON 则原样给 `output`; 非 JSON 则包 `{"raw": ...}`
//! 原文透传 (不编造结构)。`start`/`stop` 无状态 no-op (实例即用即弃)。

use std::sync::Arc;
use std::time::Duration;

use crate::llm::{CompletionMessage, CompletionRequest, LlmFactory};
use crate::{Orchestrator, OrchestratorError, SubagentOutcome, SubagentRole, SubagentSpec};

/// 人工审批门 (production 注入, 例如 CLI 交互 y/N): Err = 拒绝。
pub type HumanApprovalGate = Arc<dyn Fn(&SubagentSpec) -> Result<(), String> + Send + Sync>;

/// 默认 dispatch 超时 (毫秒)。
pub const SUBAGENT_DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// LLM 驱动的生产 Orchestrator。
pub struct LlmSubagentOrchestrator {
    factory: Arc<dyn LlmFactory>,
    default_model: String,
    timeout_ms: u64,
    approval_gate: Option<HumanApprovalGate>,
}

impl LlmSubagentOrchestrator {
    /// 构造 (factory = 真 LLM 镜像工厂; default_model = spec.model 缺省时使用)。
    pub fn new(factory: Arc<dyn LlmFactory>, default_model: impl Into<String>) -> Self {
        Self {
            factory,
            default_model: default_model.into(),
            timeout_ms: SUBAGENT_DEFAULT_TIMEOUT_MS,
            approval_gate: None,
        }
    }

    /// 注入人工审批门 (不注入 = require_human_approval 的 spec 自动 deny)。
    #[must_use]
    pub fn with_approval_gate(mut self, gate: HumanApprovalGate) -> Self {
        self.approval_gate = Some(gate);
        self
    }

    /// 覆盖 dispatch 超时 (毫秒)。
    #[must_use]
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// 角色默认 system prompt (spec.system_prompt 可覆盖)。
fn role_system_prompt(role: SubagentRole) -> &'static str {
    match role {
        SubagentRole::Planner => {
            "你是规划子代理 (Planner): 把任务拆成可验证的实施步骤, 输出 JSON \
             {\"steps\": [...], \"risks\": [...]}。不写代码, 只出计划。"
        }
        SubagentRole::Implementer => {
            "你是实施子代理 (Implementer): 按计划产出最小可行实现, 输出 JSON \
             {\"changes\": [...], \"notes\": \"...\"}。不夸大, 完成多少说多少。"
        }
        SubagentRole::Reviewer => {
            "你是评审子代理 (Reviewer): 只按可验证证据评审, 输出 JSON \
             {\"passed\": bool, \"score\": 0.0-1.0, \"blocking_issues\": [...], \
             \"suggestions\": [...]}。宁可拒也不错放。"
        }
        _ => {
            "你是子代理: 按任务产出结构化 JSON 结果。0 假装 —— 未完成的部分显式标注, \
             不编造。"
        }
    }
}

fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

#[async_trait::async_trait]
impl Orchestrator for LlmSubagentOrchestrator {
    async fn start(&mut self) -> Result<(), OrchestratorError> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), OrchestratorError> {
        Ok(())
    }

    async fn dispatch(&self, spec: SubagentSpec) -> Result<SubagentOutcome, OrchestratorError> {
        // ① 人工审批 (fail-closed: 无门自动 deny, 绝不静默放行)。
        if spec.require_human_approval {
            match &self.approval_gate {
                Some(gate) => gate(&spec).map_err(OrchestratorError::HumanDenied)?,
                None => {
                    return Err(OrchestratorError::HumanDenied(
                        "spec requires human approval but no approval gate is wired \
                         (自动 deny —— fail-closed)"
                            .to_string(),
                    ))
                }
            }
        }

        // ② 隔离实例 (scene-d §3; §5: spec.model 覆盖默认 model)。
        let model = spec
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());
        let mut instance = self
            .factory
            .spawn(spec.role, &model)
            .await
            .map_err(|e| OrchestratorError::SubagentFailed(format!("spawn failed: {e}")))?;

        // ③ 请求 (system prompt 角色默认 / spec 覆盖; 载荷 = 标题 + payload)。
        let system_prompt = spec
            .system_prompt
            .clone()
            .unwrap_or_else(|| role_system_prompt(spec.role).to_string());
        let user_content = format!(
            "# 任务\n{}\n\n# 载荷\n{}",
            spec.title,
            serde_json::to_string_pretty(&spec.payload).unwrap_or_default()
        );
        let request = CompletionRequest {
            system_prompt,
            messages: vec![CompletionMessage {
                role: "user".to_string(),
                content: user_content,
            }],
            temperature: 1.0,
            tools: Vec::new(),
            // thinking 模型教训 (真缺陷 #4): 不限 max_tokens, 由 provider 默认配额
            // 兜底 —— 免得 reasoning_content 把预算吃光留下空 content。
            max_tokens: None,
        };

        // ④ 有界等待 (超时 = 显式失败, 不静默)。
        let response = tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            instance.complete(request),
        )
        .await
        .map_err(|_| {
            OrchestratorError::SubagentFailed(format!(
                "dispatch timed out after {}ms",
                self.timeout_ms
            ))
        })?
        .map_err(|e| OrchestratorError::SubagentFailed(format!("completion failed: {e}")))?;

        // ⑤ 输出 (JSON 解析成功原样给; 否则 {"raw": ...} 原文透传, 不编造)。
        let content = response.message.content;
        let output = serde_json::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|_| serde_json::json!({ "raw": content }));

        Ok(SubagentOutcome {
            spec_id: spec.id,
            role: spec.role,
            output,
            success: true,
            error: None,
            completed_at: now_epoch_ms(),
        })
    }
}

#[cfg(test)]
mod subagent_llm_tests {
    use super::*;
    use crate::llm::{CompletionResponse, LlmError, LlmInstance, TokenUsage};

    struct FakeInstance {
        content: String,
    }

    #[async_trait::async_trait]
    impl LlmInstance for FakeInstance {
        async fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            Ok(CompletionResponse {
                message: CompletionMessage {
                    role: "assistant".to_string(),
                    content: self.content.clone(),
                },
                tool_calls: Vec::new(),
                finish_reason: "stop".to_string(),
                usage: TokenUsage::default(),
            })
        }
        fn name(&self) -> &str {
            "fake"
        }
    }

    struct FakeFactory {
        content: String,
        seen: std::sync::Mutex<Vec<(SubagentRole, String)>>,
    }

    #[async_trait::async_trait]
    impl LlmFactory for FakeFactory {
        async fn spawn(
            &self,
            role: SubagentRole,
            model: &str,
        ) -> Result<Box<dyn LlmInstance>, LlmError> {
            self.seen
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .push((role, model.to_string()));
            Ok(Box::new(FakeInstance {
                content: self.content.clone(),
            }))
        }
        async fn available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec!["fake".to_string()])
        }
        fn name(&self) -> &str {
            "fake-factory"
        }
    }

    fn spec(id: &str, require_human_approval: bool) -> SubagentSpec {
        SubagentSpec {
            id: id.to_string(),
            role: SubagentRole::Planner,
            title: "demo".to_string(),
            payload: serde_json::json!({"goal": "x"}),
            model: Some("iso-model".to_string()),
            system_prompt: None,
            require_human_approval,
        }
    }

    #[tokio::test]
    async fn require_human_approval_without_gate_denies() {
        // 五件门② 对偶 (fail-closed): 无审批门 = 自动 deny, 绝不静默放行。
        let factory = Arc::new(FakeFactory {
            content: "{}".to_string(),
            seen: std::sync::Mutex::new(Vec::new()),
        });
        let orch = LlmSubagentOrchestrator::new(factory.clone(), "default-model");
        let result = orch.dispatch(spec("t1", true)).await;
        match result {
            Err(OrchestratorError::HumanDenied(_)) => {}
            other => panic!("expected HumanDenied, got {other:?}"),
        }
        assert!(
            factory
                .seen
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .is_empty(),
            "deny 后不得 spawn 实例"
        );
    }

    #[tokio::test]
    async fn dispatch_parses_json_output_and_isolates_model() {
        // 五件门③: 效果可见 = JSON 原样给 + spec.model 隔离生效 (scene-d §5)。
        let factory = Arc::new(FakeFactory {
            content: "{\"answer\": 42}".to_string(),
            seen: std::sync::Mutex::new(Vec::new()),
        });
        let orch = LlmSubagentOrchestrator::new(factory.clone(), "default-model");
        let outcome = orch.dispatch(spec("t2", false)).await.expect("dispatch ok");
        assert!(outcome.success);
        assert_eq!(outcome.output["answer"], serde_json::json!(42));
        assert_eq!(outcome.spec_id, "t2");
        let seen = factory
            .seen
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        assert_eq!(seen, vec![(SubagentRole::Planner, "iso-model".to_string())]);
    }

    #[tokio::test]
    async fn non_json_output_wrapped_raw_not_fabricated() {
        // 0 假装: 非 JSON 输出原样透传 ({"raw": ...}), 不编造结构。
        let factory = Arc::new(FakeFactory {
            content: "自由文本结论".to_string(),
            seen: std::sync::Mutex::new(Vec::new()),
        });
        let orch = LlmSubagentOrchestrator::new(factory, "default-model");
        let outcome = orch.dispatch(spec("t3", false)).await.expect("dispatch ok");
        assert_eq!(outcome.output["raw"], serde_json::json!("自由文本结论"));
    }

    #[tokio::test]
    async fn approval_gate_rejection_maps_to_human_denied() {
        let factory = Arc::new(FakeFactory {
            content: "{}".to_string(),
            seen: std::sync::Mutex::new(Vec::new()),
        });
        let gate: HumanApprovalGate = Arc::new(|_spec| Err("主人拒绝".to_string()));
        let orch =
            LlmSubagentOrchestrator::new(factory.clone(), "default-model").with_approval_gate(gate);
        match orch.dispatch(spec("t4", true)).await {
            Err(OrchestratorError::HumanDenied(msg)) => assert!(msg.contains("主人拒绝")),
            other => panic!("expected HumanDenied, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn approval_gate_grant_proceeds() {
        let factory = Arc::new(FakeFactory {
            content: "{}".to_string(),
            seen: std::sync::Mutex::new(Vec::new()),
        });
        let gate: HumanApprovalGate = Arc::new(|_spec| Ok(()));
        let orch = LlmSubagentOrchestrator::new(factory, "default-model").with_approval_gate(gate);
        assert!(orch.dispatch(spec("t5", true)).await.is_ok());
    }
}
