//! **W2 §4.1 dreaming 接线 (决策点 D1)** — LLM 元思考器 + 确定性降级链 (2026-10-10)。
//!
//! 与 `organ_llm_bridge` 同居装配层 (LLM 桥归装配, memory 层零 LLM 依赖):
//!
//! - [`LlmMetaThinker`] — 经 [`LlmFactory`] 桥真思考 (做梦 6 阶段的 DeepSleep 推演)。
//!   `MetaThinker::think` 是**同步口** (trait 契约), 内部以私有 current-thread
//!   runtime 隔离 async 调用 —— **只可在非 async 上下文调用** (做梦是 CLI 显式命令,
//!   由同步入口触发, 合规); 在 async 上下文调用会 panic (tokio 语义), 属误用。
//! - [`FallbackMetaThinker`] — **降级链** (元层原则: 评审类机制只降级不枪毙主任务):
//!   主思考器失败 → 兜底思考器接管, 产出带 `[LLM 不可用(...), 降级 ...]` 留痕
//!   (0 装: 降级必须可见, 不许静默换脑)。
//!
//! **0 假装边界**: `stream`/多模态不涉; LLM 无配置时 CLI 不构造本思考器
//! (直接走确定性版, 见 `adapters/cli::dispatch_dream`), 不造"调用时才失败"的假可用。

use std::sync::Arc;

use apeireth_memory::meta_thinking::{
    MetaThinkError, MetaThinkInput, MetaThinkOutput, MetaThinker,
};
use apeireth_orchestration::SubagentRole;
use apeireth_plugin::llm_factory::{CompletionMessage, CompletionRequest, LlmFactory};

/// 做梦元思考的 system prompt (每步只出一段中文思考, 不出结构化结果)。
const DREAM_META_SYSTEM_PROMPT: &str = "你是认知做梦阶段(DeepSleep)的元思考器。\
每步输入给你: 阶段序号、思维簇名、原始查询、簇上下文、上一阶段结论。\
请只输出一段 <=200 字的中文思考: 承接上阶结论与本簇视角做交叉校核, \
一致处收敛, 冲突处明确标注「待裁」, 不要输出 JSON 或列表。";

/// 思考量进入 prompt 的截断 (防超长素材灌爆上下文)。
const PROMPT_SNIPPET_CHARS: usize = 2000;

fn prompt_of(input: &MetaThinkInput) -> String {
    let snippet = |text: &str| -> String {
        let mut out: String = text.chars().take(PROMPT_SNIPPET_CHARS).collect();
        if text.chars().count() > PROMPT_SNIPPET_CHARS {
            out.push('…');
        }
        out
    };
    format!(
        "阶段 {} / 思维簇「{}」\n原始查询:\n{}\n簇上下文:\n{}\n上一阶段结论:\n{}",
        input.stage,
        input.cluster,
        snippet(&input.query),
        snippet(&input.cluster_context),
        input
            .previous_thought
            .as_deref()
            .map(snippet)
            .unwrap_or_else(|| "(首阶段, 无)".to_string())
    )
}

/// **LLM 元思考器** (真思考; 与 organ 的 LLM 桥同模式, 落装配层)。
pub struct LlmMetaThinker {
    factory: Arc<dyn LlmFactory>,
    model: String,
    role: SubagentRole,
    rt: tokio::runtime::Runtime,
}

impl LlmMetaThinker {
    /// 构造 (自带私有 current-thread runtime; role 默认 Reviewer)。
    ///
    /// # Panics
    /// runtime 构造失败才 panic (进程级不可恢复, 属 tokio 语义)。
    pub fn new(factory: Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("dream thinker runtime");
        Self {
            factory,
            model: model.into(),
            role: SubagentRole::Reviewer,
            rt,
        }
    }

    /// 覆盖思考角色 (决定 LlmFactory 的 system prompt 模板)。
    #[must_use]
    pub fn with_role(mut self, role: SubagentRole) -> Self {
        self.role = role;
        self
    }
}

impl MetaThinker for LlmMetaThinker {
    fn think(&self, input: &MetaThinkInput) -> Result<MetaThinkOutput, MetaThinkError> {
        let request = CompletionRequest {
            system_prompt: DREAM_META_SYSTEM_PROMPT.to_string(),
            messages: vec![CompletionMessage {
                role: "user".to_string(),
                content: prompt_of(input),
            }],
            temperature: 0.7,
            tools: Vec::new(),
            // 思考型模型 reasoning_content 吃预算致 content 落空 (canonical_openai_
            // compatible.rs adapt_request 注释教训) —— 给足 reasoning 余量。
            max_tokens: Some(2048),
        };
        let outcome = self.rt.block_on(async {
            let instance = self
                .factory
                .spawn(self.role, &self.model)
                .await
                .map_err(|error| error.to_string())?;
            let response = instance
                .complete(request)
                .await
                .map_err(|e| e.to_string())?;
            Ok::<String, String>(response.message.content)
        });
        outcome
            .map(MetaThinkOutput::new)
            .map_err(MetaThinkError::Thinker)
    }
}

/// **降级链思考器**: 主思考器失败 → 兜底思考器, 产出带降级留痕 (只降级不枪毙)。
pub struct FallbackMetaThinker<P, F> {
    primary: P,
    fallback: F,
}

impl<P, F> FallbackMetaThinker<P, F>
where
    P: MetaThinker,
    F: MetaThinker,
{
    /// 组装降级链 (primary → fallback)。
    pub fn new(primary: P, fallback: F) -> Self {
        Self { primary, fallback }
    }
}

