//! 深度维接线 (C-H2 · W2 单元 B): 运行时**真计数** + 超限**即帧**。
//!
//! 钉死的行为:
//!
//! 1. **喂点序列** —— 走一次真实嵌套 (模块发起的侧车调用) ⇒ 账本记的是既有
//!    调用树读数给出的**绝对**深度 (顶层 0 / 侧车 1), 峰值 = 1, 嵌套返回后
//!    当前层回落 (不泄漏计数);
//! 2. **早退路径** (嵌套调用失败) 不抬高当前层 —— RAII 守卫, 不靠自觉配对;
//! 3. **超深 ⇒ 拒绝即帧** —— 错误面与既有配额拒绝 (token / 步数 / 花费) **同形**;
//! 4. **未启用深度维 ⇒ 回合行为不变** (不拒、不出帧);
//! 5. **非法上限值 ⇒ 明确报错** (绝不静默放行成"无上限");
//! 6. **反序列化伪造不出计数** (落盘只带上限, 计数从 0 起);
//! 7. **可达深度 ≤ 1** —— 存量结构下上限 > 1 不可达 (断言钉住这个事实, 不写死
//!    一个会过时的数); 本维**不是有效的深度防护**, 只是把既有读数记成账;
//! 8. **四维检查逐维独立**。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedUsage,
};
use apeireth_runtime::canonical::quota::{
    parse_quota_max_depth_strict, DepthQuota, DepthQuotaHandle, MAX_QUOTA_DEPTH,
};
use apeireth_runtime::canonical::{
    InvocationContext, Runtime, RuntimeError, Session, SessionStore, TurnRequest,
};
use apeireth_runtime::{
    HookPoint, Module, ModuleContext, ModuleManifest, ModuleOutcome, SubLoopSpec,
    DEFAULT_MAX_INVOCATION_DEPTH,
};
use async_trait::async_trait;

const MODEL: &str = "depth-model";

/// 脚本化 provider 的一个脚本步。
enum ScriptStep {
    /// 回答一句话。
    Say(&'static str),
    /// 失败 (离线瞬时故障) —— 用来钉住"早退路径"。
    Fail,
}

struct ScriptedProvider {
    id: CapabilityId,
    script: Vec<ScriptStep>,
    calls: AtomicUsize,
}

impl ScriptedProvider {
    fn new(script: Vec<ScriptStep>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.depth").unwrap(),
            script,
            calls: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl ProviderCapability for ScriptedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            ModelId::new(MODEL).unwrap(),
            self.id.clone(),
        )]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        let step = self
            .script
            .get(index)
            .unwrap_or_else(|| panic!("called beyond script at step {index}"));
        match step {
            ScriptStep::Say(text) => Ok(NormalizedResponse {
                id: format!("resp_{index}"),
                model: request.model.clone(),
                content: (*text).to_string(),
                finish_reason: Some(NormalizedFinishReason::Stop),
                usage: NormalizedUsage {
                    prompt_tokens: 5,
                    completion_tokens: 5,
                    total_tokens: 10,
                    prompt_cache_hit_tokens: None,
                },
                tool_calls: Vec::new(),
                raw_metadata: serde_json::Map::new(),
            }),
            ScriptStep::Fail => Err(ProviderError::Network {
                provider: self.id.to_string(),
                detail: "offline transient failure".into(),
            }),
        }
    }
}

struct ScriptedProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<ScriptedProvider>,
}

impl ScriptedProviderPlugin {
    fn new(provider: Arc<ScriptedProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(PluginId::new("vendor.depth").unwrap(), "1.0.0", "mock")
                .declare_capability(
                    provider.id.clone(),
                    CapabilityKind::Provider,
                    "Scripted completions",
                )
                .unwrap(),
            provider,
        })
    }
}

#[async_trait]
impl Plugin for ScriptedProviderPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![Arc::clone(&self.provider) as Arc<dyn ProviderCapability>]
    }
}

/// 在 TurnStart 钩子发起一次**真实**嵌套 (侧车调用) 的模块。
struct SpawningModule {
    manifest: ModuleManifest,
    outcomes: Mutex<Vec<String>>,
}

