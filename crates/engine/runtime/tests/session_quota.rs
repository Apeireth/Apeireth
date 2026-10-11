//! Session-level cognitive quota: the production spend gate over the session
//! ledger.
//!
//! Deterministic end-to-end proof through the canonical runtime, governance
//! pipeline, plugin registry and an in-process scripted provider (no network).
//! The behaviours pinned here:
//!
//! 1. consumption accumulates on the session ledger across rounds (token =
//!    per-round usage metering summed; tool steps = dispatched call count;
//!    cost = per-round `usage × unit price` at the frozen conversion);
//! 2. an exhausted token dimension closes the turn at the next checkpoint with
//!    a budget-exhausted frame carrying the real numbers and the dimension name;
//! 3. an exhausted tool-step dimension does the same under the `tool_steps`
//!    name, before the call that would exceed is dispatched;
//! 4. the frame aborts the turn (the answer is not delivered) while the prior
//!    transcript stays intact and every call in the batch is answered;
//! 5. an exhausted session burns no further provider rounds (the gate is
//!    consulted before spend, not after);
//! 6. without quota knobs the metering still runs (the self-report column is
//!    real) and no turn is ever cut;
//! 7. the cost dimension converts priced rounds (frozen per-side ceil
//!    rounding), never invents a price for an unpriced model (its round is
//!    counted `unpriced_rounds`, cost untouched), and binds its ceiling with a
//!    `cost_micros` frame carrying real numbers;
//! 8. the three metered dimensions never interfere: each accumulates only its
//!    own column and frames name only their own dimension.

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
    CostRateMicros, CostRateTable, QuotaDimension, Runtime, RuntimeError, SessionEventKind,
    SessionQuotaLimits, TurnRequest,
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

/// A fixed unit price for `MODEL`: 2 000 micros / 1k input, 8 000 / 1k output.
///
/// Any other model stays unpriced (never invented), so a test that must see
/// "no price row" semantics only has to name a different model.
#[derive(Debug)]
struct FixedRate(CostRateMicros);

impl CostRateTable for FixedRate {
    fn rate_for(&self, model: &str) -> Option<CostRateMicros> {
        (model == MODEL).then_some(self.0)
    }
}

fn fixed_rates() -> Arc<dyn CostRateTable> {
    Arc::new(FixedRate(CostRateMicros {
        input_micros_per_1k_tokens: 2_000,
        output_micros_per_1k_tokens: 8_000,
    }))
}

/// Build the runtime under test: an ungated note tool, the scripted provider,
/// and the session quota ceilings (`None` = that dimension stays unlimited).
/// `cost_rates` installs the unit-price source behind the cost dimension.
async fn build_runtime(
    provider: Arc<ScriptedProvider>,
    invocations: Arc<AtomicUsize>,
    quota: SessionQuotaLimits,
    cost_rates: Option<Arc<dyn CostRateTable>>,
) -> Runtime {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.note".into()));
    let mut builder = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(PermissionGovernanceHook::new(policy)))
        .with_plugin(TestPlugin::tools(invocations))
        .with_plugin(TestPlugin::provider(provider))
        .with_default_model(MODEL)
        .with_session_quota(quota);
    if let Some(rates) = cost_rates {
        builder = builder.with_cost_rates(rates);
    }
    builder.build().await.unwrap()
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
        None,
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
            max_cost_micros: None,
        },
        None,
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
            max_cost_micros: None,
        },
        None,
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
            max_cost_micros: None,
        },
        None,
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

