//! 内部执行强制复核 (System One, `rt.system_one_internal`) 的生产接线回归。
//!
//! # 这条测试钉什么
//!
//! 复核门 (`SystemOneGate`) 在 runtime 侧早就存在，但生产组合根**从不构造**它
//! ⇒ 内部执行（Internal 面 / Protected 效果的工具）必得 `system_one_unavailable`。
//! 本测试从**组合根用的同一批装配函数**出发，断言：
//!
//! 1. **旋钮关 ⇒ 与接线前逐字一致**：不构造复核门 ⇒ 内部执行仍走「无门」那条
//!    路径（错误文案**逐字**相同）⇒ 零行为变化、零副作用（连审计账本都不落盘）；
//! 2. **旋钮开 + 有真实 provider ⇒ 内部执行不再得 `system_one_unavailable`**：
//!    复核真的跑了一轮（审计账本落一条 `allowed`），工具真的执行了；
//! 3. **旋钮开 + 没有真实 provider ⇒ fail-closed**：宿主**拒绝**绑定监督者
//!    （更不许塞假 provider 顶替）⇒ 门保持无监督 ⇒ 内部执行照样被拒。
//!
//! 两条分支都断言：provider 不可用时**允许** fail-closed，所以本测试**不**断言
//! 「旋钮开就一定成功」，只断言「旋钮开 + 有真实 provider ⇒ 不再是 unavailable」。
//!
//! # 「真实 provider」是什么意思
//!
//! 真 provider 插件（`MinimaxProviderPlugin`，生产用的那个）指向一个**本地
//! OpenAI 兼容端点**（测试夹具）⇒ 走的是生产同一条 HTTP + 凭据 + 解析路径。
//! 夹具**只**当厂商端点，不当 provider：监督者仍然必须是真实 provider 回答，
//! 且回答里必须逐字带上请求里的 `review_id` 与 `action_arguments_digest`
//! （否则复核判它 malformed ⇒ fail-closed）。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_cli::llm_factory_env::{bind_system_one_supervisor, system_one_gate_from_env};
use apeireth_cli::{system_one_enabled_from_env, ENABLE_SYSTEM_ONE_ENV};
use apeireth_core::kernel::{CapabilityId, PluginId, SessionId};
use apeireth_governance::AllowAll;
use apeireth_plugin::agents::{AgentBudget, AgentConfig};
use apeireth_plugin::module_contract::ExecutionMode;
use apeireth_plugin::{CredentialResolver, ToolCapability, ToolModule};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use apeireth_provider::canonical_minimax::MinimaxProviderPlugin;
use apeireth_runtime::canonical::system_one::SystemOneGate;
use apeireth_runtime::{AgentHostPolicy, Runtime, SessionEventKind};
use async_trait::async_trait;
use serde_json::{json, Value};

/// 本测试会动的 env（旋钮 + 厂商凭据 + 厂商端点）。
const ENV_KEYS: &[&str] = &[
    ENABLE_SYSTEM_ONE_ENV,
    "APEIRETH_API_KEY",
    "APEIRETH_API_URL",
    "APEIRETH_API_MODELS",
    "APEIRETH_MODEL",
    "OPENAI_API_KEY",
    "APEIRETH_ANTHROPIC_KEY",
];