impl SpawningModule {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            manifest: ModuleManifest::new("module.depth.spawn", "深度维接线模块"),
            outcomes: Mutex::new(Vec::new()),
        })
    }
}

#[async_trait]
impl Module for SpawningModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, apeireth_runtime::ModuleError> {
        if hook != HookPoint::TurnStart {
            return Ok(ModuleOutcome::continue_());
        }
        let spec = SubLoopSpec::single_turn("nested task").with_max_rounds(1);
        // 嵌套失败也**吞掉** (不把回合带崩): 早退路径的计数回落照样被观测。
        let outcome = match ctx.subloop().spawn(spec).await {
            Ok(result) => format!("ok:{}", result.text),
            Err(error) => format!("err:{error}"),
        };
        self.outcomes.lock().unwrap().push(outcome);
        Ok(ModuleOutcome::continue_())
    }
}

/// 造一个跑着"模块发起侧车调用"的运行时。
async fn runtime_with_nesting(
    depth_ceiling: Option<u64>,
    script: Vec<ScriptStep>,
) -> (Runtime, Arc<SpawningModule>) {
    let provider = ScriptedProvider::new(script);
    let plugin = ScriptedProviderPlugin::new(provider);
    let module = SpawningModule::new();
    let runtime = Runtime::builder()
        .with_plugin(plugin)
        .with_module(Arc::clone(&module) as Arc<dyn Module>)
        .with_governance(Arc::new(apeireth_governance::AllowAll))
        .with_default_model(MODEL)
        .with_depth_ceiling(depth_ceiling)
        .build()
        .await
        .expect("runtime builds cleanly");
    (runtime, module)
}

/// ① 喂点序列: 一次真实嵌套 ⇒ 记的是既有调用树读数的绝对深度, 峰值 = 1。
#[tokio::test]
async fn a_real_nesting_records_the_absolute_call_tree_depth() {
    let (runtime, module) = runtime_with_nesting(
        None,
        vec![
            ScriptStep::Say("nested answer"),
            ScriptStep::Say("parent answer"),
        ],
    )
    .await;
    let session_id = SessionId::new();
    let response = runtime
        .execute(TurnRequest::new(session_id, "hello").with_model(MODEL))
        .await
        .expect("turn executes");
    assert_eq!(response.text, "parent answer");
    assert!(
        module
            .outcomes
            .lock()
            .unwrap()
            .iter()
            .any(|out| out.starts_with("ok:")),
        "嵌套必须真的发生: {:?}",
        module.outcomes.lock().unwrap()
    );

    let session = runtime
        .sessions()
        .load(&session_id)
        .await
        .expect("load completes")
        .expect("session exists");
    assert_eq!(
        session.depth_quota.peak_depth(),
        1,
        "峰值 = 侧车调用的绝对层 (顶层 0 / 侧车 1)"
    );
    assert_eq!(
        session.depth_quota.current_depth(),
        0,
        "嵌套返回后当前层回落 (不泄漏计数)"
    );
    assert_eq!(session.depth_quota.max_depth(), None, "未启用 = 不设限");
}

/// ② 早退路径: 嵌套调用失败 ⇒ 当前层不抬高 (RAII 守卫)。
#[tokio::test]
async fn an_early_failing_nesting_does_not_leave_the_current_depth_raised() {
    let (runtime, module) = runtime_with_nesting(
        None,
        vec![ScriptStep::Fail, ScriptStep::Say("parent answer")],
    )
    .await;
    let session_id = SessionId::new();
    let _ = runtime
        .execute(TurnRequest::new(session_id, "hello").with_model(MODEL))
        .await;
    assert!(
        module
            .outcomes
            .lock()
            .unwrap()
            .iter()
            .any(|out| out.starts_with("err:")),
        "嵌套必须真的失败: {:?}",
        module.outcomes.lock().unwrap()
    );
    let session = runtime
        .sessions()
        .load(&session_id)
        .await
        .expect("load completes")
        .expect("session exists");
    assert_eq!(
        session.depth_quota.current_depth(),
        0,
        "早退路径不得抬高当前层 (守卫 drop 即回落)"
    );
    assert_eq!(
        session.depth_quota.peak_depth(),
        1,
        "峰值仍是真实观测到的层"
    );
}

