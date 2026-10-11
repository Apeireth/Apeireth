//! Session-level cognitive quota: the production spend gate over the session
//! ledger.
//!
//! Deterministic end-to-end proof through the canonical runtime, governance
//! pipeline, plugin registry and an in-process scripted provider (no network).
//! The behaviours pinned here:
//!
//! 1. consumption accumulates on the session ledger across rounds (token =
//!    per-round usage metering summed; tool steps = dispatched call count);
//! 2. an exhausted token dimension closes the turn at the next checkpoint with
//!    a budget-exhausted frame carrying the real numbers and the dimension name;
//! 3. an exhausted tool-step dimension does the same under the `tool_steps`
//!    name, before the call that would exceed is dispatched;
//! 4. the frame aborts the turn (the answer is not delivered) while the prior
//!    transcript stays intact and every call in the batch is answered;
//! 5. an exhausted session burns no further provider rounds (the gate is
//!    consulted before spend, not after);
//! 6. without quota knobs the metering still runs (the self-report column is
//!    real) and no turn is ever cut.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId, Timestamp, VirtualClock};
use apeireth_governance::{Permission, PermissionGovernanceHook, PermissionPolicy};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{
    QuotaDimension, Runtime, RuntimeError, SessionEventKind, SessionQuotaLimits, TurnRequest,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-quota";

/// One scripted provider response.
enum Step {
    /// A response with text and no tool calls.
    Say {
        text: &'static str,
        usage: NormalizedUsage,
    },
    /// A response with text plus tool calls.
    ToolCalls {
        content: &'static str,
        usage: NormalizedUsage,
        calls: Vec<ToolCall>,
    },
}

struct ScriptedProvider {
    id: CapabilityId,
    steps: Vec<Step>,
    calls: AtomicUsize,
}

impl ScriptedProvider {
    fn new(steps: Vec<Step>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            steps,
            calls: AtomicUsize::new(0),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ProviderCapability for ScriptedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new(MODEL).unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        let step = self
            .steps
            .get(index)
            .unwrap_or_else(|| panic!("provider script exhausted at call {index}"));
        Ok(match step {
            Step::Say { text, usage } => NormalizedResponse {
                id: format!("resp_{index}"),
                model: request.model.clone(),
                content: (*text).to_string(),
                finish_reason: Some(NormalizedFinishReason::Stop),
                usage: usage.clone(),
                tool_calls: Vec::new(),
                raw_metadata: serde_json::Map::new(),
            },
            Step::ToolCalls {
                content,
                usage,
                calls,
            } => NormalizedResponse {
                id: format!("resp_{index}"),
                model: request.model.clone(),
                content: (*content).to_string(),
                finish_reason: Some(NormalizedFinishReason::ToolCalls),
                usage: usage.clone(),
                tool_calls: calls.clone(),
                raw_metadata: serde_json::Map::new(),
            },
        })
    }
}

/// A counting demo tool dispatched freely under the test policy.
struct NoteTool {
    id: CapabilityId,
    invocations: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for NoteTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "note".into(),
            description: Some("counts invocations".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!({ "noted": true }))
    }
}

struct TestPlugin {
    manifest: PluginManifest,
    provider: Option<Arc<ScriptedProvider>>,
    tool_invocations: Option<Arc<AtomicUsize>>,
}

impl TestPlugin {
    fn provider(provider: Arc<ScriptedProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("test.quota_provider").unwrap(),
                "1.0.0",
                "scripted provider for session-quota tests",
            )
            .declare_capability(
                CapabilityId::new("provider.fake").unwrap(),
                CapabilityKind::Provider,
                "scripted provider",
            )
            .unwrap(),
            provider: Some(provider),
            tool_invocations: None,
        })
    }

    fn tools(invocations: Arc<AtomicUsize>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("test.quota_tool").unwrap(),
                "1.0.0",
                "ungated demo tool for session-quota tests",
            )
            .declare_capability(
                CapabilityId::new("tool.note").unwrap(),
                CapabilityKind::Tool,
                "counts invocations",
            )
            .unwrap(),
            provider: None,
            tool_invocations: Some(invocations),
        })
    }
}

