//! V3-a 双引擎主动关怀的装配级证明 (`tool.care_schedule`)。
//!
//! 走真 [`ProductionModules`] 生产组装根 + 真 [`Runtime`] 标准回合:
//! 注册条件 / 能力名册行 / 自述关怀节 (规则数·下次触发·最近关怀) 对账真值;
//! 输出面走**标准回合** —— 双引擎判定产出的标准回合输入过 canonical agent
//! loop, 生成的关怀消息进会话流 (零新消息通道), 计费落会话配额账本
//! (与三维烧钱闸同一本账, 照常计费; 烧不起即帧, 无免费通道)。

#![allow(dead_code)] // D-1（owner 裁定 2026-10-25）：测试夹具字段/方法（供用例构造完整对象，部分字段本文件内未被读取）
use std::sync::Arc;

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_governance::AllowAll;
use apeireth_organ::care::{
    CareDiscipline, CareEngine, CarePoll, CareRule, CareStore, CareTrigger, FrozenCareClock,
    RuleFallbackJudge,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse,
    NormalizedUsage, ToolCall,
};
use apeireth_runtime::canonical::quota::{QuotaDimension, SessionQuotaLimits};
use apeireth_runtime::canonical::{Runtime, RuntimeError, TurnRequest};
use apeireth_runtime_assembly::{
    care_status_probe, ProductionBackends, ProductionModules, ProductionModulesConfig,
};
use serde_json::{json, Value};

const MODEL: &str = "fake-model-care";

struct FakeProvider {
    id: CapabilityId,
    text: &'static str,
    calls: std::sync::atomic::AtomicUsize,
}

impl FakeProvider {
    fn new(text: &'static str) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            text,
            calls: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
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
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(NormalizedResponse {
            id: "resp_care".to_string(),
            model: request.model.clone(),
            content: self.text.to_string(),
            finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                prompt_cache_hit_tokens: None,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<FakeProvider>,
}

impl ProviderPlugin {
    fn new(provider: Arc<FakeProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("vendor.fake").unwrap(),
                "1.0.0",
                "Fake provider",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "Scripted completions",
            )
            .unwrap(),
            provider,
        })
    }
}

#[async_trait::async_trait]
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

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_786_838_400_000)
            .unwrap()
            .as_datetime(),
    ))
}

fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

async fn invoke_self_status(modules: &ProductionModules) -> Value {
    let self_status = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.self_status")
        .cloned()
        .expect("tool.self_status");
    let call = ToolCall {
        id: "call_status".into(),
        name: "self_status".into(),
        arguments: json!({}),
    };
    let result = self_status.invoke(&call).await;
    serde_json::from_str(&result.render()).expect("self report json")
}

/// 注册条件 + 能力名册行: 数据目录注入才注册, 名册照实报生效值。
#[tokio::test]
async fn care_schedule_registers_only_when_configured_and_the_roster_says_so() {
    let mut off = base_config();
    off.care_schedule = None;
    let modules = ProductionModules::build(off, ProductionBackends::default(), frozen_clock())
        .expect("build");
    assert!(modules
        .capabilities()
        .iter()
        .all(|capability| capability.id().as_str() != "tool.care_schedule"));
    let report = invoke_self_status(&modules).await;
    assert_eq!(report["capabilities"]["care_schedule"], false);

    let dir = tempfile::tempdir().expect("tempdir");
    let mut on = base_config();
    on.care_schedule = Some(dir.path().to_path_buf());
    let modules =
        ProductionModules::build(on, ProductionBackends::default(), frozen_clock()).expect("build");
    let ids: Vec<String> = modules
        .capabilities()
        .iter()
        .map(|capability| capability.id().to_string())
        .collect();
    assert!(
        ids.iter().any(|id| id == "tool.care_schedule"),
        "missing tool.care_schedule: {ids:?}"
    );
    let report = invoke_self_status(&modules).await;
    assert_eq!(report["capabilities"]["care_schedule"], true);
}