/// ③ 超深 ⇒ 拒绝即帧 (错误面与既有配额拒绝同形)。
#[tokio::test]
async fn an_exhausted_depth_closes_the_turn_with_the_existing_quota_frame() {
    // 上限来自运行时配置 (配置是唯一真源: 会话载入处以它绑进账本)。
    let (runtime, _module) = runtime_with_nesting(
        Some(1),
        vec![
            ScriptStep::Say("nested answer"),
            ScriptStep::Say("parent answer"),
        ],
    )
    .await;
    let session_id = SessionId::new();

    let error = runtime
        .execute(TurnRequest::new(session_id, "hello").with_model(MODEL))
        .await
        .expect_err("深度维到顶 ⇒ 回合以预算耗尽帧收束");
    match &error {
        RuntimeError::BudgetExhausted {
            dimension,
            limit,
            consumed,
        } => {
            assert_eq!(
                *dimension,
                apeireth_runtime::canonical::QuotaDimension::Depth,
                "帧的维度名 = depth (与 token / 步数 / 花费同形)"
            );
            assert_eq!(*limit, 1, "帧带真实上限");
            assert_eq!(*consumed, 1, "帧带真实已耗");
        }
        other => panic!("expected the existing budget-exhausted frame, got {other:?}"),
    }
}

/// ④ 未启用深度维 ⇒ 回合行为不变 (不拒、不出帧)。
#[tokio::test]
async fn an_unconfigured_depth_dimension_leaves_the_turn_unchanged() {
    let (runtime, _module) = runtime_with_nesting(
        None,
        vec![
            ScriptStep::Say("nested answer"),
            ScriptStep::Say("parent answer"),
        ],
    )
    .await;
    let session_id = SessionId::new();
    let response = runtime
        .execute(TurnRequest::new(session_id, "hello").with_model(MODEL))
        .await
        .expect("未启用深度维 ⇒ 回合照常收束");
    assert_eq!(response.text, "parent answer");
    let session = runtime
        .sessions()
        .load(&session_id)
        .await
        .expect("load completes")
        .expect("session exists");
    assert_eq!(session.depth_quota.max_depth(), None);
    assert_eq!(
        session.depth_quota.peak_depth(),
        1,
        "计量照跑 (自述面的已耗列要真值); 但不拒、不出帧"
    );
}

/// ⑤ 非法上限值 ⇒ 明确报错 (绝不静默放行成"无上限")。
#[test]
fn an_illegal_depth_ceiling_is_an_explicit_error() {
    for raw in ["0", "-1", "abc", "1.5", " "]
        .into_iter()
        .chain(["0", "-1", "abc", "1.5"])
    {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        assert!(
            parse_quota_max_depth_strict(Some(raw)).is_err(),
            "非法上限值必须明确报错, 不得静默放行: {raw:?}"
        );
    }
    // 未设 / 空 = 不设限 (opt-in 默认, **不是**错误)。
    assert_eq!(parse_quota_max_depth_strict(None), Ok(None));
    assert_eq!(parse_quota_max_depth_strict(Some("")), Ok(None));
    assert_eq!(parse_quota_max_depth_strict(Some("   ")), Ok(None));
    // 正整数直通; 超大值钳到锚点 (**不**因解析失败变成"不设限")。
    assert_eq!(parse_quota_max_depth_strict(Some("4")), Ok(Some(4)));
    assert_eq!(
        parse_quota_max_depth_strict(Some("99999999999999")),
        Ok(Some(MAX_QUOTA_DEPTH)),
        "超大值钳到锚点, 两端一致"
    );
}

