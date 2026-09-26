//! Context-budget wiring: the assembled injected-context blocks are bounded by
//! a total character budget at the point they are composed into a provider
//! request. Core blocks (identity / system-convention / safety) are never
//! truncated; per-block caps apply before the total budget; non-core overflow is
//! cut greedily from the longest block first; below budget nothing changes.
//! With a bound spill sink a long-tail cut keeps a head/tail preview with a
//! retrieval guide and spills the full original; without one the overflow path
//! keeps the full content inline (prefer too long over lost information).

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_governance::AllowAll;
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest,
    NormalizedResponse, NormalizedUsage,
};
use apeireth_runtime::canonical::{
    budget_context_blocks, AgentModule, ContextBlock, HookPoint, ModuleContext, ModuleError,
    ModuleManifest, ModuleOutcome, Runtime, TurnRequest,
};
use async_trait::async_trait;

// ---------------------------------------------------------------------------
// Seam semantics: `budget_context_blocks` (the production budget primitive).
// ---------------------------------------------------------------------------

/// ① Over budget the longest (long-tail) non-core block is cut first while the
/// core block survives intact and a shorter non-core block is left untouched.
#[test]
fn over_budget_truncates_long_tail_and_preserves_core() {
    let core = ContextBlock::new("identity", "IDENTITY".repeat(10)).core(true); // 80 chars
    let mem = ContextBlock::new("memory", "M".repeat(400)); // longest non-core
    let lesson = ContextBlock::new("lesson", "L".repeat(100)); // shorter non-core
    let budget = 300;

    let out = budget_context_blocks(vec![core.clone(), mem.clone(), lesson.clone()], budget);

    let out_core = out.iter().find(|b| b.name == "identity").unwrap();
    assert_eq!(
        out_core.content, core.content,
        "core block must never be cut"
    );
    assert!(out_core.core, "core flag preserved");

    let out_mem = out.iter().find(|b| b.name == "memory").unwrap();
    assert!(
        out_mem.content.chars().count() < 400,
        "longest non-core block is the long tail that gets cut first"
    );

    let out_lesson = out.iter().find(|b| b.name == "lesson").unwrap();
    assert_eq!(
        out_lesson.content, lesson.content,
        "shorter non-core block survives once the long tail absorbs the cut"
    );

    let total: usize = out.iter().map(|b| b.content.chars().count()).sum();
    assert!(total <= budget, "total {total} must fit budget {budget}");
}

/// ② Below budget the blocks are returned byte-for-byte unchanged (zero-change
/// guarantee: small contexts are not truncated at all).
#[test]
fn below_budget_is_byte_identical() {
    let blocks = vec![
        ContextBlock::new("identity", "IDENTITY ".repeat(8)).core(true),
        ContextBlock::new("memory", "remembered fact"),
        ContextBlock::new("lesson", "lesson learned"),
        ContextBlock::new("organ", "organ product"),
    ];
    let budget = 100_000;

    let out = budget_context_blocks(blocks.clone(), budget);
    assert_eq!(out, blocks, "below budget the blocks must be identical");
}

/// Per-block `cap_chars` apply before the total budget: the capped block is
/// trimmed to its cap first, which then lets the total budget cut land on the
/// (now longest) uncapped block instead.
#[test]
fn per_block_cap_applies_before_total_budget() {
    // Raw total = 300 + 100 = 400 > 120, so the assembler runs.
    // Cap first: capped -> 50. Then total (50 + 100 = 150 > 120) cuts the
    // longest remaining non-core block (uncapped 100) by 30 -> 70. If the cap
    // had not applied first, the greedy total cut would have left capped at 20
    // and uncapped at 100 instead.
    let capped = ContextBlock::new("capped", "X".repeat(300)).with_cap(50);
    let uncapped = ContextBlock::new("uncapped", "Y".repeat(100));
    let budget = 120;

    let out = budget_context_blocks(vec![capped, uncapped], budget);
    let out_capped = out.iter().find(|b| b.name == "capped").unwrap();
    let out_uncapped = out.iter().find(|b| b.name == "uncapped").unwrap();
    assert_eq!(
        out_capped.content.chars().count(),
        50,
        "per-block cap must apply first"
    );
    assert_eq!(
        out_uncapped.content.chars().count(),
        70,
        "total budget then cuts the longest remaining non-core block"
    );
}

