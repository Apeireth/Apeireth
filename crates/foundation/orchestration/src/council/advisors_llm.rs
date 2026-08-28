//! 7 个 `LlmAdvisor` 真接 LLM (RC-6 真兑现, 子代理 N).

use std::sync::Arc;

use crate::llm::{CompletionMessage, CompletionRequest, LlmError, LlmFactory, LlmInstance};
use crate::{Advisor, AdvisorKind, AdvisorVerdict, Proposal, SubagentRole};

/// 默认 primary model (per scene-d §3 决策 1: MiniMax-M3-thinking for 7 advisor).
pub const DEFAULT_PRIMARY_MODEL: &str = "minimax-m3-thinking";

/// Cheap fallback model (per scene-d §3 决策 1: 不同 model 隔离).
pub const DEFAULT_FALLBACK_MODEL: &str = "minimax-m3";

/// 7 advisor 的 canonical system prompt 模板.
pub fn seven_system_prompts() -> Vec<(&'static str, &'static str)> {
    vec![
        ("SafetyAdvisor", "review for safety risks, deny if any unsafe"),
        ("PerformanceAdvisor", "review for performance impact"),
        (
            "PhilosophyAdvisor",
            "review for philosophical consistency with 9 anchors",
        ),
        ("HistoryAdvisor", "review for historical precedent"),
        ("StrategyAdvisor", "review for strategic value"),
        ("EthicsAdvisor", "review for ethical implications"),
        ("LegalAdvisor", "review for legal compliance"),
    ]
}

/// LlmAdvisor — 单个 advisor 真接 LLM (per scene-d §3 决策 1, per-call 独立 instance).
pub struct LlmAdvisor {
    kind: AdvisorKind,
    display_name: &'static str,
    system_prompt: &'static str,
    factory: Arc<dyn LlmFactory>,
    model: String,
}

impl LlmAdvisor {
    pub fn new(
        kind: AdvisorKind,
        display_name: &'static str,
        system_prompt: &'static str,
        factory: Arc<dyn LlmFactory>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            display_name,
            system_prompt,
            factory,
            model: model.into(),
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn system_prompt(&self) -> &str {
        self.system_prompt
    }

    /// 把 LLM 响应 text 解析成 `AdvisorVerdict`.
    fn parse_verdict(text: &str) -> AdvisorVerdict {
        let lower = text.to_ascii_lowercase();
        if lower.contains("deny") || lower.contains("veto") || lower.contains("reject") {
            AdvisorVerdict::Deny {
                reason: text.trim().to_string(),
            }
        } else if lower.contains("abstain") || lower.contains("skip") {
            AdvisorVerdict::Abstain
        } else {
            AdvisorVerdict::Allow
        }
    }
}

#[async_trait::async_trait]
impl Advisor for LlmAdvisor {
    fn name(&self) -> &'static str {
        self.display_name
    }

    fn kind(&self) -> AdvisorKind {
        self.kind
    }

    async fn evaluate(&self, proposal: &Proposal) -> AdvisorVerdict {
        let proposal_json = match serde_json::to_string(proposal) {
            Ok(s) => s,
            Err(_e) => return AdvisorVerdict::Abstain,
        };

        let req = CompletionRequest {
            system_prompt: self.system_prompt.to_string(),
            messages: vec![CompletionMessage {
                role: "user".into(),
                content: format!(
                    "Review the following proposal and respond with one of: \
                     Allow / Deny (with reason) / Abstain.\n\nProposal:\n{proposal_json}"
                ),
            }],
            temperature: 0.0,
            tools: vec![],
            max_tokens: Some(512),
        };

        let instance: Box<dyn LlmInstance> =
            match self.factory.spawn(SubagentRole::Reviewer, &self.model).await {
                Ok(i) => i,
                Err(e) => return map_llm_error_to_deny("spawn", e),
            };

        match instance.complete(req).await {
            Ok(resp) => {
                let text = resp.message.content;
                if text.trim().is_empty() {
                    AdvisorVerdict::Abstain
                } else {
                    Self::parse_verdict(&text)
                }
            }
            Err(e) => map_llm_error_to_deny("complete", e),
        }
    }
}

/// 把 `LlmError` 一致映射到 `Deny { reason }` (per 子代理 B 0 模型污染路径 + 子代理 E 0 装诚实标注).
///
/// **0 装诚实**: 任何 LLM 错误 (网络 / 凭证 / rate limit / timeout / provider / not impl)
/// → Deny + reason 标记 "advisor error: ..." — 明确告知上层 Council "我没法评审,
/// 这不是 Abstain (advisor 选择), 这是基础设施失败". 比 Abstain 更诚实, 让上层能区分
/// "advisor 选择不参与" vs "我没法评审", 与子代理 B 风险 #5 "0 模型污染路径" 对齐.
fn map_llm_error_to_deny(stage: &'static str, err: LlmError) -> AdvisorVerdict {
    let msg = format!("advisor error: {stage}: {err}");
    AdvisorVerdict::Deny { reason: msg }
}

