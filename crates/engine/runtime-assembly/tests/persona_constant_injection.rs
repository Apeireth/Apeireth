//! V4-a 恒定小注入的装配级两态证明 (真 canonical turn 机器, keyless)。
//!
//! 两态合同:
//! - **有人格层且有自我叙事沉淀** = `TurnStart` 把自我叙事摘要注入提示词
//!   (≤ [`apeireth_memory::CONSTANT_INJECTION_BUDGET_CHARS`] 字符恒预算);
//! - **缺人格层** = 零注入零变化 —— 发给 Provider 的请求消息**逐字节**与不装
//!   本模块时一致 (对比捕获的请求消息序列化字节)。

use apeireth_runtime_assembly as apeireth_runtime;

use std::sync::{Arc, Mutex};

use apeireth_core::clock::SystemClock;
use apeireth_core::kernel::{CapabilityId, Clock, ModelId, PluginId, SessionId};
use apeireth_governance::{Decision, GovernanceHook, GovernanceRequest};
use apeireth_memory::{InMemoryPersonaMemoryStore, PersonaMemory, PersonaMemoryStore};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, NormalizedFinishReason, NormalizedRequest, NormalizedResponse, NormalizedUsage,
};
use apeireth_runtime::canonical::{Runtime, TurnOutcome, TurnRequest};
use apeireth_runtime_assembly::{
    CognitiveBackends, PersonaConstantInjectionModule, ProductionModules, ProductionModulesConfig,
    PERSONA_INJECTION_MODULE_ID,
};
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";

// ---------------------------------------------------------------------
// 捕获 Provider: 只记录发往模型的请求消息字节 (逐字对比的取证面)
// ---------------------------------------------------------------------

struct CapturingProvider {
    id: CapabilityId,
    requests: Mutex<Vec<String>>,
}

impl CapturingProvider {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            requests: Mutex::new(Vec::new()),
        })
    }

    fn captured(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProviderCapability for CapturingProvider {
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
        let bytes = serde_json::to_string(&request.messages).expect("messages serialize");
        let mut requests = self.requests.lock().unwrap();
        requests.push(bytes);
        let call = requests.len();
        Ok(NormalizedResponse {
            id: format!("response-{call}"),
            model: request.model.clone(),
            content: "ok".to_string(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::default(),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<CapturingProvider>,
}

impl ProviderPlugin {
    fn new(provider: Arc<CapturingProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("builtin.fake_provider").unwrap(),
                "1.0.0",
                "fake provider",
            )
            .declare_capability(
                provider.id.clone(),
                CapabilityKind::Provider,
                "fake provider",
            )
            .unwrap(),
            provider,
        })
    }
}

#[async_trait]
impl Plugin for ProviderPlugin {
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

struct AllowAll;

#[async_trait]
impl GovernanceHook for AllowAll {
    fn name(&self) -> &str {
        "allow-all"
    }

    async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
        Decision::Allow
    }
}

async fn build_runtime(
    provider: Arc<CapturingProvider>,
    modules: Vec<Arc<dyn apeireth_runtime::canonical::AgentModule>>,
) -> Runtime {
    let mut builder = Runtime::builder()
        .with_default_model(MODEL)
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(provider));
    for module in modules {
        builder = builder.with_module(module);
    }
    builder.build().await.unwrap()
}

// ---------------------------------------------------------------------
// 两态之一: 有人格层且有沉淀 = 注入进 TurnStart 提示词
// ---------------------------------------------------------------------