/// A core block that alone overflows the budget is still never truncated; the
/// non-core blocks are cut away entirely and filtered out.
#[test]
fn core_block_survives_even_when_it_alone_overflows() {
    let core = ContextBlock::new("identity", "C".repeat(200)).core(true);
    let mem = ContextBlock::new("memory", "M".repeat(80));
    let budget = 100;

    let out = budget_context_blocks(vec![core.clone(), mem], budget);
    let out_core = out.iter().find(|b| b.name == "identity").unwrap();
    assert_eq!(out_core.content, core.content, "core never truncated");
    assert!(
        !out.iter().any(|b| b.name == "memory"),
        "non-core block cut to empty is dropped"
    );
}

// ---------------------------------------------------------------------------
// End-to-end wiring: the budget is applied to the messages a provider receives.
// ---------------------------------------------------------------------------

const MODEL: &str = "fake-model-budget";

struct FakeProvider {
    id: CapabilityId,
    requests: Mutex<Vec<NormalizedRequest>>,
}

impl FakeProvider {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            requests: Mutex::new(Vec::new()),
        })
    }

    fn first_request(&self) -> NormalizedRequest {
        self.requests.lock().unwrap()[0].clone()
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
        self.requests.lock().unwrap().push(request.clone());
        Ok(NormalizedResponse {
            id: "response-0".into(),
            model: request.model.clone(),
            content: "ok".into(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::default(),
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

/// Injects transient system overlays at TurnStart (the shape used by the memory
/// / organ / lesson modules), simulating assembled injected-context blocks.
struct InjectOverlays {
    manifest: ModuleManifest,
    overlays: Vec<String>,
}

#[async_trait]
impl AgentModule for InjectOverlays {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook != HookPoint::TurnStart {
            return Ok(ModuleOutcome::continue_());
        }
        let mut outcome = ModuleOutcome::continue_();
        for text in &self.overlays {
            outcome = outcome.with_system_overlay(text.clone());
        }
        Ok(outcome)
    }
}

fn message_text(message: &apeireth_protocol::canonical::NormalizedMessage) -> String {
    ContentPart::join_text(&message.content)
}

/// The budget is wired into the provider request: with a small budget and a
/// bound spill sink, the long-tail overlay arrives as a head/tail preview with
/// an omission marker and a retrieval guide, while the core system / identity
/// block and the shorter overlay survive intact and the full original lands on
/// disk at the guided path.
#[tokio::test]
async fn provider_request_receives_budgeted_injected_context() {
    let provider = FakeProvider::new();
    let identity = "system identity safety preamble".to_string();
    let long = "L".repeat(2000);
    let short = "S".repeat(100);
    let budget = 400usize;
    let spill_root = tempfile::tempdir().unwrap();

    let module = InjectOverlays {
        manifest: ModuleManifest::new("cognitive.inject", "inject"),
        overlays: vec![long.clone(), short.clone()],
    };

    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
        .with_module(Arc::new(module))
        .with_context_budget_chars(budget)
        .with_context_spill_root(spill_root.path())
        .build()
        .await
        .unwrap();

    runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_system(identity.clone()))
        .await
        .unwrap();

    let request = provider.first_request();
    let texts: Vec<String> = request.messages.iter().map(message_text).collect();

    // Core (system / identity / safety) block survives untouched.
    assert!(
        texts.iter().any(|t| *t == identity),
        "core identity block must survive intact"
    );

    // The shorter injected block survives intact.
    assert!(
        texts.iter().any(|t| *t == short),
        "short injected block must survive intact"
    );

    // The long-tail injected block arrives as head + marker + tail + guide.
    let long_msg = texts
        .iter()
        .find(|t| {
            t.lines()
                .next()
                .is_some_and(|line| !line.is_empty() && line.chars().all(|c| c == 'L'))
        })
        .expect("long injected block present");
    let lines: Vec<&str> = long_msg.lines().collect();
    assert_eq!(lines.len(), 4, "头 / 标记行 / 尾 / 取回指引行: {lines:?}");
    assert!(lines[0].chars().all(|c| c == 'L'), "头预览: {:?}", lines[0]);
    assert!(lines[2].chars().all(|c| c == 'L'), "尾预览: {:?}", lines[2]);
    assert!(
        lines[1].contains("已省略"),
        "中间以标记行替代: {:?}",
        lines[1]
    );
    assert!(lines[3].contains("完整内容已存于"), "附取回指引行");
    assert_eq!(
        lines[0].chars().count() + lines[2].chars().count(),
        269,
        "预览宽度 = 贪心切点宽度 (核心 31 + 短块 100 + 269 = 400)"
    );

    // The guide line carries a real path and the file holds the full original.
    let path_text = lines[3]
        .strip_prefix("完整内容已存于 ")
        .and_then(|rest| rest.split('，').next())
        .expect("指引行含路径");
    let spilled = std::path::Path::new(path_text);
    assert!(spilled.is_file(), "指引行路径必须真实存在: {path_text}");
    assert_eq!(
        std::fs::read_to_string(spilled).unwrap(),
        long,
        "落盘文件保存完整原文"
    );

    // The preview width respects the budget; the marker and guide lines are
    // bounded overhead on top of it.
    let injected_total: usize = texts
        .iter()
        .filter(|t| **t == identity || t.starts_with('L') || t.starts_with('S'))
        .map(|t| t.chars().count())
        .sum();
    assert!(
        injected_total <= budget + 300,
        "injected {injected_total} must fit budget {budget} + bounded overhead"
    );
}

/// Without a spill sink the overflow path keeps the full injected content
/// inline: prefer too long over lost information, and a successful call is
/// never turned into an error.
#[tokio::test]
async fn overflow_without_spill_sink_keeps_the_full_injected_content_inline() {
    let provider = FakeProvider::new();
    let identity = "system identity safety preamble".to_string();
    let long = "L".repeat(2000);
    let short = "S".repeat(100);

    let module = InjectOverlays {
        manifest: ModuleManifest::new("cognitive.inject", "inject"),
        overlays: vec![long.clone(), short.clone()],
    };

    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
        .with_module(Arc::new(module))
        .with_context_budget_chars(400)
        .build()
        .await
        .unwrap();

    runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_system(identity.clone()))
        .await
        .unwrap();

    let request = provider.first_request();
    let texts: Vec<String> = request.messages.iter().map(message_text).collect();
    assert!(
        texts.iter().any(|t| *t == long),
        "无落盘点时宁长勿丢: 长块完整内联"
    );
    assert!(texts.iter().any(|t| *t == short));
    assert!(texts.iter().any(|t| *t == identity));
}

/// Below budget the provider request is unchanged: the injected blocks arrive
/// byte-for-byte identical to what the modules produced.
#[tokio::test]
async fn provider_request_is_unchanged_below_budget() {
    let provider = FakeProvider::new();
    let identity = "system identity safety preamble".to_string();
    let long = "L".repeat(200);
    let short = "S".repeat(100);

    let module = InjectOverlays {
        manifest: ModuleManifest::new("cognitive.inject", "inject"),
        overlays: vec![long.clone(), short.clone()],
    };

    let runtime = Runtime::builder()
        .with_default_model(MODEL)
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
        .with_module(Arc::new(module))
        .with_context_budget_chars(100_000)
        .build()
        .await
        .unwrap();

    runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_system(identity.clone()))
        .await
        .unwrap();

    let request = provider.first_request();
    let texts: Vec<String> = request.messages.iter().map(message_text).collect();
    assert!(
        texts.iter().any(|t| *t == long),
        "long block must be byte-identical below budget"
    );
    assert!(
        texts.iter().any(|t| *t == short),
        "short block must be byte-identical below budget"
    );
    assert!(texts.iter().any(|t| *t == identity), "identity preserved");
}