/// env 是进程全局的 ⇒ 本文件内所有改 env 的测试共用一把锁。
static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    previous: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn guard(keys: &'static [&'static str]) -> Self {
        let previous = keys
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        let guard = Self { previous };
        for key in keys {
            std::env::remove_var(key);
        }
        guard
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, previous) in &self.previous {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 复核审计账本的文件名（与装配面同一个常量口径）。
const AUDIT_FILE: &str = "system-one-review.jsonl";

/// 「无门」那条路径的**逐字**文案（接线前的原样）。
const NO_GATE_MESSAGE: &str =
    "System One is required for internal execution; supervisor not configured";

// ---------------------------------------------------------------------------
// 内部面工具（Protected 效果 ⇒ 走强制复核接缝）
// ---------------------------------------------------------------------------

struct ProtectedWrite {
    id: CapabilityId,
    entered: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for ProtectedWrite {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("protected_write").with_parameters(
            json!({"type":"object","properties":{"value":{"type":"string"}},"required":["value"],"additionalProperties":false})
                .as_object()
                .unwrap()
                .clone(),
        )
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.entered.fetch_add(1, Ordering::AcqRel);
        ToolResult::ok(&call.id, json!({"written": true})).with_name(&call.name)
    }
}

/// 内部面工具模块（`with_public_contract(Direct)` ⇒ 效果为 Protected ⇒ 必走复核）。
fn protected_module(entered: Arc<AtomicUsize>) -> ToolModule {
    let tool = Arc::new(ProtectedWrite {
        id: CapabilityId::new("tool.protected_write").expect("capability id"),
        entered,
    });
    ToolModule::new(
        PluginId::new("module.protected").expect("plugin id"),
        vec![tool],
    )
    .expect("module")
    .with_public_contract(ExecutionMode::Direct)
    .expect("public contract")
}

// ---------------------------------------------------------------------------
// 厂商端点夹具（只当端点，不当 provider）
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct VendorStub {
    main_calls: Arc<AtomicUsize>,
    review_calls: Arc<AtomicUsize>,
}

/// 复核请求 = 用户消息里那份 `SystemOneRequest` JSON（带 `review_id`）。
///
/// 夹具按**内容**判定而不是按请求序号判定 ⇒ 复核请求插在主回合中间也不会认错。
fn review_request(body: &Value) -> Option<Value> {
    for message in body["messages"].as_array()? {
        if message["role"].as_str()? != "user" {
            continue;
        }
        let content = message["content"].as_str()?;
        let start = content.find('{')?;
        let end = content.rfind('}')?;
        let parsed: Value = serde_json::from_str(&content[start..=end]).ok()?;
        if parsed.get("review_id").is_some() {
            return Some(parsed);
        }
    }
    None
}

async fn vendor_chat(
    axum::extract::State(state): axum::extract::State<VendorStub>,
    axum::Json(body): axum::Json<Value>,
) -> axum::Json<Value> {
    if let Some(request) = review_request(&body) {
        state.review_calls.fetch_add(1, Ordering::SeqCst);
        // 判决必须**逐字**复制请求里的标识（复核会校验 version / policy_version /
        // review_id / action_arguments_digest 四项）。
        let verdict = json!({
            "version": request["version"],
            "policy_version": request["policy_version"],
            "review_id": request["review_id"],
            "action_arguments_digest": request["action_arguments_digest"],
            "decision": "allow",
        });
        return axum::Json(json!({
            "id": "review",
            "model": body["model"],
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": verdict.to_string()},
                "finish_reason": "stop",
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1},
        }));
    }
    let first = state.main_calls.fetch_add(1, Ordering::SeqCst) == 0;
    let (message, finish) = if first {
        (
            json!({
                "role": "assistant",
                "content": "",
                "tool_calls": [{
                    "id": "call-1",
                    "type": "function",
                    "function": {"name": "protected_write", "arguments": "{\"value\":\"sentinel\"}"},
                }],
            }),
            "tool_calls",
        )
    } else {
        (json!({"role": "assistant", "content": "done"}), "stop")
    };
    axum::Json(json!({
        "id": "chat",
        "model": body["model"],
        "choices": [{"index": 0, "message": message, "finish_reason": finish}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1},
    }))
}

/// 起一个本地 OpenAI 兼容端点，返回它的基址。
async fn start_vendor_stub(state: VendorStub) -> String {
    use axum::routing::post;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("addr");
    let app = axum::Router::new()
        .route("/chat/completions", post(vendor_chat))
        .with_state(state);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    format!("http://{address}")
}

// ---------------------------------------------------------------------------
// 运行时装配（与生产组合根同一形状：托管身份 + 受限预算）
// ---------------------------------------------------------------------------

fn desktop_budget() -> AgentBudget {
    AgentBudget {
        max_requests: u64::MAX,
        max_tokens: u64::MAX,
        max_tool_calls: u64::MAX,
        max_cost_micros: None,
        turn_timeout_ms: u64::MAX,
    }
}

/// 造一个带**真 provider 插件**与内部面工具的运行时。
///
/// `gate` = `Some` ⇒ 复核门已装上（旋钮开时的生产形状）；`None` ⇒ 与接线前同形状。
async fn runtime_with(
    gate: Option<Arc<SystemOneGate>>,
    resolver: Arc<dyn CredentialResolver>,
    entered: Arc<AtomicUsize>,
) -> Runtime {
    let provider = MinimaxProviderPlugin::from_env().expect("minimax provider plugin");
    let mut builder = Runtime::builder()
        .with_default_model("MiniMax-M3")
        .with_governance(Arc::new(AllowAll))
        .with_credentials(resolver)
        .with_plugin(Arc::new(provider))
        .with_plugin(Arc::new(protected_module(entered)));
    if let Some(gate) = gate {
        builder = builder.with_system_one(gate);
    }
    let mut policy = AgentHostPolicy::default();
    policy.max_budget = desktop_budget();
    let host = builder
        .build_agent_runtime(policy)
        .await
        .expect("agent runtime");
    host.into_main_runtime_with_session_quotas(AgentConfig {
        tools: vec![CapabilityId::new("tool.protected_write").expect("capability id")],
        budget: desktop_budget(),
        ..Default::default()
    })
    .await
    .expect("main runtime")
}