#[async_trait]
impl Plugin for TestPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tool_invocations
            .as_ref()
            .map(|invocations| {
                vec![Arc::new(NoteTool {
                    id: CapabilityId::new("tool.note").unwrap(),
                    invocations: Arc::clone(invocations),
                }) as Arc<dyn ToolCapability>]
            })
            .unwrap_or_default()
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        self.provider
            .as_ref()
            .map(|provider| vec![Arc::clone(provider) as Arc<dyn ProviderCapability>])
            .unwrap_or_default()
    }
}

fn frozen_clock() -> Arc<VirtualClock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

fn note_call(id: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "note".into(),
        arguments: serde_json::json!({ "text": "hi" }),
    }
}

fn usage(prompt: u32, completion: u32) -> NormalizedUsage {
    NormalizedUsage::new(prompt, completion)
}

/// Build the runtime under test: an ungated note tool, the scripted provider,
/// and the session quota ceilings (0 = that dimension stays unlimited).
async fn build_runtime(
    provider: Arc<ScriptedProvider>,
    invocations: Arc<AtomicUsize>,
    quota: SessionQuotaLimits,
) -> Runtime {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.note".into()));
    Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(PermissionGovernanceHook::new(policy)))
        .with_plugin(TestPlugin::tools(invocations))
        .with_plugin(TestPlugin::provider(provider))
        .with_default_model(MODEL)
        .with_session_quota(quota)
        .build()
        .await
        .unwrap()
}

/// ① 累计正确: token = 回合计量 usage 累加 (跨轮求和), 步数 = 工具调用计数;
/// 不设限时计量照跑、回合照常完成。
#[tokio::test]
async fn the_session_ledger_accumulates_usage_tokens_and_tool_steps_across_rounds() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![
        Step::ToolCalls {
            content: "先记一条",
            usage: usage(10, 5),
            calls: vec![note_call("call-1")],
        },
        Step::Say {
            text: "done",
            usage: usage(20, 5),
        },
    ]);
    let runtime = build_runtime(
        provider.clone(),
        invocations.clone(),
        SessionQuotaLimits::default(),
    )
    .await;
    let session = SessionId::new();

    let response = runtime
        .execute(TurnRequest::new(session, "run"))
        .await
        .expect("an unlimited quota never cuts a turn");
    assert_eq!(response.text, "done");
    assert_eq!(invocations.load(Ordering::SeqCst), 1);

    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert_eq!(
        stored.quota.consumed_tokens,
        15 + 25,
        "token 计量 = 逐轮 usage.total_tokens 累加 (10+5 与 20+5)"
    );
    assert_eq!(
        stored.quota.consumed_tool_steps, 1,
        "步数 = 派发的工具调用数"
    );
}

/// ② token 超限即帧收束: 帧带维度名 tokens 与真实数字, 回合中止、前文保留。
#[tokio::test]
async fn token_quota_exhaustion_closes_the_turn_with_a_budget_exhausted_frame() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::ToolCalls {
        content: "准备记账",
        usage: usage(10, 5),
        calls: vec![note_call("call-1")],
    }]);
    let runtime = build_runtime(
        provider.clone(),
        invocations.clone(),
        SessionQuotaLimits {
            max_tokens: Some(10),
            max_tool_steps: None,
        },
    )
    .await;
    let session = SessionId::new();

    let error = runtime
        .execute(TurnRequest::new(session, "run"))
        .await
        .expect_err("the token ceiling binds at the tool-call checkpoint");
    let RuntimeError::BudgetExhausted {
        dimension,
        limit,
        consumed,
    } = error
    else {
        panic!("expected BudgetExhausted, got {error}");
    };
    assert_eq!(dimension, QuotaDimension::Tokens, "维度名正确");
    assert_eq!(limit, 10, "上限必须是真实数字");
    assert_eq!(consumed, 15, "已耗必须是真实数字 (10 输入 + 5 输出)");
    assert!(
        error_message_contains(dimension, limit, consumed, "tokens"),
        "错误帧正文必须带维度名/上限/已耗"
    );

    // 回合中止即帧: 该调用没有派发, 前文保留, 批内调用全部有答案。
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        0,
        "超限的调用点不再派发"
    );
    assert_eq!(provider.call_count(), 1, "不再烧 provider 往返");
    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert!(
        stored.messages.iter().any(
            |message| apeireth_protocol::canonical::ContentPart::join_text(&message.content)
                .contains("run")
        ),
        "用户前文保留"
    );
    assert!(
        stored.messages.iter().any(
            |message| apeireth_protocol::canonical::ContentPart::join_text(&message.content)
                .contains("quota is exhausted")
        ),
        "未答调用以合成结果收尾, transcript 合法"
    );
    assert!(stored.events.iter().any(|event| matches!(
        &event.event,
        SessionEventKind::ExecutionFailed { phase, .. } if phase == "quota_exhausted"
    )));
}