/// 自述关怀节: 规则数/下次触发/最近关怀取自 `care_rules.json` 真值 (如实)。
#[tokio::test]
async fn self_status_care_section_reports_real_rule_counts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = CareStore::new(dir.path());
    store
        .add_rule(CareRule {
            id: "check-in".to_string(),
            trigger: CareTrigger::Interval { hours: 6 },
            material: "问问他最近怎么样".to_string(),
            enabled: true,
            created_at_ms: 1_786_838_400_000,
        })
        .expect("add");
    store
        .add_rule(CareRule {
            id: "morning".to_string(),
            trigger: CareTrigger::DailyMoment {
                minutes_of_day: 8 * 60 + 30,
            },
            material: "问候早安".to_string(),
            enabled: true,
            created_at_ms: 1_786_838_400_000,
        })
        .expect("add");

    let probe_dir = dir.path().to_path_buf();
    let backends = ProductionBackends {
        self_status_care: Some(care_status_probe(
            probe_dir,
            Arc::new(|| 1_786_838_400_000),
            Arc::new(|| None),
        )),
        ..ProductionBackends::default()
    };
    let modules = ProductionModules::build(base_config(), backends, frozen_clock()).expect("build");
    let report = invoke_self_status(&modules).await;
    let section = &report["care"];
    assert_eq!(section["rules"], 2);
    assert_eq!(
        section["next_trigger_at_ms"], 1_786_860_000_000i64,
        "下次触发 = 两条里最早的一条 (间隔规则 6 小时后 = 创建时刻 + 6h)"
    );
    assert_eq!(section["last_care_at_ms"], Value::Null);
    assert_eq!(section["last_care_note"], "尚无关怀记录");
    assert_eq!(section["reason"], Value::Null);
}

/// 自述关怀节: 拒开 (坏 `care_rules.json`) = 显式 null + 真因 (不编数)。
#[tokio::test]
async fn self_status_care_section_is_null_with_a_reason_when_the_store_refuses() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("care_rules.json"), "{ not json").expect("write");
    let probe_dir = dir.path().to_path_buf();
    let backends = ProductionBackends {
        self_status_care: Some(care_status_probe(
            probe_dir,
            Arc::new(|| 1_786_838_400_000),
            Arc::new(|| None),
        )),
        ..ProductionBackends::default()
    };
    let modules = ProductionModules::build(base_config(), backends, frozen_clock()).expect("build");
    let report = invoke_self_status(&modules).await;
    let section = &report["care"];
    assert_eq!(section["rules"], Value::Null);
    assert!(
        section["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("care_rules.json refused to open"),
        "{section}"
    );
}

/// 一次关怀发起的完整竖切: 双引擎判定 → 标准回合输入 → 真 Runtime 标准回合
/// → 生成的关怀消息进会话流 (零新消息通道)。
#[tokio::test]
async fn care_message_enters_the_conversation_flow_via_a_standard_turn() {
    // ① 双引擎: 规则引擎到点 → 发起意愿判定 → 标准回合输入。
    // 显式关窗隔离 (默认勿扰窗口 23:00-8:00 会拦下 01:00 的到点; 本竖切盯
    // 输出面, 不是勿扰纪律)。
    let dir = tempfile::tempdir().expect("tempdir");
    let store = CareStore::new(dir.path());
    store
        .add_rule(CareRule {
            id: "check-in".to_string(),
            trigger: CareTrigger::Interval { hours: 1 },
            material: "问问他最近怎么样".to_string(),
            enabled: true,
            created_at_ms: 1_786_838_400_000,
        })
        .expect("add");
    let mut body = store.list().expect("list");
    body.discipline = CareDiscipline {
        quiet_start_minutes: None,
        quiet_end_minutes: None,
        ..CareDiscipline::default()
    };
    store.save(&body).expect("save discipline");
    let clock = Arc::new(FrozenCareClock::new(1_786_838_400_000));
    let mut engine = CareEngine::new(store, clock.clone(), Arc::new(RuleFallbackJudge));
    clock.set(1_786_838_400_000 + 3_600_000);
    let initiative = match engine.poll().expect("poll") {
        CarePoll::Initiating(initiative) => initiative,
        other => panic!("关怀到点必须发起: {other:?}"),
    };
    let turn_input = initiative.to_turn_input();

    // ② 输出面 = 标准回合 (零新消息通道): 同一 canonical agent loop。
    let provider = FakeProvider::new("最近挺好, 别担心~");
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_default_model(MODEL)
        .build()
        .await
        .expect("runtime builds");
    let session_id = SessionId::new();
    let outcome = runtime
        .execute(TurnRequest::new(session_id, turn_input.clone()))
        .await
        .expect("the care turn completes");
    assert_eq!(outcome.text, "最近挺好, 别担心~");

    // ③ 会话流: 关怀回合输入与生成的关怀消息都进普通转写 (与人说的话同一条流)。
    let session = runtime
        .sessions()
        .load_or_create(session_id)
        .await
        .expect("session");
    let texts: Vec<String> = session
        .messages
        .iter()
        .map(|message| ContentPart::join_text(&message.content))
        .collect();
    assert!(
        texts
            .iter()
            .any(|text| text.as_str() == turn_input.as_str()),
        "关怀回合输入进会话流: {texts:?}"
    );
    assert!(
        texts.iter().any(|text| text.contains("最近挺好, 别担心~")),
        "生成的关怀消息进会话流: {texts:?}"
    );
}