/// 跑一轮并取回那次工具调用的结果（失败时是那次的错误）。
async fn dispatch_internal(runtime: &Runtime, session: SessionId) -> ToolResult {
    runtime
        .execute(apeireth_runtime::TurnRequest::new(
            session,
            "request the registered operation",
        ))
        .await
        .expect("turn");
    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .expect("session load")
        .expect("session");
    stored
        .events
        .iter()
        .find_map(|entry| match &entry.event {
            SessionEventKind::ToolEventRecorded { fact } => Some(fact.clone()),
            _ => None,
        })
        .expect("tool event")
        .result()
        .clone()
}

/// 审计账本里那条复核记录的状态。
fn audit_status(path: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let last = text.lines().last()?;
    let record: Value = serde_json::from_str(last).ok()?;
    record
        .get("status")
        .and_then(|status| status.as_str())
        .map(ToString::to_string)
}

/// 起一个带凭据的厂商端点（主回合的模型要能真说话）。
async fn vendor_with_key() -> VendorStub {
    let stub = VendorStub::default();
    let base_url = start_vendor_stub(stub.clone()).await;
    std::env::set_var("APEIRETH_API_URL", &base_url);
    std::env::set_var("APEIRETH_API_KEY", "sk-local-vendor-key");
    stub
}

// ---------------------------------------------------------------------------
// ① 旋钮关 ⇒ 与接线前逐字一致（零行为变化）
// ---------------------------------------------------------------------------

/// 旋钮默认关 ⇒ 组合根**不构造**复核门 ⇒ 内部执行走的还是「无门」那条路径。
#[tokio::test]
async fn the_knob_is_off_by_default_and_keeps_the_pre_wiring_behavior() {
    let _lock = lock_env();
    let _env = EnvGuard::guard(ENV_KEYS);
    let dir = tempfile::tempdir().expect("temp dir");

    assert!(!system_one_enabled_from_env(), "旋钮默认关");
    assert!(
        system_one_gate_from_env(dir.path()).await.is_none(),
        "旋钮关 ⇒ 不构造复核门 ⇒ 组合根不调 with_system_one"
    );
    assert!(
        !dir.path().join(AUDIT_FILE).exists(),
        "旋钮关 ⇒ 连审计账本都不落盘（零副作用）"
    );
}

/// 旋钮关时内部执行的失败**逐字**等于接线前（无门那条路径的文案）。
#[tokio::test]
async fn the_knob_off_internal_execution_fails_exactly_as_before_wiring() {
    let _lock = lock_env();
    let _env = EnvGuard::guard(ENV_KEYS);
    let _stub = vendor_with_key().await;
    let entered = Arc::new(AtomicUsize::new(0));
    let resolver = Arc::new(apeireth_provider::credentials::EnvCredentialResolver::new());
    // 组合根在旋钮关时**不会**调 with_system_one ⇒ 这里刻意不装门。
    let runtime = runtime_with(None, resolver, entered.clone()).await;

    let result = dispatch_internal(&runtime, SessionId::new()).await;

    assert!(!result.is_ok(), "内部执行必须被拒");
    let rendered = result.render();
    assert!(
        rendered.contains("system_one_unavailable"),
        "错误码必须仍是接线前的那个: {rendered}"
    );
    assert!(
        rendered.contains(NO_GATE_MESSAGE),
        "错误文案必须**逐字**等于接线前（无门那条路径）: {rendered}"
    );
    assert_eq!(entered.load(Ordering::Acquire), 0, "不许有任何副作用");
}

// ---------------------------------------------------------------------------
// ② 旋钮开 + 有真实 provider ⇒ 内部执行不再得 SystemOneUnavailable
// ---------------------------------------------------------------------------