/// ③ 步数超限即帧收束: 帧的维度名是 tool_steps, 数字是真实计数。
#[tokio::test]
async fn tool_step_quota_exhaustion_names_the_tool_steps_dimension_with_real_numbers() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::ToolCalls {
        content: "",
        usage: usage(10, 5),
        calls: vec![note_call("call-1"), note_call("call-2")],
    }]);
    let runtime = build_runtime(
        provider.clone(),
        invocations.clone(),
        SessionQuotaLimits {
            max_tokens: None,
            max_tool_steps: Some(1),
        },
    )
    .await;
    let session = SessionId::new();

    let error = runtime
        .execute(TurnRequest::new(session, "run both"))
        .await
        .expect_err("the second call is over the step ceiling");
    let RuntimeError::BudgetExhausted {
        dimension,
        limit,
        consumed,
    } = error
    else {
        panic!("expected BudgetExhausted, got {error}");
    };
    assert_eq!(dimension, QuotaDimension::ToolSteps, "维度名正确");
    assert_eq!(limit, 1);
    assert_eq!(consumed, 1, "已耗 = 已派发的步数");
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        1,
        "上限内的一放行一, 超限的那一步不派发"
    );
    assert_eq!(provider.call_count(), 1);
}

/// ④⑤ 回合收束检查点 + 烧钱闸语义: 收束超限即帧中止; 已耗尽的会话不再烧
/// provider 往返 (新回合在起手检查点直接收束)。
#[tokio::test]
async fn an_exhausted_session_burns_no_further_provider_rounds() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![
        Step::Say {
            text: "答案已生成",
            usage: usage(10, 5),
        },
        Step::Say {
            text: "second",
            usage: usage(10, 5),
        },
    ]);
    let runtime = build_runtime(
        provider.clone(),
        invocations.clone(),
        SessionQuotaLimits {
            max_tokens: Some(10),
            max_tool_steps: None,
        },
    )
    .await;
    let session = SessionId::new();

    // 回合收束检查点: 最后一轮 usage 超限, 收束前即帧 (回合中止)。
    let error = runtime
        .execute(TurnRequest::new(session, "run"))
        .await
        .expect_err("the wrap-up checkpoint frames an over-budget turn");
    let RuntimeError::BudgetExhausted { dimension, .. } = error else {
        panic!("expected BudgetExhausted, got {error}");
    };
    assert_eq!(dimension, QuotaDimension::Tokens);
    assert_eq!(provider.call_count(), 1);

    // 烧钱闸: 会话已耗尽, 下一回合在起手检查点直接收束, 不再有 provider 往返。
    let error = runtime
        .execute(TurnRequest::new(session, "again"))
        .await
        .expect_err("an exhausted session must not burn another provider round");
    let RuntimeError::BudgetExhausted {
        dimension,
        limit,
        consumed,
    } = error
    else {
        panic!("expected BudgetExhausted, got {error}");
    };
    assert_eq!(dimension, QuotaDimension::Tokens);
    assert_eq!(limit, 10);
    assert_eq!(consumed, 15, "会话级累计跨回合保持");
    assert_eq!(
        provider.call_count(),
        1,
        "起手检查点生效: 一次 provider 往返都不多烧"
    );

    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert_eq!(stored.quota.consumed_tokens, 15, "会话账本落库且跨回合累计");
}

/// 错误帧正文合同: 维度名 / 上限 / 已耗三个真实数字都在帧里。
fn error_message_contains(
    dimension: QuotaDimension,
    limit: u64,
    consumed: u64,
    name: &str,
) -> bool {
    let message = RuntimeError::BudgetExhausted {
        dimension,
        limit,
        consumed,
    }
    .to_string();
    message.contains(name)
        && message.contains(&limit.to_string())
        && message.contains(&consumed.to_string())
}
