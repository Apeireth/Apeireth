//! 7 个 `LlmAdvisor` 真接 LLM (RC-6 真兑现, 子代理 N).

use std::sync::Arc;

use crate::llm::{CompletionMessage, CompletionRequest, LlmError, LlmFactory, LlmInstance};
use crate::{Advisor, AdvisorDecision, AdvisorKind, AdvisorVerdict, Proposal, SubagentRole};

/// 默认 primary model (per scene-d §3 决策 1: MiniMax-M3-thinking for 7 advisor).
pub const DEFAULT_PRIMARY_MODEL: &str = "minimax-m3-thinking";

/// Cheap fallback model (per scene-d §3 决策 1: 不同 model 隔离).
pub const DEFAULT_FALLBACK_MODEL: &str = "minimax-m3";

/// 7 advisor 的 canonical system prompt 模板.
pub fn seven_system_prompts() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "SafetyAdvisor",
            "review for safety risks, deny if any unsafe",
        ),
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

    /// 把 LLM 响应 text 解析成 bounded typed `AdvisorVerdict`.
    ///
    /// **输入即不可信**: LLM 响应可以任意长 (真实事故: 上游复述 >2000 字符时
    /// `AdvisorVerdict::validate` 的 critique 上限让调用点 `.expect` 直接 panic)。
    /// 入口先 trim + `chars().take(2000)` 截断再构造, 保证 keyword 派生路径
    /// **永不 panic**。
    fn parse_verdict(text: &str) -> AdvisorVerdict {
        // 先截断再小写化: 多字节 UTF-8 也不会切在 char 边界上 (chars() 而非 bytes)。
        let bounded: String = text.trim().chars().take(2_000).collect();
        let lower = bounded.to_ascii_lowercase();
        let (score, verdict) =
            if lower.contains("deny") || lower.contains("veto") || lower.contains("reject") {
                (0.0, AdvisorDecision::Stop)
            } else if lower.contains("abstain") || lower.contains("skip") {
                (0.0, AdvisorDecision::Abstain)
            } else {
                (1.0, AdvisorDecision::Allow)
            };
        AdvisorVerdict::new(score, verdict, bounded, None)
            .expect("keyword-derived advisor verdict is bounded")
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
            Err(_e) => {
                return AdvisorVerdict::new(
                    0.0,
                    AdvisorDecision::Abstain,
                    "proposal serialization failed",
                    None,
                )
                .expect("serialization-error advisor verdict is bounded")
            }
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
            // 缺省 = 不设上限, 由 provider 层 fill 4096 (canonical_openai_compatible.rs
            // adapt_request 的 reasoning 余量兜底)。原 Some(512) 对思考型模型必死
            // (reasoning_content 吃光预算 → content 空 → fail-loud; 教训: 500 必截断/
            // 2048 仍空 1/3/4096 稳), 2026-10-10 live 实锤: 顾问全灭被映射成
            // advisor error → Stop。答案本身只需几个 token, 余量是给 reasoning 的。
            max_tokens: None,
        };

        let instance: Box<dyn LlmInstance> = match self
            .factory
            .spawn(SubagentRole::Reviewer, &self.model)
            .await
        {
            Ok(i) => i,
            Err(e) => return map_llm_error_to_deny("spawn", e),
        };

        match instance.complete(req).await {
            Ok(resp) => {
                let text = resp.message.content;
                if text.trim().is_empty() {
                    AdvisorVerdict::new(0.0, AdvisorDecision::Abstain, "empty LLM response", None)
                        .expect("empty advisor verdict is bounded")
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
    AdvisorVerdict::new(0.0, AdvisorDecision::Stop, msg, None)
        .expect("error advisor verdict is bounded")
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
            (
                "SafetyAdvisor",
                "review for safety risks, deny if any unsafe",
            ),
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
        assert_eq!(v.verdict, AdvisorDecision::Stop);
        assert!(v.critique.contains("unsafe"));
        let v = LlmAdvisor::parse_verdict("VETO");
        assert_eq!(v.verdict, AdvisorDecision::Stop);
        let v = LlmAdvisor::parse_verdict("reject this proposal");
        assert_eq!(v.verdict, AdvisorDecision::Stop);
    }

    #[test]
    fn test_parse_verdict_abstain_keywords() {
        let v = LlmAdvisor::parse_verdict("Abstain: not my domain");
        assert_eq!(v.verdict, AdvisorDecision::Abstain);
        let v = LlmAdvisor::parse_verdict("skip this review");
        assert_eq!(v.verdict, AdvisorDecision::Abstain);
    }

    #[test]
    fn test_parse_verdict_default_allow() {
        let v = LlmAdvisor::parse_verdict("Looks fine, approve.");
        assert_eq!(v.verdict, AdvisorDecision::Allow);
    }

    /// 回归 (H1): 超长 LLM 响应曾在 `AdvisorVerdict::validate` 的 2000 字符上限上
    /// 让调用点 `.expect` panic。现在必须在入口截断且保持有界。
    #[test]
    fn test_parse_verdict_oversized_response_truncated_not_panicked() {
        // deny 放开头: 截断后 (尾部复述被切掉) 仍应判 Stop。
        let oversized = format!("deny {}", "a".repeat(3_000));
        let v = LlmAdvisor::parse_verdict(&oversized);
        assert_eq!(v.verdict, AdvisorDecision::Stop);
        assert_eq!(v.critique.chars().count(), 2_000);
        assert!(v.critique.is_char_boundary(v.critique.len()));

        // 纯中文超长: 验证截断按 char 而非 byte (不会 panic / 不会切坏 UTF-8)。
        let zh_only = "复".repeat(5_000);
        let v = LlmAdvisor::parse_verdict(&zh_only);
        assert_eq!(v.critique.chars().count(), 2_000);
        assert_eq!(v.verdict, AdvisorDecision::Allow);
        assert!(v.critique.is_char_boundary(v.critique.len()));
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
        assert_eq!(verdict.verdict, AdvisorDecision::Stop);
        assert!(verdict.critique.contains("advisor error"));
        assert!(verdict.critique.contains("spawn"));
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