/// 旋钮开 + **真 provider**（真插件 + 真凭据 + 真 HTTP 路径）⇒ 复核真的跑了，
/// 内部执行不再被 `system_one_unavailable` 挡下。
#[tokio::test]
async fn the_knob_on_with_a_real_provider_passes_the_mandatory_review() {
    let _lock = lock_env();
    let _env = EnvGuard::guard(ENV_KEYS);
    let dir = tempfile::tempdir().expect("temp dir");
    let stub = vendor_with_key().await;
    std::env::set_var(ENABLE_SYSTEM_ONE_ENV, "1");

    assert!(system_one_enabled_from_env(), "旋钮开");
    let gate = system_one_gate_from_env(dir.path())
        .await
        .expect("旋钮开 ⇒ 复核门必须装配出来");
    let resolver: Arc<dyn CredentialResolver> =
        Arc::new(apeireth_provider::credentials::EnvCredentialResolver::new());
    let entered = Arc::new(AtomicUsize::new(0));
    let runtime = runtime_with(Some(gate.clone()), resolver.clone(), entered.clone()).await;

    assert!(
        bind_system_one_supervisor(&runtime, &gate, resolver.as_ref()).await,
        "有真实凭据 ⇒ 必须绑上真实监督 provider"
    );

    let result = dispatch_internal(&runtime, SessionId::new()).await;

    assert!(
        !result.render().contains("system_one_unavailable"),
        "有真实 provider 时内部执行不再得 system_one_unavailable: {}",
        result.render()
    );
    assert!(
        result.is_ok(),
        "复核放行 ⇒ 工具真的执行了: {}",
        result.render()
    );
    assert_eq!(entered.load(Ordering::Acquire), 1, "副作用恰好发生一次");
    assert_eq!(
        stub.review_calls.load(Ordering::Acquire),
        1,
        "复核真的问过监督者"
    );
    assert_eq!(
        audit_status(&dir.path().join(AUDIT_FILE)).as_deref(),
        Some("allowed"),
        "复核判决必须落进审计账本"
    );
}

// ---------------------------------------------------------------------------
// ③ 旋钮开 + 没有真实 provider ⇒ fail-closed
// ---------------------------------------------------------------------------

/// 旋钮开但**没有真实凭据** ⇒ 宿主**拒绝**绑定监督者（绝不塞假 provider 顶替）
/// ⇒ 门保持无监督 ⇒ 这一轮什么都不会发生（fail-closed：旋钮开不等于放行）。
#[tokio::test]
async fn the_knob_on_without_a_real_provider_fails_closed() {
    let _lock = lock_env();
    let _env = EnvGuard::guard(ENV_KEYS);
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var(ENABLE_SYSTEM_ONE_ENV, "1");

    let gate = system_one_gate_from_env(dir.path())
        .await
        .expect("旋钮开 ⇒ 复核门必须装配出来");
    let resolver: Arc<dyn CredentialResolver> =
        Arc::new(apeireth_provider::credentials::EnvCredentialResolver::new());
    let entered = Arc::new(AtomicUsize::new(0));
    // 没有凭据 ⇒ 主回合的模型也答不出来 ⇒ 这一轮什么都不会发生。
    let runtime = runtime_with(Some(gate.clone()), resolver.clone(), entered.clone()).await;

    assert!(
        !bind_system_one_supervisor(&runtime, &gate, resolver.as_ref()).await,
        "没有真实凭据 ⇒ 绝不装配监督 provider（更不许塞假 provider 顶替）"
    );
    assert!(
        runtime
            .execute(apeireth_runtime::TurnRequest::new(
                SessionId::new(),
                "request the registered operation",
            ))
            .await
            .is_err(),
        "没有真实 provider ⇒ 这一轮必须失败（fail-closed）"
    );
    assert_eq!(entered.load(Ordering::Acquire), 0, "不许有任何副作用");
}

/// 门已装但**没有绑定监督者**（= 上一个用例绑定失败后运行时留下的状态）
/// ⇒ 内部执行被复核接缝拒绝（**不**因为「门装上了」就放行）。
#[tokio::test]
async fn a_gate_without_a_bound_supervisor_refuses_internal_execution() {
    let _lock = lock_env();
    let _env = EnvGuard::guard(ENV_KEYS);
    let dir = tempfile::tempdir().expect("temp dir");
    let _stub = vendor_with_key().await;
    std::env::set_var(ENABLE_SYSTEM_ONE_ENV, "1");

    let gate = system_one_gate_from_env(dir.path())
        .await
        .expect("旋钮开 ⇒ 复核门必须装配出来");
    let resolver: Arc<dyn CredentialResolver> =
        Arc::new(apeireth_provider::credentials::EnvCredentialResolver::new());
    let entered = Arc::new(AtomicUsize::new(0));
    let runtime = runtime_with(Some(gate.clone()), resolver.clone(), entered.clone()).await;
    // 刻意**不**绑监督者（宿主绑定失败后的状态）。

    let result = dispatch_internal(&runtime, SessionId::new()).await;

    assert!(!result.is_ok(), "没有监督者 ⇒ 内部执行必须被拒");
    assert!(
        result.render().contains("system_one_unavailable"),
        "fail-closed 的错误码: {}",
        result.render()
    );
    assert_eq!(entered.load(Ordering::Acquire), 0, "不许有任何副作用");
}