/// 构造 7 个默认 LlmAdvisor (per scene-d §5 决策 1: 全部 primary model).
pub fn default_seven_advisors(
    factory: Arc<dyn LlmFactory>,
    primary_model: &str,
) -> Vec<Arc<dyn Advisor>> {
    let templates = seven_system_prompts();
    let kinds = [
        AdvisorKind::Safety,
        AdvisorKind::Performance,
        AdvisorKind::Philosophy,
        AdvisorKind::History,
        AdvisorKind::Strategy,
        AdvisorKind::Ethics,
        AdvisorKind::Legal,
    ];
    debug_assert_eq!(templates.len(), kinds.len());

    kinds
        .iter()
        .zip(templates.iter())
        .map(|(kind, (display_name, system_prompt))| {
            Arc::new(LlmAdvisor::new(
                *kind,
                display_name,
                system_prompt,
                Arc::clone(&factory),
                primary_model,
            )) as Arc<dyn Advisor>
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::NoopLlmFactory;

    #[test]
    fn test_seven_system_prompts_match_documented_templates() {
        let prompts = seven_system_prompts();
        assert_eq!(prompts.len(), 7);

        let expected: [(&str, &str); 7] = [
            ("SafetyAdvisor", "review for safety risks, deny if any unsafe"),
            ("PerformanceAdvisor", "review for performance impact"),
            (
                "PhilosophyAdvisor",
                "review for philosophical consistency with 9 anchors",
            ),
            ("HistoryAdvisor", "review for historical precedent"),
            ("StrategyAdvisor", "review for strategic value"),
            ("EthicsAdvisor", "review for ethical implications"),
            ("LegalAdvisor", "review for legal compliance"),
        ];
        for (i, (name, prompt)) in prompts.iter().enumerate() {
            assert_eq!(*name, expected[i].0, "advisor #{} name drift", i);
            assert_eq!(*prompt, expected[i].1, "advisor #{} prompt drift", i);
        }
    }

    #[test]
    fn test_parse_verdict_deny_keywords() {
        let v = LlmAdvisor::parse_verdict("Deny: this is unsafe");
        match v {
            AdvisorVerdict::Deny { reason } => assert!(reason.contains("unsafe")),
            _ => panic!("expected Deny"),
        }
        let v = LlmAdvisor::parse_verdict("VETO");
        assert!(matches!(v, AdvisorVerdict::Deny { .. }));
        let v = LlmAdvisor::parse_verdict("reject this proposal");
        assert!(matches!(v, AdvisorVerdict::Deny { .. }));
    }

    #[test]
    fn test_parse_verdict_abstain_keywords() {
        let v = LlmAdvisor::parse_verdict("Abstain: not my domain");
        assert_eq!(v, AdvisorVerdict::Abstain);
        let v = LlmAdvisor::parse_verdict("skip this review");
        assert_eq!(v, AdvisorVerdict::Abstain);
    }

    #[test]
    fn test_parse_verdict_default_allow() {
        let v = LlmAdvisor::parse_verdict("Looks fine, approve.");
        assert_eq!(v, AdvisorVerdict::Allow);
    }

    #[test]
    fn test_llm_advisor_name_and_kind() {
        let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
        let advisor = LlmAdvisor::new(
            AdvisorKind::Safety,
            "SafetyAdvisor",
            "review for safety risks, deny if any unsafe",
            factory,
            DEFAULT_PRIMARY_MODEL,
        );
        assert_eq!(advisor.name(), "SafetyAdvisor");
        assert_eq!(advisor.kind(), AdvisorKind::Safety);
        assert_eq!(advisor.model(), DEFAULT_PRIMARY_MODEL);
    }

    #[tokio::test]
    async fn test_llm_advisor_spawn_failure_deny_with_reason() {
        let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
        let advisor = LlmAdvisor::new(
            AdvisorKind::Safety,
            "SafetyAdvisor",
            "test",
            factory,
            "minimax-m3",
        );
        let proposal = Proposal {
            id: "p-1".into(),
            proposer: "test".into(),
            payload: serde_json::json!({"action": "deploy"}),
            submitted_at: 1_700_000_000,
            session_id: apeireth_core::kernel::SessionId::new(),
        };
        let verdict = advisor.evaluate(&proposal).await;
        match verdict {
            AdvisorVerdict::Deny { reason } => {
                assert!(reason.contains("advisor error"));
                assert!(reason.contains("spawn"));
            }
            _ => panic!("expected Deny with reason on spawn failure, got {verdict:?}"),
        }
    }

    #[test]
    fn test_default_seven_advisors_count_and_names() {
        let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
        let advisors = default_seven_advisors(factory, DEFAULT_PRIMARY_MODEL);
        assert_eq!(advisors.len(), 7);

        let expected_names = [
            "SafetyAdvisor",
            "PerformanceAdvisor",
            "PhilosophyAdvisor",
            "HistoryAdvisor",
            "StrategyAdvisor",
            "EthicsAdvisor",
            "LegalAdvisor",
        ];
        for (i, advisor) in advisors.iter().enumerate() {
            assert_eq!(advisor.name(), expected_names[i]);
        }
    }
}