/// ⑥ 反序列化伪造不出计数 (落盘只带上限, 计数从 0 起)。
#[test]
fn a_deserialized_depth_ledger_cannot_forge_counters() {
    let forged = serde_json::json!({
        "max_depth": 3,
        "current_depth": 99,
        "peak_depth": 99,
    });
    let quota: DepthQuota = serde_json::from_value(forged.clone()).expect("shape still parses");
    assert_eq!(quota.max_depth(), Some(3), "上限是真配置, 照读");
    assert_eq!(quota.current_depth(), 0, "当前层不可伪造");
    assert_eq!(quota.peak_depth(), 0, "峰值不可伪造");
    assert_eq!(quota.consumed(), 0, "已耗 = 峰值 ⇒ 不可伪造");

    // 句柄同理: 落盘只带上限。
    let handle: DepthQuotaHandle =
        serde_json::from_value(forged.clone()).expect("shape still parses");
    assert_eq!(handle.max_depth(), Some(3));
    assert_eq!(handle.peak_depth(), 0);
    assert_eq!(handle.current_depth(), 0);
}

/// ⑦ 可达深度 ≤ 1 ⇒ 上限 > 1 不可达 (断言钉住事实, 不写死会过时的数)。
#[test]
fn the_currently_reachable_call_tree_depth_is_one() {
    // 顶层回合的调用树读数 = 0 (既有 `InvocationContext` 只有 user_turn)。
    assert_eq!(InvocationContext::user_turn().depth, 0);
    // 侧车调用的子层 = 父层 + 1 = 1。
    let reachable = u32::from(InvocationContext::user_turn().depth) + 1;
    assert_eq!(reachable, 1);
    // 存量运行时自己在 depth 2 就拒 ⇒ 可达深度被运行时自身封顶。
    assert!(
        u32::from(DEFAULT_MAX_INVOCATION_DEPTH) >= reachable,
        "运行时的调用树上限不得低于已观测到的可达深度"
    );
    assert_eq!(
        DEFAULT_MAX_INVOCATION_DEPTH, 1,
        "存量调用树上限 = 1 ⇒ **上限 > 1 的深度配额不可达** (配了也不触发)"
    );
}

/// ⑧ 四维检查逐维独立 (深度维启用后, 其余三维的判定不受牵连)。
#[test]
fn the_four_dimensions_stay_independent_once_depth_is_enabled() {
    use apeireth_orchestration::CognitiveQuota;
    use apeireth_runtime::canonical::QuotaDimension;
    use apeireth_runtime::canonical::SessionQuotaLimits;

    let limits = SessionQuotaLimits {
        max_tokens: Some(10),
        max_tool_steps: Some(2),
        max_cost_micros: Some(1_000),
    };
    let mut depth = DepthQuotaHandle::with_max_depth(Some(1));
    depth.observe_depth(1);
    assert_eq!(depth.max_depth(), Some(1));

    // 只有深度维到顶 ⇒ 只报深度维。
    let mut meter = CognitiveQuota::default();
    meter.consume_tokens(1);
    meter.consume_tool_step();
    let fact = limits
        .check_with_depth(&mut meter, &depth.read())
        .expect("深度维到顶");
    assert_eq!(fact.dimension, QuotaDimension::Depth);

    // token 维到顶 ⇒ 只报 token 维 (深度维不牵连)。
    let mut tokens = CognitiveQuota::default();
    tokens.consume_tokens(10);
    assert_eq!(
        limits
            .check_with_depth(&mut tokens, &depth.read())
            .map(|fact| fact.dimension),
        Some(QuotaDimension::Tokens)
    );

    // 步数维到顶 ⇒ 只报步数维。
    let mut steps = CognitiveQuota::default();
    steps.consume_tool_step();
    steps.consume_tool_step();
    assert_eq!(
        limits
            .check_with_depth(&mut steps, &depth.read())
            .map(|fact| fact.dimension),
        Some(QuotaDimension::ToolSteps)
    );

    // 花费维到顶 ⇒ 只报花费维。
    let mut cost = CognitiveQuota::default();
    cost.consume_cost_micros(1_000);
    assert_eq!(
        limits
            .check_with_depth(&mut cost, &depth.read())
            .map(|fact| fact.dimension),
        Some(QuotaDimension::CostMicros)
    );
}