/// 关怀预算 (同账本接缝): 关怀回合照常计费 —— 计量落会话配额账本 (三维
/// tokens/tool_steps/cost_micros 同一本账); 烧不起即帧 (无免费通道)。
#[tokio::test]
async fn a_care_turn_is_billed_on_the_shared_session_ledger_and_has_no_free_channel() {
    // ① 照常计费: 不设限也照计量 (自述列是真的)。
    let provider = FakeProvider::new("关怀消息");
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_default_model(MODEL)
        .build()
        .await
        .expect("runtime builds");
    let session_id = SessionId::new();
    runtime
        .execute(TurnRequest::new(session_id, "[care:check-in] 关怀回合输入"))
        .await
        .expect("the care turn completes");
    let stored = runtime
        .sessions()
        .load(&session_id)
        .await
        .expect("load")
        .expect("session row");
    assert_eq!(
        stored.quota.consumed_tokens, 15,
        "关怀回合照常计费: usage 10+5 记进会话配额账本 (与普通回合同一本账)"
    );

    // ② 无免费通道: 烧钱闸对关怀回合同语义 —— 账本耗尽即帧, 不再多烧一轮。
    let provider = FakeProvider::new("关怀消息");
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_default_model(MODEL)
        .with_session_quota(SessionQuotaLimits {
            max_tokens: Some(10),
            max_tool_steps: None,
            max_cost_micros: None,
        })
        .build()
        .await
        .expect("runtime builds");
    let session_id = SessionId::new();
    let error = runtime
        .execute(TurnRequest::new(session_id, "[care:check-in] 关怀回合输入"))
        .await
        .expect_err("关怀回合不豁免烧钱闸");
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
    assert!(consumed >= 15, "已耗是真数字: {consumed}");
}

/// 规则面工具的治理档在装配后仍由治理链承载 (工具面不自带放行): 工具本体
/// 拒收非法参数, 合法调用的写面归类如实。
#[tokio::test]
async fn the_care_schedule_tool_face_manages_rules_with_the_write_face_labelled() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut config = base_config();
    config.care_schedule = Some(dir.path().to_path_buf());
    let modules = ProductionModules::build(config, ProductionBackends::default(), frozen_clock())
        .expect("build");
    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.care_schedule")
        .cloned()
        .expect("tool.care_schedule");
    let added = tool
        .invoke(&ToolCall {
            id: "call_add".into(),
            name: "care_schedule".into(),
            arguments: json!({
                "action": "care_add",
                "id": "check-in",
                "trigger": { "kind": "inactive", "hours": 48 },
                "material": "问问他最近怎么样"
            }),
        })
        .await;
    let value: Value = serde_json::from_str(&added.render()).expect("json");
    assert_eq!(value["action"], "care_add");
    assert_eq!(value["write_face"], "rule_file_write");
    assert_eq!(value["rules"], 1);

    // 真落盘 (StoredDoc 信封), 自述节同源读到同一真值。
    let raw = std::fs::read_to_string(dir.path().join("care_rules.json")).expect("read");
    assert!(raw.contains("apeireth.care_rules"), "{raw}");
}
