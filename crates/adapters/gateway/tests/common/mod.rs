//! IM 快捷接入测试共用夹具 (确定性 provider / 工具 / 治理策略 / 信封样例)。
//!
//! 只有 provider 与工具是测试夹具; 运行时 / 插件管理 / 治理管线 / 会话存储
//! 都是真实现。所有外部端点用本地 mock (wiremock), 0 真实网络。
#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::clock::{Clock, VirtualClock};
use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId, Timestamp};
use apeireth_governance::{
    AllowAll, GovernanceHook, GovernancePipeline, GovernanceRequest, Permission,
    PermissionGovernanceHook, PermissionPolicy,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedTool, NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime, SessionStore};
use async_trait::async_trait;

pub const MODEL: &str = "fake-model-1";
pub const TOOL: &str = "calculator";
pub const CAPABILITY: &str = "tool.calculator";
pub const FINAL_TEXT: &str = "The result is 2.";
pub const CONVERSATION: &str = "oc_demo_chat";

/// 固定起点时钟 (可 set 快进, 与运行时同一时间口径)。
pub fn test_clock() -> Arc<VirtualClock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

/// 需审批的治理策略: 工具已授权, 但每次派发都要人批。
pub fn approval_policy() -> GovernancePipeline {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool(CAPABILITY.into()));
    policy.require_approval_for(CAPABILITY);
    GovernancePipeline::new().with(Arc::new(PermissionGovernanceHook::new(policy)))
}

/// 全放行治理 (无审批路径)。
pub fn allow_all() -> Arc<dyn GovernanceHook> {
    Arc::new(AllowAll)
}

/// 确定性 provider: 首轮发一个工具调用, 次轮收尾文本。
pub struct FakeProvider {
    id: CapabilityId,
    calls: AtomicUsize,
}

impl FakeProvider {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            calls: AtomicUsize::new(0),
        })
    }

    pub fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ProviderCapability for FakeProvider {
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
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let mut response = NormalizedResponse {
            id: format!("response-{}", call + 1),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        match call {
            0 => {
                response.finish_reason = Some(NormalizedFinishReason::ToolCalls);
                response.tool_calls.push(ToolCall {
                    id: "call-1".into(),
                    name: TOOL.into(),
                    arguments: serde_json::json!({ "a": 1, "b": 1 }),
                });
            }
            // 后续轮次都是收尾文本 (允许同一会话继续对话)。
            _ => response.content = FINAL_TEXT.into(),
        }
        Ok(response)
    }
}

/// 计数工具 (是否执行 = 治理闭合的副作用证据)。
pub struct Calculator {
    id: CapabilityId,
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolCapability for Calculator {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: TOOL.into(),
            description: Some("Add two integers".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.calls.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, serde_json::json!("2"))
    }
}

/// 夹具插件 (provider + 工具)。
pub struct TestPlugin {
    manifest: PluginManifest,
    provider: Option<Arc<FakeProvider>>,
    tool_calls: Option<Arc<AtomicUsize>>,
}

impl TestPlugin {
    pub fn provider(provider: Arc<FakeProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("test.fake_provider").unwrap(),
                "1.0.0",
                "deterministic provider",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "fake completion provider",
            )
            .unwrap(),
            provider: Some(provider),
            tool_calls: None,
        })
    }

    pub fn calculator(calls: Arc<AtomicUsize>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("test.calculator").unwrap(),
                "1.0.0",
                "deterministic calculator",
            )
            .declare_capability(
                CapabilityId::new(CAPABILITY).unwrap(),
                CapabilityKind::Tool,
                "add two integers",
            )
            .unwrap(),
            provider: None,
            tool_calls: Some(calls),
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
        self.tool_calls
            .as_ref()
            .map(|calls| {
                vec![Arc::new(Calculator {
                    id: CapabilityId::new(CAPABILITY).unwrap(),
                    calls: Arc::clone(calls),
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

/// 装一座确定性运行时 (治理策略可换: 放行 / 需审批)。
pub async fn build_runtime(
    clock: Arc<dyn Clock>,
    governance: Arc<dyn GovernanceHook>,
    provider: Arc<FakeProvider>,
    tool_calls: Arc<AtomicUsize>,
) -> (Arc<Runtime>, Arc<dyn SessionStore>) {
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let runtime = Runtime::builder()
        .with_clock(clock)
        .with_session_store(store.clone())
        .with_governance(governance)
        .with_plugin(TestPlugin::provider(provider.clone()))
        .with_plugin(TestPlugin::calculator(tool_calls.clone()))
        .with_default_model(MODEL)
        .build()
        .await
        .unwrap();
    (Arc::new(runtime), store)
}

/// 固定会话 id (映射表断言用)。
pub fn fixed_session() -> SessionId {
    SessionId::from_uuid(uuid::Uuid::from_u128(4242))
}

/// `im-feishu` 入站文本消息信封 (消费 webhook 信封形状)。
pub fn feishu_text_event(conversation: &str, text: &str) -> String {
    serde_json::json!({
        "header": {"event_type": "im.message.receive_v1", "app_id": "app", "token": "t"},
        "event": {
            "message": {
                "chat_id": conversation,
                "message_id": format!("om_{}", text.len()),
                "content": serde_json::json!({"text": text}).to_string(),
            },
            "sender": {"sender_id": {"open_id": "ou_demo"}},
        }
    })
    .to_string()
}

/// `im-wecom` 入站文本消息信封。
pub fn wecom_text_event(conversation: &str, text: &str) -> String {
    serde_json::json!({
        "msgtype": "text",
        "text": {"content": text},
        "chatid": conversation,
        "msgid": format!("msg_{}", text.len()),
        "from": {"userid": "u_demo"},
    })
    .to_string()
}

/// `im-wecom` 入站按钮回调信封 (载荷即卡片按钮往返值)。
pub fn wecom_button_event(conversation: &str, payload: &serde_json::Value) -> String {
    serde_json::json!({
        "action": "callback",
        "action_value": payload.to_string(),
        "chatid": conversation,
        "msgid": "msg_button",
        "operator": "u_demo",
    })
    .to_string()
}