impl<P, F> MetaThinker for FallbackMetaThinker<P, F>
where
    P: MetaThinker,
    F: MetaThinker,
{
    fn think(&self, input: &MetaThinkInput) -> Result<MetaThinkOutput, MetaThinkError> {
        match self.primary.think(input) {
            Ok(output) => Ok(output),
            Err(primary_error) => {
                let mut degraded = self.fallback.think(input)?;
                degraded.thought =
                    format!("[LLM 不可用({primary_error}), 降级] {}", degraded.thought);
                Ok(degraded)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_memory::meta_thinking::MetaThinkOutput as Out;
    use async_trait::async_trait;

    struct FailingThinker;
    impl MetaThinker for FailingThinker {
        fn think(&self, _input: &MetaThinkInput) -> Result<Out, MetaThinkError> {
            Err(MetaThinkError::Thinker("forced failure".to_string()))
        }
    }

    struct EchoThinker;
    impl MetaThinker for EchoThinker {
        fn think(&self, input: &MetaThinkInput) -> Result<Out, MetaThinkError> {
            Ok(Out::new(format!("[规则推演] {}", input.cluster)))
        }
    }

    fn sample_input() -> MetaThinkInput {
        MetaThinkInput {
            stage: 1,
            cluster: "经验簇".to_string(),
            query: "q".to_string(),
            cluster_context: "ctx".to_string(),
            previous_thought: None,
        }
    }

    #[test]
    fn fallback_degrades_with_visible_marker() {
        let chain = FallbackMetaThinker::new(FailingThinker, EchoThinker);
        let out = chain.think(&sample_input()).expect("degrade not crash");
        assert!(
            out.thought
                .starts_with("[LLM 不可用(思考器执行失败: forced failure), 降级]"),
            "降级必须留痕: {}",
            out.thought
        );
        assert!(out.thought.contains("[规则推演] 经验簇"));
    }

    #[test]
    fn fallback_passes_primary_output_untouched() {
        let chain = FallbackMetaThinker::new(EchoThinker, FailingThinker);
        let out = chain.think(&sample_input()).unwrap();
        assert_eq!(out.thought, "[规则推演] 经验簇");
    }

    // ---- LlmMetaThinker: fake LlmFactory 双路径 (成功映射 / 失败归一为 Thinker 错) ----

    struct FakeInstance {
        content: String,
    }

    #[async_trait]
    impl apeireth_plugin::llm_factory::LlmInstance for FakeInstance {
        async fn complete(
            &self,
            _req: CompletionRequest,
        ) -> Result<
            apeireth_plugin::llm_factory::CompletionResponse,
            apeireth_plugin::llm_factory::LlmError,
        > {
            Ok(apeireth_plugin::llm_factory::CompletionResponse {
                message: CompletionMessage {
                    role: "assistant".to_string(),
                    content: self.content.clone(),
                },
                tool_calls: Vec::new(),
                finish_reason: "stop".to_string(),
                usage: Default::default(),
            })
        }
        fn name(&self) -> &str {
            "fake_dream_instance"
        }
    }

    struct FakeFactory {
        content: String,
    }

    #[async_trait]
    impl LlmFactory for FakeFactory {
        async fn spawn(
            &self,
            _role: SubagentRole,
            _model: &str,
        ) -> Result<
            Box<dyn apeireth_plugin::llm_factory::LlmInstance>,
            apeireth_plugin::llm_factory::LlmError,
        > {
            Ok(Box::new(FakeInstance {
                content: self.content.clone(),
            }))
        }
        async fn available_models(
            &self,
        ) -> Result<Vec<String>, apeireth_plugin::llm_factory::LlmError> {
            Ok(vec!["fake-model".to_string()])
        }
        fn name(&self) -> &str {
            "fake_dream_factory"
        }
    }

    struct BrokenFactory;

    #[async_trait]
    impl LlmFactory for BrokenFactory {
        async fn spawn(
            &self,
            _role: SubagentRole,
            _model: &str,
        ) -> Result<
            Box<dyn apeireth_plugin::llm_factory::LlmInstance>,
            apeireth_plugin::llm_factory::LlmError,
        > {
            Err(apeireth_plugin::llm_factory::LlmError::Network(
                "forced network down".to_string(),
            ))
        }
        async fn available_models(
            &self,
        ) -> Result<Vec<String>, apeireth_plugin::llm_factory::LlmError> {
            Ok(Vec::new())
        }
        fn name(&self) -> &str {
            "broken_dream_factory"
        }
    }

    #[test]
    fn llm_meta_thinker_maps_llm_output() {
        let thinker = LlmMetaThinker::new(
            Arc::new(FakeFactory {
                content: "收敛: 保留 X, 待裁 Y".to_string(),
            }),
            "fake-model",
        );
        let out = thinker.think(&sample_input()).expect("fake llm think");
        assert_eq!(out.thought, "收敛: 保留 X, 待裁 Y");
    }

    #[test]
    fn llm_meta_thinker_failure_becomes_thinker_error_and_chain_degrades() {
        let thinker = LlmMetaThinker::new(Arc::new(BrokenFactory), "fake-model");
        assert!(matches!(
            thinker.think(&sample_input()),
            Err(MetaThinkError::Thinker(_))
        ));

        // 降级链端到端: 坏工厂 + 确定性兜底 → 带痕成功。
        let chain = FallbackMetaThinker::new(thinker, EchoThinker);
        let out = chain.think(&sample_input()).expect("degrade");
        assert!(out.thought.contains("降级"), "{}", out.thought);
    }
}