/// ⑥ 有价轮换算/取整: 计量点按 `usage × 单价` (每侧向上取整, 永不低估) 把
/// 花费记进成本列; 不设限也照计量 (自述列是真的), token 维照常互不干扰。
#[tokio::test]
async fn a_priced_round_is_converted_onto_the_cost_column_with_frozen_rounding() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![
        Step::ToolCalls {
            content: "",
            usage: usage(1_500, 500),
            calls: vec![note_call("call-1")],
        },
        Step::Say {
            text: "done",
            usage: usage(1, 0),
        },
    ]);
    let runtime = build_runtime(
        provider.clone(),
        invocations.clone(),
        SessionQuotaLimits::default(),
        Some(fixed_rates()),
    )
    .await;
    let session = SessionId::new();
    let response = runtime
        .execute(TurnRequest::new(session, "run"))
        .await
        .expect("不设限的配额永不截断");
    assert_eq!(response.text, "done");

    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert_eq!(
        stored.quota.consumed_cost_micros,
        7_000 + 2,
        "换算口径 (冻结): 1.5k 输入 × 2 + 0.5k 输出 × 8 = 7000; 第二轮 1 输入 \
         token = 2 微单位 (每侧向上取整, 永不低估)"
    );
    assert_eq!(stored.quota.unpriced_rounds, 0, "两轮都有价");
    assert_eq!(stored.quota.consumed_tokens, 2_000 + 1, "token 维照计量");
    assert_eq!(stored.quota.consumed_tool_steps, 1, "步数维照计量");
}

/// ⑥b 取整口径走完整计量点: 单价非整数微/token 时逐侧向上取整 (永不低估)。
#[tokio::test]
async fn the_metering_point_rounds_each_side_up_never_underestimating_spend() {
    #[derive(Debug)]
    struct OddRate(CostRateMicros);

    impl CostRateTable for OddRate {
        fn rate_for(&self, model: &str) -> Option<CostRateMicros> {
            (model == MODEL).then_some(self.0)
        }
    }

    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::Say {
        text: "tiny",
        usage: usage(1, 1),
    }]);
    let runtime = build_runtime(
        provider.clone(),
        invocations.clone(),
        SessionQuotaLimits::default(),
        Some(Arc::new(OddRate(CostRateMicros {
            input_micros_per_1k_tokens: 2_501,
            output_micros_per_1k_tokens: 1,
        }))),
    )
    .await;
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "run"))
        .await
        .expect("不设限");

    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert_eq!(
        stored.quota.consumed_cost_micros,
        3 + 1,
        "1 输入 token × 2.501 微 = 2.501 → 3 (向上取整); 1 输出 token × 0.001 → 1"
    );
}

/// ⑦ 无价轮: cost=None 不计入 (绝不编价), 只计 `unpriced_rounds`; 不伪造
/// 超限事实 —— 有上限也照常跑完整回合。
#[tokio::test]
async fn an_unpriced_model_round_is_never_charged_but_counted_as_unpriced() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![
        Step::ToolCalls {
            content: "",
            usage: usage(1_500, 500),
            calls: vec![note_call("call-1")],
        },
        Step::Say {
            text: "done",
            usage: usage(1, 0),
        },
    ]);
    let runtime = build_runtime(
        provider.clone(),
        invocations.clone(),
        SessionQuotaLimits {
            max_tokens: None,
            max_tool_steps: None,
            // 花费上限给得很小: 无价轮不得伪造出超限事实。
            max_cost_micros: Some(1),
        },
        None,
    )
    .await;
    let session = SessionId::new();
    let response = runtime
        .execute(TurnRequest::new(session, "run"))
        .await
        .expect("无价轮 cost=None: 不编价也就没有超限");
    assert_eq!(response.text, "done");

    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert_eq!(
        stored.quota.consumed_cost_micros, 0,
        "无价轮不计入成本列 (绝不编价)"
    );
    assert_eq!(
        stored.quota.unpriced_rounds, 2,
        "两轮无价如实计数 (部分计量)"
    );
    assert_eq!(stored.quota.consumed_tokens, 2_001, "token 维不受牵连");
    assert_eq!(stored.quota.consumed_tool_steps, 1, "步数维不受牵连");
}