/// ⑨ 峰值与当前是两个口径: 峰值单调不降, 当前层随作用域回落。
#[test]
fn the_peak_and_the_current_are_two_different_readings() {
    let handle = DepthQuotaHandle::with_max_depth(None);
    {
        let _scope = handle.scope(1).expect("不设限 ⇒ 放行");
        assert_eq!(handle.current_depth(), 1, "嵌套作用域内 = 当前层");
        assert_eq!(handle.peak_depth(), 1, "峰值 = 已观测的最深层");
    }
    assert_eq!(handle.current_depth(), 0, "作用域结束 ⇒ 当前层回落");
    assert_eq!(handle.peak_depth(), 1, "峰值不回落 (单调)");
    assert_eq!(handle.consumed(), 1, "已耗口径 = 峰值");
    let reading = handle.reading();
    assert_eq!(reading.limit, None);
    assert_eq!(reading.peak_depth, 1);
    assert_eq!(reading.consumed(), 1);
}

/// ⑩ 到顶 ⇒ 作用域拒绝 (既有配额拒绝语义的事实), 且拒绝不改计数。
#[test]
fn an_exhausted_depth_refuses_the_scope_without_touching_the_counters() {
    let handle = DepthQuotaHandle::with_max_depth(Some(1));
    {
        let _scope = handle.scope(1).expect("到顶前放行");
    }
    let refused = handle.scope(2).expect_err("到顶 ⇒ 拒绝这次嵌套");
    assert_eq!(
        refused.dimension,
        apeireth_runtime::canonical::QuotaDimension::Depth
    );
    assert_eq!(refused.limit, 1);
    assert_eq!(refused.consumed, 1, "事实带真数字");
    assert_eq!(handle.current_depth(), 0, "被拒的作用域不改计数");
    assert_eq!(handle.peak_depth(), 1, "被拒的作用域不改峰值");
}

/// ⑪ 深度上限旋钮: `None` ⇒ 恒不设限 (与不设该字段逐字节等); `Some(N)` ⇒
/// 上限绑进会话账本, 读数里 `limit = Some(N)`。
#[tokio::test]
async fn the_depth_ceiling_knob_is_off_by_default_and_binds_when_set() {
    // ① 未设 (默认 None) ⇒ 深度维恒不设限: 嵌套照常发生, 回合照常收束。
    let (runtime, _module) = runtime_with_nesting(
        None,
        vec![
            ScriptStep::Say("nested answer"),
            ScriptStep::Say("parent answer"),
        ],
    )
    .await;
    let session_id = SessionId::new();
    let response = runtime
        .execute(TurnRequest::new(session_id, "hello").with_model(MODEL))
        .await
        .expect("未设上限 ⇒ 回合照常收束 (零变化)");
    assert_eq!(response.text, "parent answer");
    let session = runtime
        .sessions()
        .load(&session_id)
        .await
        .expect("load completes")
        .expect("session exists");
    assert_eq!(session.depth_quota.max_depth(), None, "未设 ⇒ 恒不设限");
    assert_eq!(
        session.depth_quota.reading().limit,
        None,
        "自述读数里的上限同样是 None (不设限)"
    );

    // ② Some(N) ⇒ 上限绑进会话账本并生效 (到顶按既有配额语义出帧),
    //    读数里 limit = Some(N)。
    let (runtime, _module) = runtime_with_nesting(
        Some(1),
        vec![
            ScriptStep::Say("nested answer"),
            ScriptStep::Say("parent answer"),
        ],
    )
    .await;
    let session_id = SessionId::new();
    let error = runtime
        .execute(TurnRequest::new(session_id, "hello").with_model(MODEL))
        .await
        .expect_err("上限生效 ⇒ 到顶出帧");
    assert!(
        matches!(&error, RuntimeError::BudgetExhausted { .. }),
        "错误面与既有配额拒绝同形: {error:?}"
    );
    let session = runtime
        .sessions()
        .load(&session_id)
        .await
        .expect("load completes")
        .expect("session exists");
    assert_eq!(session.depth_quota.max_depth(), Some(1), "上限已绑进账本");
    assert_eq!(
        session.depth_quota.reading().limit,
        Some(1),
        "自述读数里的上限 = Some(N)"
    );
}
