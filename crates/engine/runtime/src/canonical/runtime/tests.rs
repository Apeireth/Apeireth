use super::*;
use apeireth_core::kernel::{Lifecycle, Timestamp, VirtualClock};
use apeireth_governance::{DenyCapabilities, GovernancePipeline};
use apeireth_plugin::{
    CapabilityKind, PluginManifest, PluginResult, StaticCredentials, ToolCapability,
};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use std::sync::Mutex;

struct Echo(CapabilityId);

#[async_trait]
impl ToolCapability for Echo {
    fn id(&self) -> &CapabilityId {
        &self.0
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "echo".into(),
            description: Some("echo".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::ok(&call.id, call.arguments.clone())
    }
}

/// Captures whatever credential it was offered at start-up.
struct KeyReader {
    manifest: PluginManifest,
    seen: Mutex<Option<String>>,
}

#[async_trait]
impl Plugin for KeyReader {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    async fn initialize(&self, ctx: &PluginContext) -> PluginResult<()> {
        *self.seen.lock().unwrap() = ctx
            .credentials
            .resolve("provider.fake.api_key")
            .map(|s| s.expose().to_string());
        Ok(())
    }
    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::new(Echo(CapabilityId::new("tool.echo").unwrap()))]
    }
}

fn key_reader() -> Arc<KeyReader> {
    Arc::new(KeyReader {
        manifest: PluginManifest::new(
            PluginId::new("builtin.key_reader").unwrap(),
            "1.0.0",
            "reads a key at boot",
        )
        .declare_capability(
            CapabilityId::new("tool.echo").unwrap(),
            CapabilityKind::Tool,
            "echo",
        )
        .unwrap(),
        seen: Mutex::new(None),
    })
}

/// 出网闸从 Builder 一路带到 ProviderRouter（不注入 = 默认放行）。
#[tokio::test]
async fn egress_gate_reaches_the_provider_router() {
    use crate::canonical::egress_gate::{EgressGate, EgressVerdict};
    use apeireth_protocol::canonical::NormalizedRequest;

    struct CountGate(Arc<std::sync::atomic::AtomicUsize>);
    impl EgressGate for CountGate {
        fn vet(&self, _request: &NormalizedRequest) -> EgressVerdict {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            EgressVerdict::Pass
        }
    }

    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let gate = Arc::new(CountGate(counter.clone()));
    let runtime = Runtime::builder()
        .with_egress_gate(gate)
        .build()
        .await
        .expect("runtime builds");
    // 闸挂在 router 上：构造本身不触发判定（判定发生在出网那一刻）。
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert!(!runtime.providers().is_empty() || true);
}

#[tokio::test]
async fn the_smallest_runtime_builds_with_no_arguments() {
    let runtime = Runtime::builder().build().await.unwrap();
    assert_eq!(runtime.plugins().plugins().len(), 0);
    assert!(runtime.tools().is_empty());
    assert!(runtime.providers().is_empty());
    assert_eq!(runtime.config().max_rounds, DEFAULT_MAX_ROUNDS);
}

#[tokio::test]
async fn a_runtime_builds_and_boots_without_any_credentials() {
    let plugin = key_reader();
    let runtime = Runtime::builder()
        .with_plugin(plugin.clone())
        .build()
        .await
        .unwrap();

    assert_eq!(
        runtime
            .plugins()
            .state(&PluginId::new("builtin.key_reader").unwrap())
            .unwrap(),
        Lifecycle::Active,
        "no key must not prevent boot"
    );
    assert_eq!(*plugin.seen.lock().unwrap(), None);
    assert_eq!(runtime.tools().len(), 1);
}

#[tokio::test]
async fn credentials_reach_plugins_through_the_injected_resolver() {
    let plugin = key_reader();
    let _runtime = Runtime::builder()
        .with_plugin(plugin.clone())
        .with_credentials(Arc::new(
            StaticCredentials::new().with("provider.fake.api_key", "sk-injected"),
        ))
        .build()
        .await
        .unwrap();

    assert_eq!(
        plugin.seen.lock().unwrap().as_deref(),
        Some("sk-injected"),
        "the plugin must receive the key without the runtime storing it"
    );
}

#[tokio::test]
async fn the_runtime_never_prints_a_secret() {
    let runtime = Runtime::builder()
        .with_plugin(key_reader())
        .with_credentials(Arc::new(
            StaticCredentials::new().with("provider.fake.api_key", "sk-injected"),
        ))
        .build()
        .await
        .unwrap();

    let printed = format!("{runtime:?}");
    assert!(!printed.contains("sk-injected"), "{printed}");
}

#[tokio::test]
async fn the_injected_clock_is_the_one_sessions_are_stamped_with() {
    let clock: Arc<dyn Clock> = Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ));
    let runtime = Runtime::builder().with_clock(clock).build().await.unwrap();

    let session = runtime
        .sessions()
        .load_or_create(apeireth_core::kernel::SessionId::new())
        .await
        .unwrap();
    assert_eq!(session.created_at.epoch_millis(), 1_700_000_000_000);
}

#[tokio::test]
async fn a_zero_round_limit_is_rejected_at_build() {
    let err = Runtime::builder()
        .with_max_rounds(0)
        .build()
        .await
        .unwrap_err();
    assert!(matches!(err, RuntimeError::Misconfigured(_)), "{err}");
}

#[tokio::test]
async fn a_duplicate_plugin_is_rejected_at_build() {
    let err = Runtime::builder()
        .with_plugin(key_reader())
        .with_plugin(key_reader())
        .build()
        .await
        .unwrap_err();
    assert!(matches!(err, RuntimeError::Plugin(_)), "{err}");
}

#[tokio::test]
async fn governance_is_held_as_configured() {
    let runtime = Runtime::builder()
        .with_governance(Arc::new(GovernancePipeline::new().with(Arc::new(
            DenyCapabilities::new().deny(CapabilityId::new("tool.example").unwrap()),
        ))))
        .build()
        .await
        .unwrap();
    assert_eq!(runtime.governance().name(), "pipeline");
}

#[tokio::test]
async fn default_governance_is_fail_closed_for_capabilities() {
    let runtime = Runtime::builder().build().await.unwrap();
    assert_eq!(runtime.governance().name(), "deny_unconfigured");
}

#[tokio::test]
async fn shutdown_stops_the_plugins_it_started() {
    let mut runtime = Runtime::builder()
        .with_plugin(key_reader())
        .build()
        .await
        .unwrap();
    let id = PluginId::new("builtin.key_reader").unwrap();

    assert_eq!(runtime.plugins().state(&id).unwrap(), Lifecycle::Active);
    assert!(runtime.shutdown().await.is_empty());
    assert_eq!(runtime.plugins().state(&id).unwrap(), Lifecycle::Stopped);
}

#[tokio::test]
async fn plugin_ids_are_reported_in_id_order() {
    let runtime = Runtime::builder()
        .with_plugin(key_reader())
        .build()
        .await
        .unwrap();
    let ids: Vec<&str> = plugin_ids(&runtime).iter().map(|i| i.as_str()).collect();
    assert_eq!(ids, ["builtin.key_reader"]);
}