/// ⑦b 花费超限即帧收束: 帧的维度名 `cost_micros`、上限/已耗是真数字; 烧钱
/// 闸同款 —— 已耗尽的会话下一回合不再烧 provider 往返。
#[tokio::test]
async fn cost_quota_exhaustion_frames_real_numbers_under_the_cost_micros_dimension() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![
        Step::ToolCalls {
            content: "",
            usage: usage(1_500, 500),
            calls: vec![note_call("call-1")],
        },
        Step::Say {
            text: "second",
            usage: usage(1, 0),
        },
    ]);
    let runtime = build_runtime(
        provider.clone(),
        invocations.clone(),
        SessionQuotaLimits {
            max_tokens: None,
            max_tool_steps: None,
            max_cost_micros: Some(7_000),
        },
        Some(fixed_rates()),
    )
    .await;
    let session = SessionId::new();

    let error = runtime
        .execute(TurnRequest::new(session, "run"))
        .await
        .expect_err("第一轮换算 7000 已到花费上限");
    let RuntimeError::BudgetExhausted {
        dimension,
        limit,
        consumed,
    } = error
    else {
        panic!("expected BudgetExhausted, got {error}");
    };
    assert_eq!(dimension, QuotaDimension::CostMicros, "维度名正确");
    assert_eq!(limit, 7_000, "上限必须是真实数字");
    assert_eq!(consumed, 7_000, "已耗必须是真实数字 (1.5k×2 + 0.5k×8)");
    assert!(
        error_message_contains(dimension, limit, consumed, "cost_micros"),
        "错误帧正文必须带维度名/上限/已耗"
    );
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        0,
        "超限的调用点不再派发"
    );
    assert_eq!(provider.call_count(), 1, "不再烧 provider 往返");

    // 烧钱闸: 会话已耗尽, 新回合起手检查点直接收束。
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
    assert_eq!(dimension, QuotaDimension::CostMicros);
    assert_eq!(limit, 7_000);
    assert_eq!(consumed, 7_000, "会话级累计跨回合保持");
    assert_eq!(provider.call_count(), 1);
}

/// ⑧ 维互不干扰: 三维各记各的列, 超限只出自己维度的帧 —— 花费绑不绑与
/// token/步数维无关, 反之亦然。
#[tokio::test]
async fn the_three_dimensions_frame_independently_of_each_other() {
    // 场景 A: 只有花费超限 → 帧名 cost_micros (token/步数都在预算内)。
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::ToolCalls {
        content: "",
        usage: usage(1_500, 500),
        calls: vec![note_call("call-1")],
    }]);
    let runtime = build_runtime(
        provider.clone(),
        invocations.clone(),
        SessionQuotaLimits {
            max_tokens: Some(10_000),
            max_tool_steps: Some(10),
            max_cost_micros: Some(100),
        },
        Some(fixed_rates()),
    )
    .await;
    let session = SessionId::new();
    let error = runtime
        .execute(TurnRequest::new(session, "run"))
        .await
        .expect_err("花费维先到顶");
    let RuntimeError::BudgetExhausted { dimension, .. } = error else {
        panic!("expected BudgetExhausted, got {error}");
    };
    assert_eq!(dimension, QuotaDimension::CostMicros);

    // 场景 B: 只有 token 超限 → 帧名 tokens (花费照记 7000 但不牵连)。
    let invocations = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider::new(vec![Step::ToolCalls {
        content: "",
        usage: usage(1_500, 500),
        calls: vec![note_call("call-1")],
    }]);
    let runtime = build_runtime(
        provider.clone(),
        invocations.clone(),
        SessionQuotaLimits {
            max_tokens: Some(10),
            max_tool_steps: Some(10),
            max_cost_micros: Some(1_000_000),
        },
        Some(fixed_rates()),
    )
    .await;
    let session = SessionId::new();
    let error = runtime
        .execute(TurnRequest::new(session, "run"))
        .await
        .expect_err("token 维先到顶");
    let RuntimeError::BudgetExhausted { dimension, .. } = error else {
        panic!("expected BudgetExhausted, got {error}");
    };
    assert_eq!(dimension, QuotaDimension::Tokens);
    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert_eq!(
        stored.quota.consumed_cost_micros, 7_000,
        "花费维照记账, 只是不绑"
    );
    assert_eq!(stored.quota.consumed_tokens, 2_000, "token 维照记账");
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