#[tokio::test]
async fn with_persona_sediment_the_constant_injection_enters_the_turn_start_prompt() {
    let persona: Arc<dyn PersonaMemoryStore> = Arc::new(InMemoryPersonaMemoryStore::new());
    let mut memory = PersonaMemory::default();
    memory.self_narrative = "我是一个先验证再叙事的记忆体。".to_string();
    persona.save(&memory).unwrap();
    let expected = apeireth_memory::build_constant_injection(&memory);

    let provider = CapturingProvider::new();
    let runtime = build_runtime(
        provider.clone(),
        vec![Arc::new(PersonaConstantInjectionModule::new(Arc::clone(
            &persona,
        )))],
    )
    .await;
    let outcome = runtime
        .execute_outcome(TurnRequest::new(SessionId::new(), "turn one"))
        .await
        .unwrap();
    assert!(matches!(outcome, TurnOutcome::Completed(_)));

    let captured = provider.captured();
    assert_eq!(captured.len(), 1, "一回合一拍主 round");
    // 逐字面: 解回消息表, 注入文本与回取面纯渲染完全一致。
    let messages: Vec<apeireth_protocol::canonical::NormalizedMessage> =
        serde_json::from_str(&captured[0]).expect("messages parse");
    let texts: Vec<String> = messages
        .iter()
        .map(|message| apeireth_protocol::canonical::ContentPart::join_text(&message.content))
        .collect();
    assert!(
        texts.contains(&expected),
        "恒定自述必须逐字进 TurnStart 提示词: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|text| text.contains("【我是谁 · 恒定自述】")),
        "{texts:?}"
    );
}

// ---------------------------------------------------------------------
// 两态之二: 缺人格层 = 零注入零变化 (Provider 请求消息逐字节一致)
// ---------------------------------------------------------------------

#[tokio::test]
async fn without_a_persona_layer_the_provider_request_is_byte_identical() {
    // 基线: 不装恒定注入模块 (缺人格层的装配形态)。
    let baseline_provider = CapturingProvider::new();
    let baseline = build_runtime(baseline_provider.clone(), Vec::new()).await;
    baseline
        .execute_outcome(TurnRequest::new(SessionId::new(), "turn one"))
        .await
        .unwrap();

    // 空人格层 (无自我叙事沉淀): 模块在场也不发 overlay。
    let empty_provider = CapturingProvider::new();
    let empty_persona: Arc<dyn PersonaMemoryStore> = Arc::new(InMemoryPersonaMemoryStore::new());
    let empty = build_runtime(
        empty_provider.clone(),
        vec![Arc::new(PersonaConstantInjectionModule::new(empty_persona))],
    )
    .await;
    empty
        .execute_outcome(TurnRequest::new(SessionId::new(), "turn one"))
        .await
        .unwrap();

    let baseline_bytes = baseline_provider.captured();
    let empty_bytes = empty_provider.captured();
    assert_eq!(baseline_bytes.len(), 1);
    assert_eq!(empty_bytes.len(), 1);
    assert_eq!(
        baseline_bytes[0], empty_bytes[0],
        "缺沉淀/缺人格层必须逐字零变化"
    );
    assert!(
        !empty_bytes[0].contains("【我是谁 · 恒定自述】"),
        "无沉淀不得编造身份: {}",
        empty_bytes[0]
    );
}

// ---------------------------------------------------------------------
// 装配缝: 恒定注入模块只在人格层在场时进 TurnStart 装配点
// ---------------------------------------------------------------------

#[tokio::test]
async fn production_assembly_registers_the_injection_module_only_with_a_persona_layer() {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    // 最小配置: 记忆/偏好槽默认关 (本用例只看恒定注入装配缝)。
    let config = ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    };

    // 缺人格层: 模块不注册 (零注入零变化)。
    let absent = ProductionModules::build(
        config.clone(),
        CognitiveBackends::default(),
        Arc::clone(&clock),
    )
    .expect("assembly builds");
    assert!(
        !absent
            .ids()
            .contains(&PERSONA_INJECTION_MODULE_ID.to_string()),
        "缺人格层不得注册恒定注入模块: {:?}",
        absent.ids()
    );

    // 有人格层: TurnStart 装配点收进恒定注入模块。
    let present = ProductionModules::build(
        config,
        CognitiveBackends {
            persona_layer: Some(Arc::new(InMemoryPersonaMemoryStore::new())),
            ..CognitiveBackends::default()
        },
        clock,
    )
    .expect("assembly builds");
    assert!(
        present
            .ids()
            .contains(&PERSONA_INJECTION_MODULE_ID.to_string()),
        "有人格层必须注册恒定注入模块: {:?}",
        present.ids()
    );
}
